/// # TODO
/// - goalie jump or bobble or shield (probably shield?)
/// - asset loading
/// - render unowned balls names (nice to have?)
///
/// #Later Learning
/// - confirm soundness of the system orders (client and server side)
/// - Determinism:
///     - https://github.com/dimforge/bevy_rapier/issues/79
///     - https://github.com/Looooong/doce/blob/67a32acbd8cfbf31c88b253bf5991a17da5d06bc/src/main.rs
/// - Lag compensation on server (maybe, this is hard)
///     - store X ticks of server state
///     - when a kick arrives, simulate from the kick tick that the client THOUGHT he saw
///     - if the ball hits the goal or completely misses, do nothing and let it score
///     - if the ball is "bounced" by the goalie, stitch this physics into the current physics
///     engine.  (this is hard/unknown for bevy_rapier.  Not sure how to do this but the ball
///     basically needs to fly as if it was bounced off at that past point, into the future
///     somehow.  The current physics will need to keep moving forward)
use crate::protocol::messages::{Auth, KeyCommand};

use std::f32::consts::*;

// use bevy::input::InputPlugin;
// use bevy::log::LogPlugin;
use bevy::prelude::*;
// use bevy_asset_loader::prelude::*;
use bevy_rapier3d::prelude::*;

use bevy_debug_text_overlay::{screen_print, OverlayPlugin};
// use bevy_inspector_egui::prelude::*;
use bevy_inspector_egui::quick::WorldInspectorPlugin;

use naia_bevy_client::{
    transport::webrtc, Client, ClientConfig, CommandHistory, Plugin as NaiaClientPlugin,
    ReceiveEvents,
};

// #[derive(Debug, Clone, Copy, Eq, PartialEq, Hash, Default, States)]
// pub enum AppState {
//     #[default]
//     Loading,
//     Staging,
//     Connect,
//     InGame,
// }

#[derive(SystemSet, Debug, Hash, PartialEq, Eq, Clone)]
struct MainLoop;

#[derive(SystemSet, Debug, Hash, PartialEq, Eq, Clone)]
struct Tick;

pub fn debug_overlay(time: Res<Time>) {
    let current_time = time.elapsed_seconds_f64();
    let at_interval = |t: f64| current_time % t < time.delta_seconds_f64();
    if at_interval(0.1) {
        let last_fps = 1.0 / time.delta_seconds();
        screen_print!(col: Color::CYAN, "fps: {last_fps:.0}");
    }
}

