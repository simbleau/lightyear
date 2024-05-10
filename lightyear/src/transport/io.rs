//! Wrapper around a transport, that can perform additional transformations such as
//! bandwidth monitoring or compression
use async_channel::Receiver;
use std::fmt::{Debug, Formatter};
use std::net::{IpAddr, SocketAddr};

use bevy::app::{App, Plugin};
use bevy::diagnostic::{Diagnostic, DiagnosticPath, Diagnostics, RegisterDiagnostic};
use bevy::prelude::{Deref, DerefMut, Real, Res, Resource, Time};
use crossbeam_channel::Sender;
#[cfg(feature = "metrics")]
use metrics;
use tracing::info;

use crate::transport::local::{LocalChannel, LocalChannelBuilder};
use crate::transport::middleware::conditioner::{
    ConditionedPacketReceiver, LinkConditioner, LinkConditionerConfig, PacketLinkConditioner,
};
use crate::transport::middleware::PacketReceiverWrapper;
use crate::transport::{PacketReceiver, PacketSender, Transport};

use super::error::{Error, Result};
use super::{BoxedCloseFn, BoxedReceiver, BoxedSender, LOCAL_SOCKET};

/// Connected io layer that can send/receive bytes
#[derive(Resource)]
pub struct Io {
    pub(crate) local_addr: SocketAddr,
    pub(crate) sender: BoxedSender,
    pub(crate) receiver: BoxedReceiver,
    pub(crate) close_fn: Option<BoxedCloseFn>,
    pub(crate) state: IoState,
    pub(crate) event_receiver: Option<ClientIoEventReceiver>,
    pub(crate) stats: IoStats,
}

impl Default for Io {
    fn default() -> Self {
        panic!("Io::default() is not implemented. Please provide an io");
    }
}

// TODO: add stats/compression to middleware
#[derive(Default, Debug)]
pub struct IoStats {
    pub bytes_sent: usize,
    pub bytes_received: usize,
    pub packets_sent: usize,
    pub packets_received: usize,
}

impl Io {
    pub fn local_addr(&self) -> SocketAddr {
        self.local_addr
    }

    // TODO: no stats are being computed here!
    pub fn split(&mut self) -> (&mut impl PacketSender, &mut impl PacketReceiver) {
        (&mut self.sender, &mut self.receiver)
    }

    pub fn stats(&self) -> &IoStats {
        &self.stats
    }

    pub fn close(&mut self) -> Result<()> {
        self.state = IoState::Disconnected;
        if let Some(close_fn) = std::mem::take(&mut self.close_fn) {
            close_fn()?;
        }
        Ok(())
    }
}

impl Debug for Io {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Io").finish()
    }
}

impl PacketReceiver for Io {
    fn recv(&mut self) -> Result<Option<(&mut [u8], SocketAddr)>> {
        // todo: bandwidth monitoring
        self.receiver.as_mut().recv().map(|x| {
            if let Some((ref buffer, _)) = x {
                #[cfg(feature = "metrics")]
                {
                    metrics::counter!("transport.packets_received").increment(1);
                    metrics::gauge!("transport.bytes_received").increment(buffer.len() as f64);
                }
                self.stats.bytes_received += buffer.len();
                self.stats.packets_received += 1;
            }
            x
        })
    }
}

impl PacketSender for Io {
    fn send(&mut self, payload: &[u8], address: &SocketAddr) -> Result<()> {
        // todo: bandwidth monitoring
        #[cfg(feature = "metrics")]
        {
            metrics::counter!("transport.packets_sent").increment(1);
            metrics::gauge!("transport.bytes_sent").increment(payload.len() as f64);
        }
        self.stats.bytes_sent += payload.len();
        self.stats.packets_sent += 1;
        self.sender.as_mut().send(payload, address)
    }
}

pub struct IoDiagnosticsPlugin;

impl IoDiagnosticsPlugin {
    /// How many bytes do we receive per second
    pub const BYTES_IN: DiagnosticPath = DiagnosticPath::const_new("KB received per second");
    /// How many bytes do we send per second
    pub const BYTES_OUT: DiagnosticPath = DiagnosticPath::const_new("KB sent per second");

