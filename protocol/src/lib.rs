use channels::ChannelsPlugin;
use components::ComponentsPlugin;
use messages::MessagesPlugin;

use std::time::Duration;

// use naia_bevy_shared::{LinkConditionerConfig, Protocol};
use naia_bevy_shared::Protocol;

pub const MAGIC_NUMBER: u16 = 0772;
pub const SERVER_HANDSHAKE_URL: &str = "http://192.168.0.188:14191";
pub const SERVER_AD_URL: &str = "http://192.168.0.188:14192";
// pub const SERVER_HANDSHAKE_URL: &str = "http://127.0.0.1:14191";
// pub const SERVER_AD_URL: &str = "http://127.0.0.1:14192";
// pub const SERVER_URL: &str = "http://192.168.0.188:14191";
// pub const SERVER_URL: &str = "http://192.168.50.156:14191";

// Protocol Build
pub fn protocol() -> Protocol {
    Protocol::builder()
        // Config
        .tick_interval(Duration::from_millis(40))
        // .link_condition(LinkConditionerConfig::poor_condition())
        .enable_client_authoritative_entities()
        // Channels
        .add_plugin(ChannelsPlugin)
        // Messages
        .add_plugin(MessagesPlugin)
        // Components
        .add_plugin(ComponentsPlugin)
        // Build Protocol
        .build()
}

pub mod channels {
    use naia_bevy_shared::{
        Channel, ChannelDirection, ChannelMode, Protocol, ProtocolPlugin, ReliableSettings,
        TickBufferSettings,
    };

    #[derive(Channel)]
    pub struct PlayerCommandChannel;

    #[derive(Channel)]
    pub struct EntityAssignmentChannel;

    #[derive(Channel)]
    pub struct GameStateChannel;

    // Plugin
    pub struct ChannelsPlugin;

    impl ProtocolPlugin for ChannelsPlugin {
        fn build(&self, protocol: &mut Protocol) {
            protocol
                .add_channel::<PlayerCommandChannel>(
                    ChannelDirection::ClientToServer,
                    // ChannelMode::UnorderedReliable(ReliableSettings::default()),
                    ChannelMode::TickBuffered(TickBufferSettings::default()),
                )
                .add_channel::<EntityAssignmentChannel>(
                    ChannelDirection::ServerToClient,
                    ChannelMode::UnorderedReliable(ReliableSettings::default()),
                )
                .add_channel::<GameStateChannel>(
                    ChannelDirection::ServerToClient,
                    ChannelMode::UnorderedReliable(ReliableSettings::default()),
                );
        }
    }
}

pub mod primitives {
    use std::collections::HashMap;

    use bevy::prelude::Vec3 as BevyVec3;
    use naia_bevy_shared::Serde;

    #[derive(Default)]
    pub struct Scores {
        pub personal: HashMap<(String, PlayColor), u32>,
        pub blue_total: u32,
        pub pink_total: u32,
    }

    #[derive(Copy, Clone, Eq, Hash, PartialEq, Serde)]
    pub enum PlayColor {
        Blue,
        Pink,
    }

    #[derive(Clone, PartialEq, Serde)]
    pub struct Vec3 {
        pub x: f32,
        pub y: f32,
        pub z: f32,
    }

    impl From<BevyVec3> for Vec3 {
        fn from(b_vec3: BevyVec3) -> Self {
            Self {
                x: b_vec3.x,
                y: b_vec3.y,
                z: b_vec3.z,
            }
        }
    }
}

pub mod messages {
    use super::primitives::{PlayColor, Vec3};

    use bevy::prelude::Entity;
    use naia_bevy_shared::{EntityProperty, Message, Protocol, ProtocolPlugin, Serde};

    // Plugin
    pub struct MessagesPlugin;

    impl ProtocolPlugin for MessagesPlugin {
        fn build(&self, protocol: &mut Protocol) {
            protocol
                .add_message::<Auth>()
                .add_message::<EntityAssignment>()
                .add_message::<PlayerEvent>()
                .add_message::<TotalScoreState>()
                .add_message::<KeyCommand>();
        }
    }

    #[derive(Message)]
    pub struct KeyCommand {
        pub entity: EntityProperty,
        pub reset: bool,
        pub shoot: Option<(Vec3, Vec3)>,
    }

    impl KeyCommand {
        pub fn new(reset: bool, shoot: Option<(Vec3, Vec3)>) -> Self {
            Self {
                entity: EntityProperty::new_empty(),
                reset,
                shoot,
            }
        }
    }

    #[derive(Message)]
    pub struct EntityAssignment {
        pub entity: EntityProperty,
        pub assign: bool,
    }

    impl EntityAssignment {
        pub fn new(assign: bool) -> Self {
            Self {
                assign,
                entity: EntityProperty::new_empty(),
            }
        }
    }