pub fn init(
    mut commands: Commands,
    mut client: Client,
    mut meshes: ResMut<Assets<Mesh>>,
    mut images: ResMut<Assets<Image>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    // mut materials: ResMut<Assets<ColorMaterial>>,
) {
    info!("Naia Bevy Client Demo started");

    client.auth(Auth::new("charlie", "12345"));
    let socket = webrtc::Socket::new("http://127.0.0.1:14191", client.socket_config());
    client.connect(socket);

    let ball_texture_handle = images.add(crate::uv_debug_texture());
    // Setup Global Resource
    let mut global = Global::default();
    global.ball_texture = ball_texture_handle.clone();
    global.debug_material = materials.add(StandardMaterial {
        base_color_texture: Some(ball_texture_handle),
        alpha_mode: AlphaMode::Blend,
        ..default()
    });
    let ground_material = materials.add(StandardMaterial {
        base_color: Color::SEA_GREEN,
        perceptual_roughness: 1.0,
        ..default()
    });
    let goal_material = materials.add(StandardMaterial {
        base_color: Color::WHITE,
        perceptual_roughness: 1.0,
        ..default()
    });

    commands.spawn((Camera3dBundle {
        transform: crate::KICK_CAM.looking_at(crate::KICK_CAM_LOOK, Vec3::Y),
        ..Default::default()
    },));

    commands.spawn(DirectionalLightBundle {
        directional_light: DirectionalLight {
            shadows_enabled: true,
            ..default()
        },
        transform: Transform {
            translation: Vec3::new(0.0, 2.0, 0.0),
            rotation: Quat::from_rotation_x(-PI / 4.),
            ..default()
        },
        ..default()
    });

    global.ground_entity = Some(
        commands
            .spawn((
                Name::new("Ground"),
                PbrBundle {
                    mesh: meshes.add(shape::Plane::from_size(crate::GROUND_SIZE * 2.0).into()),
                    material: ground_material,
                    transform: Transform::from_xyz(0.0, crate::GROUND_HEIGHT, 0.0),
                    ..default()
                },
                Collider::cuboid(crate::GROUND_SIZE, 0.0, crate::GROUND_SIZE),
                RigidBody::KinematicPositionBased,
                Friction::new(100.0),
            ))
            .id(),
    );

    let x = 0.0;
    let y = crate::GROUND_HEIGHT;
    let z = 32.0;
    let rad = 0.2;
    commands
        .spawn((
            Name::new("Goal"),
            // TransformBundle::from(Transform::from_xyz(x, y, z)),
            PbrBundle {
                transform: Transform::from_xyz(x, y, z),
                ..default()
            },
        ))
        .with_children(|p| {
            p.spawn((
                Name::new("FrameTop"),
                PbrBundle {
                    mesh: meshes
                        .add(shape::Box::new(rad * 10.0 * 2.0, rad * 0.5 * 2.0, rad * 2.0).into()),
                    material: goal_material.clone(),
                    transform: Transform::from_xyz(0.0, rad * 10.0, 0.0),
                    ..default()
                },
            ));
            p.spawn((
                Name::new("FrameLeft"),
                PbrBundle {
                    mesh: meshes
                        .add(shape::Box::new(rad * 0.5 * 2.0, rad * 5.0 * 2.0, rad * 2.0).into()),
                    material: goal_material.clone(),
                    transform: Transform::from_xyz(rad * 10.0, rad * 5.0, 0.0),
                    ..default()
                },
            ));
            p.spawn((
                Name::new("FrameRight"),
                PbrBundle {
                    mesh: meshes
                        .add(shape::Box::new(rad * 0.5 * 2.0, rad * 5.0 * 2.0, rad * 2.0).into()),
                    material: goal_material.clone(),
                    transform: Transform::from_xyz(-rad * 10.0, rad * 5.0, 0.0),
                    ..default()
                },
            ));
        });

    // Insert Global Resource
    commands.insert_resource(global);
}

pub struct OwnedEntity {
    pub confirmed: Entity,
    pub predicted: Entity,
}

impl OwnedEntity {
    pub fn new(confirmed_entity: Entity, predicted_entity: Entity) -> Self {
        OwnedEntity {
            confirmed: confirmed_entity,
            predicted: predicted_entity,
        }
    }
}

#[derive(Resource, Default)]
pub struct Global {
    pub camera_is_birdseye: bool,

    // pub owned_entity: Option<OwnedEntity>,
    pub owned_entity: Option<OwnedEntity>,
    pub ground_entity: Option<Entity>,
    pub queued_command: Option<KeyCommand>,
    pub command_history: CommandHistory<KeyCommand>,

    pub debug_material: Handle<StandardMaterial>,
    pub ground_material: Handle<StandardMaterial>,
    pub goal_material: Handle<StandardMaterial>,
    pub ball_texture: Handle<Image>,
}

mod components {
    use bevy::prelude::*;

    #[derive(Component)]
    pub struct Predicted;

    #[derive(Component)]
    pub struct Confirmed;

    #[derive(Component)]
    pub struct LocalCursor;

    #[derive(Default, Component)]
    pub struct InterpPos {
        interp: f32,

        pub interp_x: f32,
        pub interp_y: f32,
        pub interp_z: f32,

        last_x: f32,
        last_y: f32,
        last_z: f32,

        pub next_x: f32,
        pub next_y: f32,
        pub next_z: f32,
    }

    impl InterpPos {
        pub fn new(x: f32, y: f32, z: f32) -> Self {
            Self {
                interp: 0.0,

                interp_x: x,
                interp_y: y,
                interp_z: z,

                last_x: x,
                last_y: y,
                last_z: z,

                next_x: x,
                next_y: y,
                next_z: z,
            }
        }

        pub(crate) fn next(&mut self, next_x: f32, next_y: f32, next_z: f32) {
            self.interp = 0.0;
            self.last_x = self.next_x;
            self.last_y = self.next_y;
            self.last_z = self.next_z;

            self.interp_x = self.next_x;
            self.interp_y = self.next_y;
            self.interp_z = self.next_z;

            self.next_x = next_x;
            self.next_y = next_y;
            self.next_z = next_z;
        }

