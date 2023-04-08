use channels::ChannelsPlugin;
use components::ComponentsPlugin;
use messages::MessagesPlugin;

use std::time::Duration;

use naia_bevy_shared::{LinkConditionerConfig, Protocol};

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

pub mod systems {
    use bevy::prelude::*;
    use bevy_rapier3d::prelude::*;

    use super::components::{RepPhysics, UpdateWith};

    //sync rapier systems:
    //It should run after rapier systems and before naia’s ‘ReceiveEvents’ set. And in the
    //client the exactly opposite. Naia events, then sync system and then rapier systems.

    // in server after physics systems before naia 'ReceiveEvents' systems
    pub fn sync_from_rapier_to_naia(mut query: Query<(&Transform, &Velocity, &mut RepPhysics)>) {
        for (transform, velocity, mut physics_properties) in query.iter_mut() {
            physics_properties.update_with((transform, velocity));
        }
    }

    // in client after naia 'ReceiveEvents' systems before physics systems
    pub fn sync_from_naia_to_rapier(
        mut query: Query<(&mut Transform, &mut Velocity, &RepPhysics)>,
    ) {
        for (mut transform, mut velocity, physics_properties) in query.iter_mut() {
            transform.update_with(physics_properties);
            velocity.update_with(physics_properties);
        }
    }
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
                );
        }
    }
}

pub mod messages {
    use naia_bevy_shared::{EntityProperty, Message, Property, Protocol, ProtocolPlugin, Serde};

    // Plugin
    pub struct MessagesPlugin;

    impl ProtocolPlugin for MessagesPlugin {
        fn build(&self, protocol: &mut Protocol) {
            protocol
                .add_message::<Auth>()
                .add_message::<EntityAssignment>()
                .add_message::<KeyCommand>();
        }
    }

    #[derive(Clone, PartialEq, Serde)]
    pub struct Vec3 {
        pub x: f32,
        pub y: f32,
        pub z: f32,
    }

    impl From<bevy::prelude::Vec3> for Vec3 {
        fn from(b_vec3: bevy::prelude::Vec3) -> Self {
            Self {
                x: b_vec3.x,
                y: b_vec3.y,
                z: b_vec3.z,
            }
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
        pub username: String,
        pub password: String,
    }

    impl Auth {
        pub fn new(username: &str, password: &str) -> Self {
            Self {
                username: username.to_string(),
                password: password.to_string(),
            }
        }
    }
}

pub mod components {
    use bevy::prelude::{Component, Transform, Vec3};
    use bevy_rapier3d::prelude::*;
    use naia_bevy_shared::{Property, Protocol, ProtocolPlugin, Replicate, Serde};

    // Plugin
    pub struct ComponentsPlugin;

    impl ProtocolPlugin for ComponentsPlugin {
        fn build(&self, protocol: &mut Protocol) {
            protocol
                .add_component::<Position>()
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

    impl UpdateWith<(&Transform, &Velocity)> for RepPhysics {
        fn update_with(&mut self, (&transform, &velocity): (&Transform, &Velocity)) {
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
                );
        }
    }
}
