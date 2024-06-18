use crate::shared::{SHARED_COMPRESSION, SHARED_CONFIG, SHARED_PRIVATE_KEY, SHARED_PROTOCOL_ID};
use bevy::prelude::*;
use bevy_async_task::AsyncTask;
use lightyear::{
    prelude::server::{self, Identity},
    server::config::ServerConfig,
};
use std::{
    net::{Ipv4Addr, SocketAddr},
    path::Path,
};

pub struct NovaLightyearServerPlugins {}

impl NovaLightyearServerPlugins {
    pub fn build() -> server::ServerPlugins {
        let server_addr = SocketAddr::new(Ipv4Addr::LOCALHOST.into(), 3536);
        let certificate = AsyncTask::new(async {
            let src_dir = Path::new(file!()).parent().unwrap();
            let cert_pemfile = src_dir.join("../certificates/cert.pem");
            let private_key_pemfile = src_dir.join("../certificates/key.pem");
            Identity::load_pemfiles(cert_pemfile, private_key_pemfile)
                .await
                .unwrap()
        })
        .blocking_recv();

        let netcode_config = lightyear::server::config::NetcodeConfig::default()
            .with_protocol_id(SHARED_PROTOCOL_ID)
            .with_key(SHARED_PRIVATE_KEY);
        let io_config = server::IoConfig {
            transport: server::ServerTransport::WebTransportServer {
                server_addr,
                certificate,
            },
            conditioner: None,
            compression: SHARED_COMPRESSION,
        };
        let server_net_config = server::NetConfig::Netcode {
            config: netcode_config,
            io: io_config,
        };
        let server_config = ServerConfig {
            shared: SHARED_CONFIG,
            net: vec![server_net_config],
            ..default()
        };
        server::ServerPlugins::new(server_config)
    }
}
