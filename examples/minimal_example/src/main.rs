mod client_lightyear_plugins;
mod protocol;
mod server_lightyear_plugins;
mod shared;

use bevy::log::Level;
use bevy::{log::LogPlugin, prelude::*};
use clap::Parser;
use client_lightyear_plugins::NovaLightyearClientPlugins;
use lightyear::client::config::ClientConfig;
use lightyear::prelude::client::ClientConnection;
use lightyear::prelude::client::NetClient;
use lightyear::prelude::server::ServerCommands;
use protocol::ProtocolPlugin;
use protocol::ReliableChannel;
use protocol::{ClientPing, ServerPong};
use server_lightyear_plugins::NovaLightyearServerPlugins;
use shared::{client_netconfig, NETWORK_TICK_RATE};

/// CLI options to create an [`App`]
#[derive(Parser, PartialEq, Debug)]
pub enum Cli {
    #[cfg(not(target_family = "wasm"))]
    Server,
    Client,
}

pub fn main() {
    let cli = Cli::parse();
    match cli {
        Cli::Server => server(),
        Cli::Client => client(),
    }
}

pub fn server() {
    let mut app = App::new();

    app.add_plugins(MinimalPlugins)
        .add_plugins(LogPlugin {
            level: Level::INFO,
            ..Default::default()
        })
        .add_plugins(NovaLightyearServerPlugins::build())
        .add_plugins(ProtocolPlugin)
        .insert_resource(Time::<Fixed>::from_duration(NETWORK_TICK_RATE))
        .add_systems(Startup, |mut commands: Commands| {
            commands.start_server();
        })
        .add_systems(Update, handle_requests);

    app.run();
}

pub fn client() {
    let mut app = App::new();

    app.add_plugins(MinimalPlugins)
        .add_plugins(LogPlugin {
            level: Level::INFO,
            ..Default::default()
        })
        .add_plugins(NovaLightyearClientPlugins::build())
        .add_plugins(ProtocolPlugin)
        .insert_resource(Time::<Fixed>::from_duration(NETWORK_TICK_RATE))
        .add_systems(
            Startup,
            |mut client_config: ResMut<ClientConfig>, mut connection: ResMut<ClientConnection>| {
                let server_addr = "127.0.0.1:3536".parse().unwrap();
                client_config.net = client_netconfig(server_addr);
                connection.connect().unwrap();
            },
        )
        .add_systems(Update, (read_responses, write_requests));

    app.run();
}

pub fn handle_requests(
    mut login_requests: EventReader<lightyear::server::events::MessageEvent<ClientPing>>,
    mut manager: ResMut<lightyear::server::connection::ConnectionManager>,
) {
    for ev in login_requests.read() {
        let client_id = ev.context;
        manager
            .send_message::<ReliableChannel, _>(client_id, &ServerPong::Pong)
            .unwrap();
    }
}

pub fn read_responses(
    mut events: EventReader<lightyear::client::events::MessageEvent<ServerPong>>,
) {
    for _ev in events.read() {
        info!("received pong");
    }
}

pub fn write_requests(mut manager: ResMut<lightyear::client::connection::ConnectionManager>) {
    manager
        .send_message::<ReliableChannel, _>(&ClientPing::Ping)
        .unwrap();
}