        pub(crate) fn interpolate(&mut self, interpolation: f32) {
            if self.interp >= 1.0 || interpolation == 0.0 {
                return;
            }
            if self.interp < interpolation {
                self.interp = interpolation;
                self.interp_x = self.last_x + (self.next_x - self.last_x) * self.interp;
                self.interp_y = self.last_y + (self.next_y - self.last_y) * self.interp;
                self.interp_z = self.last_z + (self.next_z - self.last_z) * self.interp;
            }
        }
    }

    #[derive(Default, Component)]
    pub struct InterpRot {
        interp: f32,

        pub interp_x: f32,
        pub interp_y: f32,
        pub interp_z: f32,
        pub interp_w: f32,

        last_x: f32,
        last_y: f32,
        last_z: f32,
        last_w: f32,

        pub next_x: f32,
        pub next_y: f32,
        pub next_z: f32,
        pub next_w: f32,
    }

    impl InterpRot {
        pub fn new(x: f32, y: f32, z: f32, w: f32) -> Self {
            Self {
                interp: 0.0,

                interp_x: x,
                interp_y: y,
                interp_z: z,
                interp_w: w,

                last_x: x,
                last_y: y,
                last_z: z,
                last_w: w,

                next_x: x,
                next_y: y,
                next_z: z,
                next_w: w,
            }
        }

        pub(crate) fn next(&mut self, next_x: f32, next_y: f32, next_z: f32, next_w: f32) {
            self.interp = 0.0;
            self.last_x = self.next_x;
            self.last_y = self.next_y;
            self.last_z = self.next_z;
            self.last_w = self.next_w;

            self.interp_x = self.next_x;
            self.interp_y = self.next_y;
            self.interp_z = self.next_z;
            self.interp_w = self.next_w;

            self.next_x = next_x;
            self.next_y = next_y;
            self.next_z = next_z;
            self.next_w = next_w;
        }

        pub(crate) fn interpolate(&mut self, interpolation: f32) {
            if self.interp >= 1.0 || interpolation == 0.0 {
                return;
            }
            if self.interp < interpolation {
                self.interp = interpolation;
                self.interp_x = self.last_x + (self.next_x - self.last_x) * self.interp;
                self.interp_y = self.last_y + (self.next_y - self.last_y) * self.interp;
                self.interp_z = self.last_z + (self.next_z - self.last_z) * self.interp;
                self.interp_w = self.last_w + (self.next_w - self.last_w) * self.interp;
            }
        }
    }
}

mod events {
    use bevy::{pbr::NotShadowReceiver, prelude::*};
    use bevy_rapier3d::prelude::*;

    use naia_bevy_client::{
        events::{
            ClientTickEvent, ConnectEvent, DespawnEntityEvent, DisconnectEvent,
            InsertComponentEvents, MessageEvents, RejectEvent, RemoveComponentEvents,
            SpawnEntityEvent, UpdateComponentEvents,
        },
        sequence_greater_than, Client, CommandsExt, Random, Replicate, Tick,
    };

    use crate::protocol::{
        channels::{EntityAssignmentChannel, PlayerCommandChannel},
        components::{EntityKind, EntityKindValue, RepPhysics, UpdateWith},
        messages::{EntityAssignment, KeyCommand},
    };

    use super::components::{Confirmed, InterpPos, InterpRot, LocalCursor, Predicted};
    use super::{Global, OwnedEntity};

    pub fn connect_events(
        // mut commands: Commands,
        mut client: Client,
        // mut global: ResMut<Global>,
        mut event_reader: EventReader<ConnectEvent>,
    ) {
        for _ in event_reader.iter() {
            let Ok(server_address) = client.server_address() else {
            panic!("Shouldn't happen");
        };
            info!("Client connected to: {}", server_address);

            // // Create entity for Client-authoritative Cursor
            //
            // // Position component
            // let position = {
            //     let x = 16 * ((Random::gen_range_u32(0, 40) as i16) - 20);
            //     let y = 16 * ((Random::gen_range_u32(0, 30) as i16) - 15);
            //     Position::new(x, y)
            // };
            //
            // // Spawn Cursor Entity
            // let entity = commands
            //     // Spawn new Square Entity
            //     .spawn_empty()
            //     // MUST call this to begin replication
            //     .enable_replication(&mut client)
            //     // Insert Position component
            //     .insert(position)
            //     // Insert Cursor marker component
            //     .insert(LocalCursor)
            //     // return Entity id
            //     .id();
            //
            // // Insert SpriteBundle locally only
            // commands.entity(entity).insert(MaterialMesh2dBundle {
            //     mesh: global.circle.clone().into(),
            //     material: global.white.clone(),
            //     transform: Transform::from_xyz(0.0, 0.0, 1.0),
            //     ..Default::default()
            // });
            //
            // global.cursor_entity = Some(entity);
        }
    }