    #[derive(Message)]
    pub struct Auth {
        pub magic_number: u16,
        pub player_name: String,
        pub player_color: PlayColor,
    }

    impl Auth {
        pub fn new(username: &str, player_color: PlayColor) -> Self {
            Self {
                magic_number: super::MAGIC_NUMBER,
                player_name: username.to_string(),
                player_color,
            }
        }
    }

    impl From<(String, String)> for Auth {
        fn from((player_name, player_color): (String, String)) -> Self {
            let player_color = match player_color.to_lowercase().as_ref() {
                "blue" => PlayColor::Blue,
                "pink" => PlayColor::Pink,
                _ => PlayColor::Blue,
            };
            Self {
                magic_number: super::MAGIC_NUMBER,
                player_name,
                player_color,
            }
        }
    }

    #[derive(Serde, Clone, PartialEq)]
    pub enum EventKind {
        PinkScored,
        BlueScored,
        ScoreSnapshot(i32),
        Kicked,
        DeniedGoalie,
        DeniedFrame,
    }

    #[derive(Message)]
    pub struct PlayerEvent {
        pub entity: EntityProperty,
        pub kind: EventKind,
    }

    impl PlayerEvent {
        pub fn pink_scored() -> Self {
            Self {
                entity: EntityProperty::new_empty(),
                kind: EventKind::PinkScored,
            }
        }

        pub fn blue_scored() -> Self {
            Self {
                entity: EntityProperty::new_empty(),
                kind: EventKind::BlueScored,
            }
        }

        pub fn kicked() -> Self {
            Self {
                entity: EntityProperty::new_empty(),
                kind: EventKind::Kicked,
            }
        }

        pub fn new_denied_goalie() -> Self {
            Self {
                entity: EntityProperty::new_empty(),
                kind: EventKind::DeniedGoalie,
            }
        }

        pub fn new_denied_frame() -> Self {
            Self {
                entity: EntityProperty::new_empty(),
                kind: EventKind::DeniedFrame,
            }
        }
    }

    #[derive(Message)]
    pub struct TotalScoreState {
        pub blue: u32,
        pub pink: u32,
    }
}

pub mod components {
    use super::primitives::PlayColor;

    use bevy::prelude::{Component, Transform, Vec3};
    use bevy_rapier3d::prelude::*;
    use naia_bevy_shared::{Property, Protocol, ProtocolPlugin, Replicate, Serde};

    // Plugin
    pub struct ComponentsPlugin;

    impl ProtocolPlugin for ComponentsPlugin {
        fn build(&self, protocol: &mut Protocol) {
            protocol
                .add_component::<RepPhysics>()
                .add_component::<Player>()
                .add_component::<EntityKind>();
        }
    }

    pub trait UpdateWith<T> {
        fn update_with(&mut self, with: T);
    }

    /// Protocol component for Naia that stores the rapier data
    #[derive(Component, Replicate)]
    pub struct RepPhysics {
        // translation
        pub translation_x: Property<f32>,
        pub translation_y: Property<f32>,
        pub translation_z: Property<f32>,
        // rotation
        pub rotation_x: Property<f32>,
        pub rotation_y: Property<f32>,
        pub rotation_z: Property<f32>,
        pub rotation_w: Property<f32>,
        // // scale
        // pub scale_x: Property<f32>,
        // pub scale_y: Property<f32>,
        // pub scale_z: Property<f32>,
        // linvel
        pub linvel_x: Property<f32>,
        pub linvel_y: Property<f32>,
        pub linvel_z: Property<f32>,
        // angvel
        pub angvel_x: Property<f32>,
        pub angvel_y: Property<f32>,
        pub angvel_z: Property<f32>,
    }

    impl RepPhysics {
        pub fn new_with(transform: &Transform, velocity: &Velocity) -> Self {
            Self::new_complete(
                transform.translation.x,
                transform.translation.y,
                transform.translation.z,
                transform.rotation.x,
                transform.rotation.y,
                transform.rotation.z,
                transform.rotation.w,
                velocity.linvel.x,
                velocity.linvel.y,
                velocity.linvel.z,
                velocity.angvel.x,
                velocity.angvel.y,
                velocity.angvel.z,
            )
        }
    }

