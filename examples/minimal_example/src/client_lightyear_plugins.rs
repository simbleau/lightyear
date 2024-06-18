use crate::shared::{client_netconfig, SHARED_CONFIG};
use bevy::prelude::default;
use lightyear::client::config::ClientConfig;
use lightyear::prelude::client;
use std::net::{Ipv4Addr, SocketAddr};

pub struct NovaLightyearClientPlugins;

impl NovaLightyearClientPlugins {
    pub fn build() -> client::ClientPlugins {
        let server_addr = SocketAddr::new(Ipv4Addr::LOCALHOST.into(), 3536);
        let netcode = client_netconfig(server_addr);
        let client_config = ClientConfig {
            shared: SHARED_CONFIG,
            net: netcode,
            ..default()
        };
        client::ClientPlugins::new(client_config)
    }
}