    pub fn reject_events(mut event_reader: EventReader<RejectEvent>) {
        for _ in event_reader.iter() {
            info!("Client rejected from connecting to Server");
        }
    }

    pub fn disconnect_events(mut event_reader: EventReader<DisconnectEvent>) {
        for _ in event_reader.iter() {
            info!("Client disconnected from Server");
        }
    }

    pub fn message_events(
        client: Client,
        mut global: ResMut<Global>,
        mut meshes: ResMut<Assets<Mesh>>,
        mut materials: ResMut<Assets<StandardMaterial>>,

        mut commands: Commands,
        mut event_reader: EventReader<MessageEvents>,

        ball_query: Query<(&RepPhysics, &Handle<StandardMaterial>)>,
    ) {
        for events in event_reader.iter() {
            for message in events.read::<EntityAssignmentChannel, EntityAssignment>() {
                let assign = message.assign;
                let entity = message.entity.get(&client).unwrap();
                if assign {
                    info!("entityassignment ownership");

                    // Here we create a local copy of the Player entity, to use for client-side prediction
                    if let Ok((rep_physics, mat_handle)) = ball_query.get(entity) {
                        //change the owned_confirmed entity (server side) to a transparent red
                        let confirmed_ball_mat = materials.get_mut(mat_handle).unwrap();
                        confirmed_ball_mat.base_color.set_a(1.0);

                        // // take and give this texture to the prediction ball.  All balls
                        // // are spawned with the default texture
                        // let confirmed_texture = confirmed_ball_mat.base_color_texture.take();
                        // // set the confirmed ball to red with transparency
                        // confirmed_ball_mat.base_color = Color::rgba(0.7, 0.2, 0.1, 0.4);
                        // confirmed_ball_mat.unlit = true;

                        //add physics ball for clicking, eventually use it for prediction?
                        let mut prediction_transform = Transform::default();
                        prediction_transform.update_with(rep_physics);
                        let prediction_entity = commands
                            .entity(entity)
                            .duplicate()
                            .insert((
                                PbrBundle {
                                    mesh: meshes.add(
                                        shape::UVSphere {
                                            radius: crate::BALL_RADIUS,
                                            sectors: 36,
                                            stacks: 18,
                                        }
                                        .into(),
                                    ),
                                    material: materials.add(Color::rgba(0.7, 0.2, 0.1, 0.0).into()),
                                    // material: materials.add(StandardMaterial {
                                    //     base_color: Color::rgb(1.0, 1.0, 1.0),
                                    //     base_color_texture: confirmed_texture,
                                    //     alpha_mode: AlphaMode::Blend,
                                    //     ..default()
                                    // }),
                                    transform: prediction_transform,
                                    ..default()
                                },
                                RigidBody::Dynamic,
                                Collider::ball(crate::BALL_RADIUS),
                                ColliderMassProperties::Mass(crate::BALL_MASS),
                                Velocity::zero(),
                                Friction::new(5.0),
                                ExternalForce::default(),
                                ExternalImpulse::default(),
                                GravityScale::default(),
                                Damping {
                                    linear_damping: 1.0,
                                    angular_damping: 2.0,
                                },
                                Restitution {
                                    coefficient: 1.0,
                                    combine_rule: CoefficientCombineRule::Average,
                                },
                                Sleeping::default(),
                                Predicted,
                            ))
                            .id();
                        global.owned_entity = Some(OwnedEntity::new(entity, prediction_entity));
                    }
                } else {
                    info!("entityassignment disown");
                    let mut disowned: bool = false;
                    if let Some(owned_entity) = &global.owned_entity {
                        if owned_entity.confirmed == entity {
                            commands.entity(owned_entity.predicted).despawn();
                            disowned = true;
                        }
                    }
                    if disowned {
                        info!("removed ownership of entity");
                        global.owned_entity = None;
                    }
                }
            }
        }
    }