    impl UpdateWith<(&Transform, &Velocity)> for RepPhysics {
        fn update_with(&mut self, (&transform, &velocity): (&Transform, &Velocity)) {
            // if *self.translation_x == transform.translation.x {
            //     *self.translation_x = transform.translation.x;
            // }
            // if *self.translation_y == transform.translation.y {
            //     *self.translation_y = transform.translation.y;
            // }
            // if *self.translation_z == transform.translation.z {
            //     *self.translation_z = transform.translation.z;
            // }
            //
            // if *self.rotation_x == transform.rotation.x {
            //     *self.rotation_x = transform.rotation.x;
            // }
            // if *self.rotation_y == transform.rotation.y {
            //     *self.rotation_y = transform.rotation.y;
            // }
            // if *self.rotation_z == transform.rotation.z {
            //     *self.rotation_z = transform.rotation.z;
            // }
            // if *self.rotation_w == transform.rotation.w {
            //     *self.rotation_w = transform.rotation.w;
            // }
            *self.translation_x = transform.translation.x;
            *self.translation_y = transform.translation.y;
            *self.translation_z = transform.translation.z;

            *self.rotation_x = transform.rotation.x;
            *self.rotation_y = transform.rotation.y;
            *self.rotation_z = transform.rotation.z;
            *self.rotation_w = transform.rotation.w;

            // *self.scale_x = transform.scale.x;
            // *self.scale_y = transform.scale.y;
            // *self.scale_z = transform.scale.z;

            *self.linvel_x = velocity.linvel.x;
            *self.linvel_y = velocity.linvel.y;
            *self.linvel_z = velocity.linvel.z;
            *self.angvel_x = velocity.angvel.x;
            *self.angvel_y = velocity.angvel.y;
            *self.angvel_z = velocity.angvel.z;
        }
    }

    impl UpdateWith<&RepPhysics> for Transform {
        fn update_with(&mut self, physics_properties: &RepPhysics) {
            self.translation.x = *physics_properties.translation_x;
            self.translation.y = *physics_properties.translation_y;
            self.translation.z = *physics_properties.translation_z;

            self.rotation.x = *physics_properties.rotation_x;
            self.rotation.y = *physics_properties.rotation_y;
            self.rotation.z = *physics_properties.rotation_z;
            self.rotation.w = *physics_properties.rotation_w;

            // self.scale.x = *physics_properties.scale_x;
            // self.scale.y = *physics_properties.scale_y;
            // self.scale.z = *physics_properties.scale_z;
        }
    }

    impl UpdateWith<&RepPhysics> for Velocity {
        fn update_with(&mut self, physics_properties: &RepPhysics) {
            self.linvel.x = *physics_properties.linvel_x;
            self.linvel.y = *physics_properties.linvel_y;
            self.linvel.z = *physics_properties.linvel_z;

            self.angvel.x = *physics_properties.angvel_x;
            self.angvel.y = *physics_properties.angvel_y;
            self.angvel.z = *physics_properties.angvel_z;
        }
    }

    #[derive(Component, Replicate)]
    pub struct Position {
        pub x: Property<f32>,
        pub y: Property<f32>,
        pub z: Property<f32>,
    }

    impl Position {
        pub fn new(x: f32, y: f32, z: f32) -> Self {
            Self::new_complete(x, y, z)
        }
    }

    impl From<Vec3> for Position {
        fn from(v: Vec3) -> Self {
            Self::new_complete(v.x, v.y, v.z)
        }
    }

    #[derive(Component, Replicate)]
    pub struct Player {
        pub name: Property<String>,
        pub color: Property<PlayColor>,
    }

    impl Player {
        pub fn new(name: String, color: PlayColor) -> Self {
            Self::new_complete(name, color)
        }
    }

    #[derive(Component, Replicate)]
    pub struct EntityKind {
        pub value: Property<EntityKindValue>,
    }

    impl EntityKind {
        pub fn goalie() -> Self {
            Self::new_complete(EntityKindValue::Goalie)
        }

        pub fn ball() -> Self {
            Self::new_complete(EntityKindValue::Ball)
        }
    }

    #[derive(Serde, PartialEq, Clone, Debug)]
    pub enum EntityKindValue {
        Goalie,
        Ball,
    }
}

mod channel {
    use naia_bevy_shared::{
        Channel, ChannelDirection, ChannelMode, Protocol, ProtocolPlugin, ReliableSettings,
        TickBufferSettings,
    };

    #[derive(Channel)]
    pub struct PlayerCommandChannel;

    #[derive(Channel)]
    pub struct EntityAssignmentChannel;

    #[derive(Channel)]
    pub struct GameStateChannel;

    // Plugin
    pub struct ChannelsPlugin;

    impl ProtocolPlugin for ChannelsPlugin {
        fn build(&self, protocol: &mut Protocol) {
            protocol
                .add_channel::<PlayerCommandChannel>(
                    ChannelDirection::ClientToServer,
                    ChannelMode::TickBuffered(TickBufferSettings::default()),
                )
                .add_channel::<EntityAssignmentChannel>(
                    ChannelDirection::ServerToClient,
                    ChannelMode::UnorderedReliable(ReliableSettings::default()),
                )
                .add_channel::<GameStateChannel>(
                    ChannelDirection::ServerToClient,
                    ChannelMode::UnorderedReliable(ReliableSettings::default()),
                );
        }
    }
}
