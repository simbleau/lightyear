use bevy::prelude::*;
use lightyear::prelude::*;

#[derive(Channel)]
pub struct ReliableChannel;

#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum ClientPing {
    Ping,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[allow(clippy::large_enum_variant)]
pub enum ServerPong {
    Pong,
}

pub struct ProtocolPlugin;

impl Plugin for ProtocolPlugin {
    fn build(&self, app: &mut App) {
        // Messages
        app.add_message::<ClientPing>(ChannelDirection::ClientToServer);
        app.add_message::<ServerPong>(ChannelDirection::ServerToClient);

        // Channels
        app.add_channel::<ReliableChannel>(ChannelSettings {
            mode: ChannelMode::OrderedReliable(ReliableSettings::default()),
            ..default()
        });
    }
}