    pub fn spawn_entity_events(mut event_reader: EventReader<SpawnEntityEvent>) {
        for SpawnEntityEvent(_entity) in event_reader.iter() {
            info!("spawned entity");
        }
    }

    pub fn despawn_entity_events(mut event_reader: EventReader<DespawnEntityEvent>) {
        for DespawnEntityEvent(_entity) in event_reader.iter() {
            info!("despawned entity");
        }
    }

    pub fn insert_component_events(
        global: Res<Global>,
        mut meshes: ResMut<Assets<Mesh>>,
        mut materials: ResMut<Assets<StandardMaterial>>,

        mut commands: Commands,
        mut event_reader: EventReader<InsertComponentEvents>,

        kind_query: Query<&EntityKind>,
        rep_physics_query: Query<&RepPhysics>,
    ) {
        for events in event_reader.iter() {
            log::info!("insert component events");
            for entity in events.read::<RepPhysics>() {
                let kind = kind_query.get(entity).unwrap();
                let rep_physics = rep_physics_query.get(entity).unwrap();

                log::info!("entity: {:?}", *kind.value);
                match *kind.value {
                    EntityKindValue::Goalie => {
                        commands.entity(entity).insert((
                            Name::new("Goalie"),
                            PbrBundle {
                                mesh: meshes.add(
                                    shape::Capsule {
                                        radius: crate::GOALIE_RADIUS,
                                        rings: 0,
                                        depth: crate::GOALIE_HEIGHT,
                                        latitudes: 16,
                                        longitudes: 32,
                                        uv_profile: shape::CapsuleUvProfile::Aspect,
                                    }
                                    .into(),
                                ),
                                material: global.debug_material.clone(),
                                transform: Transform::from_xyz(
                                    *rep_physics.translation_x,
                                    crate::GOALIE_START.y,
                                    crate::GOALIE_START.z,
                                ),
                                ..default()
                            },
                            InterpPos::new(
                                *rep_physics.translation_x,
                                *rep_physics.translation_y,
                                *rep_physics.translation_z,
                            ),
                            InterpRot::new(
                                *rep_physics.rotation_x,
                                *rep_physics.rotation_y,
                                *rep_physics.rotation_z,
                                *rep_physics.rotation_w,
                            ),
                            Confirmed,
                        ));
                    }
                    EntityKindValue::Ball => {
                        commands.entity(entity).insert((
                            Name::new("Ball"),
                            crate::Ball::default(),
                            PbrBundle {
                                mesh: meshes.add(
                                    shape::UVSphere {
                                        radius: crate::BALL_RADIUS,
                                        sectors: 36,
                                        stacks: 18,
                                    }
                                    .into(),
                                ),
                                material: materials.add(StandardMaterial {
                                    base_color: Color::rgba(1.0, 1.0, 1.0, 0.2),
                                    base_color_texture: Some(global.ball_texture.clone()),
                                    alpha_mode: AlphaMode::Blend,
                                    ..default()
                                }),
                                transform: Transform::from_xyz(
                                    *rep_physics.translation_x,
                                    *rep_physics.translation_y,
                                    *rep_physics.translation_z,
                                ),
                                ..default()
                            },
                            InterpPos::new(
                                *rep_physics.translation_x,
                                *rep_physics.translation_y,
                                *rep_physics.translation_z,
                            ),
                            InterpRot::new(
                                *rep_physics.rotation_x,
                                *rep_physics.rotation_y,
                                *rep_physics.rotation_z,
                                *rep_physics.rotation_w,
                            ),
                            NotShadowReceiver,
                            Confirmed,
                        ));
                    }
                }
            }
        }
    }