    /// How many bytes do we receive per second
    pub const PACKETS_IN: DiagnosticPath = DiagnosticPath::const_new("packets received per second");
    /// How many bytes do we send per second
    pub const PACKETS_OUT: DiagnosticPath = DiagnosticPath::const_new("packets sent per second");

    /// Max diagnostic history length.
    pub const DIAGNOSTIC_HISTORY_LEN: usize = 60;

    pub(crate) fn update_diagnostics(
        stats: &mut IoStats,
        time: &Res<Time<Real>>,
        diagnostics: &mut Diagnostics,
    ) {
        let delta_seconds = time.delta_seconds_f64();
        if delta_seconds == 0.0 {
            return;
        }
        diagnostics.add_measurement(&Self::BYTES_IN, || {
            (stats.bytes_received as f64 / 1000.0) / delta_seconds
        });
        diagnostics.add_measurement(&Self::BYTES_OUT, || {
            (stats.bytes_sent as f64 / 1000.0) / delta_seconds
        });
        diagnostics.add_measurement(&Self::PACKETS_IN, || {
            stats.packets_received as f64 / delta_seconds
        });
        diagnostics.add_measurement(&Self::PACKETS_OUT, || {
            stats.packets_sent as f64 / delta_seconds
        });
        *stats = IoStats::default()
    }
}

impl Plugin for IoDiagnosticsPlugin {
    fn build(&self, app: &mut App) {
        app.register_diagnostic(
            Diagnostic::new(IoDiagnosticsPlugin::BYTES_IN)
                .with_max_history_length(IoDiagnosticsPlugin::DIAGNOSTIC_HISTORY_LEN),
        );
        app.register_diagnostic(
            Diagnostic::new(IoDiagnosticsPlugin::BYTES_OUT)
                .with_max_history_length(IoDiagnosticsPlugin::DIAGNOSTIC_HISTORY_LEN),
        );
        app.register_diagnostic(
            Diagnostic::new(IoDiagnosticsPlugin::PACKETS_IN)
                .with_max_history_length(IoDiagnosticsPlugin::DIAGNOSTIC_HISTORY_LEN),
        );
        app.register_diagnostic(
            Diagnostic::new(IoDiagnosticsPlugin::PACKETS_OUT)
                .with_max_history_length(IoDiagnosticsPlugin::DIAGNOSTIC_HISTORY_LEN),
        );
    }
}

/// Tracks the state of the Io
#[derive(Debug, PartialEq)]
pub(crate) enum IoState {
    Connecting,
    Connected,
    Disconnected,
}

#[derive(Deref, DerefMut)]
pub(crate) struct ClientIoEventReceiver(Receiver<ClientIoEvent>);

/// Events that will be sent from the io thread to the main thread
/// (so that we can update the netcode state when the io changes)
pub(crate) enum ClientIoEvent {
    Connected,
    Disconnected(Error),
}

#[derive(Deref, DerefMut)]
pub(crate) struct ClientNetworkEventSender(Sender<ClientIoEvent>);

#[derive(Deref, DerefMut)]
pub(crate) struct ServerIoEventReceiver(Receiver<ServerIoEvent>);

/// Events that will be sent from the io thread to the main thread
pub(crate) enum ServerIoEvent {
    ServerConnected,
    ServerDisconnected(Error),
    /// the io thread for a given client got disconnected
    ClientDisconnected(SocketAddr),
}

#[derive(Deref, DerefMut)]
pub(crate) struct ServerNetworkEventSender(async_channel::Sender<ServerNetworkEvent>);

/// Event sent from the main thread to the io thread
/// (usually to close the io thread if the main thread requested the server to stop,
/// or to close a client's io thread if the client requested a disconnection)
pub(crate) enum ServerNetworkEvent {
    /// The server is stopped, we should stop the io thread
    ServerDisconnected,
    /// A client id has sent a disconnection packet, we should stop the io thread
    ClientDisconnected(SocketAddr),
}
