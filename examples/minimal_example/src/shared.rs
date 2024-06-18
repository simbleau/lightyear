use bevy::utils::default;
use lightyear::{
    prelude::CompressionConfig,
    shared::{
        config::{Mode, SharedConfig},
        tick_manager::TickConfig,
    },
};
use std::time::Duration;

/// The standard negotiation for how often ticks occur.
///
/// Our networking layer, [`lightyear`] uses a [`Tick`](lightyear::Tick) to handle synchronization between the client and server.
/// The Tick is basically the fixed-timestep unit of simulation, it gets incremented by 1 every time the [`FixedUpdate`](bevy::app::FixedUpdate) schedule runs.
pub const NETWORK_TICK_RATE: std::time::Duration = std::time::Duration::from_millis(1000 / 64);

pub const SHARED_CONFIG: SharedConfig = SharedConfig {
    // How often the client will send packets to the server (by default it is every frame).
    // Currently, the client only works if it sends packets every frame, for proper input handling.
    client_send_interval: Duration::ZERO,
    // How often the server will send packets to clients? You can reduce this to save bandwidth.
    server_send_interval: Duration::from_millis(100),
    // The tick rate that will be used for the FixedUpdate schedule
    tick: TickConfig {
        tick_duration: NETWORK_TICK_RATE,
    },
    // Either `Separate` mode (distinct client and server apps) or `HostServer` mode (the server also acts as a client).
    mode: Mode::Separate,
};

/// An id to identify the protocol version
pub const SHARED_PROTOCOL_ID: u64 = 0;

/// a 32-byte array to authenticate via the Netcode.io protocol
pub const SHARED_PRIVATE_KEY: [u8; 32] = [0; 32];

/// Compression to use by all parties
pub const SHARED_COMPRESSION: CompressionConfig = CompressionConfig::None;

pub fn client_netconfig(
    server_addr: std::net::SocketAddr,
) -> lightyear::prelude::client::NetConfig {
    let client_addr = std::net::SocketAddr::new(std::net::Ipv4Addr::UNSPECIFIED.into(), 0);
    #[cfg(target_family = "wasm")]
    // On wasm, we need to provide a hash of the certificate to the browser
    let certificate_digest = include_str!("../../certificates/cert.sha256").to_string();

    let io_config = lightyear::prelude::client::IoConfig {
        transport: lightyear::prelude::client::ClientTransport::WebTransportClient {
            client_addr,
            server_addr,
            #[cfg(target_family = "wasm")]
            certificate_digest,
        },
        conditioner: None,
        compression: SHARED_COMPRESSION,
    };
    lightyear::prelude::client::NetConfig::Netcode {
        auth: lightyear::prelude::client::Authentication::Manual {
            server_addr,
            client_id: bevy::utils::uuid::Uuid::new_v4().as_u128() as u64,
            private_key: SHARED_PRIVATE_KEY,
            protocol_id: SHARED_PROTOCOL_ID,
        },
        config: lightyear::prelude::client::NetcodeConfig {
            client_timeout_secs: 15,
            token_expire_secs: 300,
            ..default()
        },
        io: io_config,
    }
}