    pub fn update_component_events(
        mut global: ResMut<Global>,
        mut event_reader: EventReader<UpdateComponentEvents>,
        // mut position_query: Query<&mut Position>,
    ) {
        // When we receive a new Position update for the Player's Entity,
        // we must ensure the Client-side Prediction also remains in-sync
        // So we roll the Prediction back to the authoritative Server state
        // and then execute all Player Commands since that tick, using the CommandHistory helper struct
        // if let Some(owned_entity) = &global.owned_entity {
        //     let mut latest_tick: Option<Tick> = None;
        //     let server_entity = owned_entity.confirmed;
        //     let client_entity = owned_entity.predicted;
        //
        //     for events in event_reader.iter() {
        //         for (server_tick, updated_entity) in events.read::<Position>() {
        //             // If entity is owned
        //             if updated_entity == server_entity {
        //                 if let Some(last_tick) = &mut latest_tick {
        //                     if sequence_greater_than(server_tick, *last_tick) {
        //                         *last_tick = server_tick;
        //                     }
        //                 } else {
        //                     latest_tick = Some(server_tick);
        //                 }
        //             }
        //         }
        //     }
        //
        //     if let Some(server_tick) = latest_tick {
        //         if let Ok([server_position, mut client_position]) =
        //             position_query.get_many_mut([server_entity, client_entity])
        //         {
        //             // Set to authoritative state
        //             client_position.mirror(&*server_position);
        //
        //             // Replay all stored commands
        //
        //             // TODO: why is it necessary to subtract 1 Tick here?
        //             // it's not like this in the Macroquad demo
        //             let modified_server_tick = server_tick.wrapping_sub(1);
        //
        //             let replay_commands = global.command_history.replays(&modified_server_tick);
        //             for (_command_tick, command) in replay_commands {
        //                 process_command(&command, &mut client_position);
        //             }
        //         }
        //     }
        // }
    }

    pub fn remove_component_events(mut event_reader: EventReader<RemoveComponentEvents>) {
        for events in event_reader.iter() {
            // for (_entity, _component) in events.read::<Position>() {
            //     info!("removed Position component from entity");
            // }
        }
    }

    pub fn tick_events(
        mut client: Client,
        mut global: ResMut<Global>,
        mut tick_reader: EventReader<ClientTickEvent>,
        // mut position_query: Query<&mut Position>,
    ) {
        let Some(predicted_entity) = global
            .owned_entity
            .as_ref()
            .map(|owned_entity| owned_entity.predicted) else {
            // No owned Entity
            return;
        };

        let Some(command) = global.queued_command.take() else {
            return;
        };

        for ClientTickEvent(client_tick) in tick_reader.iter() {
            if !global.command_history.can_insert(client_tick) {
                // History is full
                continue;
            }

            // Record command
            global.command_history.insert(*client_tick, command.clone());

            // Send command
            client.send_tick_buffer_message::<PlayerCommandChannel, KeyCommand>(
                client_tick,
                &command,
            );

            //#TODO client-side prediction command isn't handled yet
            // we want to 'process_ball_commmand' on the predicted entity
        }
    }
}

mod input {
    use super::Global;
    use bevy::{prelude::*, render::camera::RenderTarget, window::PrimaryWindow};
    use bevy_rapier3d::prelude::*;

    use super::components::Confirmed;
    use crate::protocol::messages::KeyCommand;
    use naia_bevy_client::Client;

    pub fn camera(
        time: Res<Time>,
        mut global: ResMut<Global>,
        keyboard_input: Res<Input<KeyCode>>,

        mut camera_query: Query<&mut Transform, With<Camera>>,
    ) {
        let mut camera_transform = camera_query.single_mut();

        if keyboard_input.just_pressed(KeyCode::C) {
            global.camera_is_birdseye = !global.camera_is_birdseye;
            if global.camera_is_birdseye {
                *camera_transform =
                    crate::BIRDS_EYE_CAM.looking_at(crate::BIRDS_EYE_CAM_LOOK, Vec3::Y);
            } else {
                *camera_transform = crate::KICK_CAM.looking_at(crate::KICK_CAM_LOOK, Vec3::Y);
            }
        }

        if keyboard_input.pressed(KeyCode::Left) {
            camera_transform.rotate_around(
                crate::BALL_START,
                Quat::from_euler(EulerRot::XYZ, 0.0, time.delta_seconds(), 0.0),
            );
        } else if keyboard_input.pressed(KeyCode::Right) {
            camera_transform.rotate_around(
                crate::BALL_START,
                Quat::from_euler(EulerRot::XYZ, 0.0, -time.delta_seconds(), 0.0),
            );
        }
    }

    pub fn ball(
        mut global: ResMut<Global>,
        client: Client,
        keyboard_input: Res<Input<KeyCode>>,
        mouse_buttons: ResMut<Input<MouseButton>>,
        ball_query: Query<&Transform, With<Confirmed>>,

        camera_query: Query<(&Camera, &Transform, &GlobalTransform)>,
        window: Query<&Window, With<PrimaryWindow>>,
        rapier_context: Res<RapierContext>,
    ) {
        let Some(owned_entity) = &global.owned_entity else {
            return;
        };

        let Ok(ball_transform) = ball_query.get(owned_entity.confirmed) else {
            return;
        };

        //#HACK only let the ball get kicked if it has already settled down from the spawn
        //point.  When prediction is implemented, We use the local predicted ball and not the
        //confirmed ball because of lag compensation.  IE, we expect the confirmed ball to be
        //already dropped by the time we actually see the updates at the client.  The predicted
        //ball is spawned after the ball entity assignment.  For now use the Confirmed Entity
        let can_shoot = ball_transform.translation.y < 0.01;

        let reset = keyboard_input.pressed(KeyCode::Q);
        let shoot = match (
            can_shoot,
            keyboard_input.pressed(KeyCode::Space),
            mouse_buttons.pressed(MouseButton::Left),
        ) {
            (true, true, _) => Some((
                Vec3::new(0.015694855, -0.011672409, 0.9998087).into(),
                Vec3::new(0.0017264052, 0.0070980787, 42.109978).into(),
            )),
            (true, false, true) => {
                capture_ball_click(camera_query, window, rapier_context, &global.ground_entity)
                    .map(|(ray_normal, ray_point)| (ray_normal.into(), ray_point.into()))
            }
            _ => None,
        };

        if let Some(command) = &mut global.queued_command {
            command.reset = reset;
            command.shoot = shoot;
        } else if let Some(owned_entity) = &global.owned_entity {
            let mut key_command = KeyCommand::new(reset, shoot);
            key_command.entity.set(&client, &owned_entity.confirmed);
            global.queued_command = Some(key_command);
        }
    }

    fn capture_ball_click(
        camera_query: Query<(&Camera, &Transform, &GlobalTransform)>,
        window: Query<&Window, With<PrimaryWindow>>,
        rapier_context: Res<RapierContext>,
        ground_entity: &Option<Entity>,
    ) -> Option<(Vec3, Vec3)> {
        let (camera, _camera_transform, camera_global_transform) = camera_query.single();

        // get the window that the camera is displaying to (or the primary window)
        let window = if let RenderTarget::Window(_id) = camera.target {
            window.single()
        } else {
            window.single()
            // window.single().get_primary().unwrap()
        };

        // check if the cursor is inside the window and get its position
        // then, ask bevy to convert into world coordinates, and truncate to discard Z
        let Some(ray) = window
            .cursor_position()
            .and_then(|cursor| camera.viewport_to_world(camera_global_transform, cursor))
            else {
            return None
        };
        let filter = QueryFilter {
            exclude_rigid_body: ground_entity.clone(),
            ..Default::default()
        };
        rapier_context
            .cast_ray_and_get_normal(ray.origin, ray.direction, 100.0, true, filter)
            .map(|(_e, intersection)| (intersection.normal, intersection.point))
    }
}

pub mod sync {
    use bevy::prelude::*;
    use bevy_rapier3d::prelude::*;

    use crate::protocol::components::{RepPhysics, UpdateWith};
    use naia_bevy_client::Client;

    use super::components::{Confirmed, InterpPos, InterpRot, Predicted};

    pub fn serverside_entities(
        client: Client,
        mut query: Query<
            (&RepPhysics, &mut InterpPos, &mut InterpRot, &mut Transform),
            With<Confirmed>,
        >,
    ) {
        for (rep_physics, mut interp, mut interp_rot, mut transform) in query.iter_mut() {
            if *rep_physics.translation_x != interp.next_x
                || *rep_physics.translation_y != interp.next_y
                || *rep_physics.translation_z != interp.next_z
            {
                interp.next(
                    *rep_physics.translation_x,
                    *rep_physics.translation_y,
                    *rep_physics.translation_z,
                );
            }

            let interp_amount = client.server_interpolation().unwrap();
            interp.interpolate(interp_amount);
            transform.translation.x = interp.interp_x;
            transform.translation.y = interp.interp_y;
            transform.translation.z = interp.interp_z;

            if *rep_physics.rotation_x != interp_rot.next_x
                || *rep_physics.rotation_y != interp_rot.next_y
                || *rep_physics.rotation_z != interp_rot.next_z
                || *rep_physics.rotation_w != interp_rot.next_w
            {
                interp_rot.next(
                    *rep_physics.rotation_x,
                    *rep_physics.rotation_y,
                    *rep_physics.rotation_z,
                    *rep_physics.rotation_w,
                );
            }

            interp_rot.interpolate(interp_amount);
            transform.rotation.x = interp_rot.interp_x;
            transform.rotation.y = interp_rot.interp_y;
            transform.rotation.z = interp_rot.interp_z;
            transform.rotation.w = interp_rot.interp_w;
        }
    }

    //sync rapier systems:
    //It should run after rapier systems and before naia’s ‘ReceiveEvents’ set. And in the
    //client the exactly opposite. Naia events, then sync system and then rapier systems.

    // in client after naia 'ReceiveEvents' systems before physics systems
    pub fn sync_from_naia_to_rapier(
        mut query: Query<(&mut Transform, &mut Velocity, &RepPhysics)>,
    ) {
        for (mut transform, mut velocity, physics_properties) in query.iter_mut() {
            transform.update_with(physics_properties);
            velocity.update_with(physics_properties);
        }
    }

    // pub fn sync_clientside_sprites(
    //     client: Client,
    //     mut query: Query<(&Position, &mut Interp, &mut Transform), With<Predicted>>,
    // ) {
    //     for (position, mut interp, mut transform) in query.iter_mut() {
    //         if *position.x != interp.next_x as i16 || *position.y != interp.next_y as i16 {
    //             interp.next_position(*position.x, *position.y);
    //         }
    //
    //         let interp_amount = client.client_interpolation().unwrap();
    //         interp.interpolate(interp_amount);
    //         transform.translation.x = interp.interp_x;
    //         transform.translation.y = interp.interp_y;
    //     }
    // }
    //
}

// #[derive(AssetCollection, Resource)]
// pub struct AppAssets {
//     #[asset(path = "images/yoshiegg.png")]
//     pub ball_texture: Handle<Image>,
//     // #[asset(path = "walking.ogg")]
//     // walking: Handle<AudioSource>,
// }

pub fn run() {
    App::default()
        // .add_state::<AppState>()
        // .add_loading_state(
        //     LoadingState::new(AppState::Loading).continue_to_state(AppState::Connect),
        // )
        // .add_collection_to_loading_state::<_, AppAssets>(AppState::Loading)
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "Power, Baby! (ONLINE)".into(),
                resolution: (960., 1080.).into(),
                fit_canvas_to_parent: true,
                prevent_default_event_handling: false,
                ..default()
            }),
            ..default()
        }))
        .add_plugin(WorldInspectorPlugin::new())
        .add_plugin(OverlayPlugin {
            font_size: 32.0,
            ..default()
        })
        .add_plugin(RapierPhysicsPlugin::<NoUserData>::default())
        .add_plugin(NaiaClientPlugin::new(
            ClientConfig::default(),
            crate::protocol::protocol(),
        ))
        // Background Color
        // .insert_resource(ClearColor(Color::BLACK))
        .add_startup_system(init)
        .add_systems(
            (
                events::connect_events,
                events::disconnect_events,
                events::reject_events,
                events::spawn_entity_events,
                events::despawn_entity_events,
                events::insert_component_events,
                events::update_component_events,
                events::remove_component_events,
                events::message_events,
            )
                .chain()
                .in_set(ReceiveEvents),
        )
        .add_system(events::tick_events.in_set(Tick))
        .add_systems(
            (
                input::camera,
                input::ball,
                sync::serverside_entities,
                debug_overlay,
            )
                .chain()
                .in_set(MainLoop),
        )
        .configure_set(Tick.after(ReceiveEvents))
        .configure_set(MainLoop.after(Tick))
        .add_system(bevy::window::close_on_esc)
        // Run App
        .run();
}
