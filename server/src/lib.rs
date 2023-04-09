use core::{
    components::{Ball, GoalieBehavior},
    constants,
    systems::{goalie, magnus_effect},
};

/// #NOTES
/// Client controlled entity is handled like so:
/// - Protocol needs to enable "client_authoritative_entities"
/// - Client needs to insert an entity with "enable_replication(client)"
/// - Server will receive "InsertComponentEvents"
/// - Server needs to spawn a "shadowed" entity and map the client_entity to the server_entity
/// - Server will receive "UpdateComponentEvents" from the client when the client changes component
/// fields.  The server then needs to query both entity's components and "mirror" the server's
/// component with the client's component.
/// - The server_entity will be replicated to other clients
use protocol::{
    self,
    channels::{EntityAssignmentChannel, PlayerCommandChannel},
    components::{EntityKind, RepPhysics, UpdateWith},
    messages::{Auth, EntityAssignment, KeyCommand},
};

use std::{collections::HashMap, time::Duration};

use bevy::app::{ScheduleRunnerPlugin, ScheduleRunnerSettings};
use bevy::asset::AssetPlugin;
use bevy::log::LogPlugin;
use bevy::prelude::*;
use bevy::scene::ScenePlugin;
use bevy::time::TimePlugin;
use bevy_rapier3d::prelude::*;
use bevy_turborand::prelude::*;

use naia_bevy_server::{
    events::{
        AuthEvents, ConnectEvent, DespawnEntityEvent, DisconnectEvent, ErrorEvent,
        InsertComponentEvents, RemoveComponentEvents, SpawnEntityEvent, TickEvent,
        UpdateComponentEvents,
    },
    transport::webrtc,
    CommandsExt, Plugin as ServerPlugin, ReceiveEvents, RoomKey, Server, ServerConfig, UserKey,
};
use naia_bevy_shared::BeforeReceiveEvents;

#[derive(Resource)]
pub struct Global {
    pub goalie_entity: Entity,
    pub main_room_key: RoomKey,
    pub user_ball_map: HashMap<UserKey, Entity>,
}

pub fn auth_events(mut server: Server, mut event_reader: EventReader<AuthEvents>) {
    for events in event_reader.iter() {
        for (user_key, auth) in events.read::<Auth>() {
            if auth.username == "charlie" && auth.password == "12345" {
                // Accept incoming connection
                server.accept_connection(&user_key);
            } else {
                // Reject incoming connection
                server.reject_connection(&user_key);
            }
        }
    }
}

pub fn connect_events(
    mut commands: Commands,
    mut server: Server,
    mut global: ResMut<Global>,
    mut event_reader: EventReader<ConnectEvent>,
) {
    for ConnectEvent(user_key) in event_reader.iter() {
        let address = server
            .user_mut(user_key)
            // Add User to the main Room
            .enter_room(&global.main_room_key)
            // Get User's address for logging
            .address();

        info!("Naia Server connected to Client: {}", address);

        let ball_transform =
            TransformBundle::from_transform(Transform::from_translation(constants::BALL_START));
        let ball_velocity = Velocity::zero();
        let ball_rep_physics = RepPhysics::new_with(&ball_transform.local, &ball_velocity);
        let ball_entity = commands
            .spawn((
                EntityKind::ball(),
                Ball::default(),
                // Position::from(constants::BALL_START),
                ball_rep_physics,
                TransformBundle::from_transform(Transform::from_translation(constants::BALL_START)),
                RigidBody::Dynamic,
                // Group2 is the ball group.  Group1 is the goal/goalie group
                CollisionGroups::new(Group::GROUP_2, Group::GROUP_1),
                Collider::ball(constants::BALL_RADIUS),
                ColliderMassProperties::Mass(constants::BALL_MASS),
                ball_velocity,
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
            ))
            .insert(Sleeping::default())
            .enable_replication(&mut server)
            .id();

        server
            .room_mut(&global.main_room_key)
            .add_entity(&ball_entity);
        global.user_ball_map.insert(*user_key, ball_entity);

        // Send an Entity Assignment message to the User that owns the Square
        let mut assignment_message = EntityAssignment::new(true);
        assignment_message.entity.set(&server, &ball_entity);
        server.send_message::<EntityAssignmentChannel, EntityAssignment>(
            user_key,
            &assignment_message,
        );
    }
}

// Destroy User's entities
pub fn disconnect_events(
    // mut commands: Commands,
    // mut server: Server,
    // mut global: ResMut<Global>,
    mut event_reader: EventReader<DisconnectEvent>,
) {
    for DisconnectEvent(_user_key, user) in event_reader.iter() {
        info!("Naia Server disconnected from: {:?}", user.address);

        // if let Some(entity) = global.user_to_square_map.remove(user_key) {
        //     commands.entity(entity).despawn();
        //     server
        //         .room_mut(&global.main_room_key)
        //         .remove_entity(&entity);
        // }
    }
}

pub fn error_events(mut event_reader: EventReader<ErrorEvent>) {
    for ErrorEvent(error) in event_reader.iter() {
        info!("Naia Server Error: {:?}", error);
    }
}

pub fn tick_events(
    mut server: Server,
    mut ball_query: Query<(&mut Transform, &mut Ball, &mut ExternalImpulse)>,
    mut tick_reader: EventReader<TickEvent>,
) {
    let mut has_ticked = false;

    for TickEvent(server_tick) in tick_reader.iter() {
        has_ticked = true;

        // All game logic should happen here, on a tick event

        let mut messages = server.receive_tick_buffer_messages(server_tick);
        for (_user_key, key_command) in messages.read::<PlayerCommandChannel, KeyCommand>() {
            let Some(entity) = &key_command.entity.get(&server) else {
                continue;
            };

            if let Ok((mut transform, mut ball, mut ext_i)) = ball_query.get_mut(*entity) {
                // let ray_normal = Vec3::new(0.015694855, -0.011672409, 0.9998087);
                // let ray_point = Vec3::new(0.0017264052, 0.0070980787, 42.109978);
                process_ball_command(key_command, &mut transform, &mut ball, &mut ext_i);
            }
        }
    }

    if has_ticked {
        // Update scopes of entities
        for (_, user_key, entity) in server.scope_checks() {
            // You'd normally do whatever checks you need to in here..
            // to determine whether each Entity should be in scope or not.

            // This indicates the Entity should be in this scope.
            server.user_scope(&user_key).include(&entity);

            // And call this if Entity should NOT be in this scope.
            // server.user_scope(..).exclude(..);
        }
    }
}

pub fn spawn_entity_events(mut event_reader: EventReader<SpawnEntityEvent>) {
    for SpawnEntityEvent(_, _) in event_reader.iter() {
        info!("spawned client entity");
    }
}

pub fn despawn_entity_events(mut event_reader: EventReader<DespawnEntityEvent>) {
    for DespawnEntityEvent(_, _) in event_reader.iter() {
        info!("despawned client entity");
    }
}

pub fn insert_component_events(
    // mut commands: Commands,
    // mut server: Server,
    // mut global: ResMut<Global>,
    mut event_reader: EventReader<InsertComponentEvents>,
    // position_query: Query<&Position>,
) {
    for _events in event_reader.iter() {
        // for (user_key, client_entity) in events.read::<Position>() {
        //     info!("insert component into client entity");
        // }
    }
}

pub fn update_component_events(
    // global: ResMut<Global>,
    mut event_reader: EventReader<UpdateComponentEvents>,
    // mut position_query: Query<&mut Position>,
) {
    for _events in event_reader.iter() {
        // for (_user_key, client_entity) in events.read::<Position>() {
        //     info!("client authoritative entity component update")
        // }
    }
}

pub fn remove_component_events(mut event_reader: EventReader<RemoveComponentEvents>) {
    for _events in event_reader.iter() {
        // for (_user_key, _entity, _component) in events.read::<Position>() {
        //     info!("removed Position component from client entity");
        // }
    }
}

pub fn process_ball_command(
    key_command: KeyCommand,
    transform: &mut Transform,
    ball: &mut Ball,
    ext_i: &mut ExternalImpulse,
) {
    if key_command.reset && ball.shot && !ball.scored {
        ball.force_reset = true;
        return;
    }

    //#HACK we stop shots from happening unless the ball is settled on the floor
    //this works for now minus the spamming inbetween the bounce of the ball when spawned.
    //probably should set this to a cooldown.  Maybe the cooldown can be implemented in the client
    //side only for now
    let Some((ray_normal, ray_point)) = key_command.shoot else {
        return;
    };

    if !ball.shot && transform.translation.y < 0.01 {
        let ray_normal = Vec3::new(ray_normal.x, ray_normal.y - 0.8, ray_normal.z);
        let ray_point = Vec3::new(ray_point.x, ray_point.y, ray_point.z);
        let kick_force = Vec3::new(-2.0, -3.0, -13.0);
        let impulse = ray_normal * kick_force;
        // let impulse_camera = camera_rotation.normalize() * force.neg();
        *ext_i = ExternalImpulse::at_point(impulse, ray_point, transform.translation);
        ext_i.torque_impulse = ext_i.torque_impulse * 0.15;
        // ext_i.impulse = impulse_camera;
        ball.shot = true;
    }
}

pub fn ball_score(
    mut collision_events: EventReader<CollisionEvent>,
    mut ball_query: Query<&mut Ball>,
) {
    for event in collision_events.iter() {
        // log::info!("Received collision event: {:?}", event);
        if let CollisionEvent::Started(_, entity, _) = event {
            let mut ball = ball_query.get_mut(*entity).unwrap();
            ball.scored = true;
        }
    }
}

pub fn ball_reset(
    time: Res<Time>,
    mut ball_query: Query<(
        &mut Transform,
        &mut Ball,
        &mut ExternalForce,
        &mut ExternalImpulse,
        &mut Velocity,
    )>,
) {
    for (mut transform, mut ball, mut ext_f, mut ext_i, mut velocity) in ball_query.iter_mut() {
        if ball.shot {
            ball.shot_elapsed += time.delta_seconds();
        }

        if ball.shot_elapsed >= constants::BALL_SHOT_WAIT_TIME
            || ball.force_reset
            || (ball.scored && ball.shot_elapsed >= 2.0)
        {
            *ext_f = ExternalForce::default();
            *ext_i = ExternalImpulse::default();
            *velocity = Velocity::zero();
            *transform = Transform::from_translation(constants::BALL_START);
            ball.shot = false;
            ball.scored = false;
            ball.force_reset = false;
            ball.shot_elapsed = 0.0;
        }
    }
}

// in server after physics systems before naia 'ReceiveEvents' systems
pub fn sync_physics(mut query: Query<(&Transform, &Velocity, &mut RepPhysics)>) {
    for (transform, velocity, mut physics_properties) in query.iter_mut() {
        physics_properties.update_with((transform, velocity));
    }
}

pub fn init(
    mut commands: Commands,
    mut server: Server,
    // mut rapier_config: ResMut<RapierConfiguration>,
) {
    info!("Naia Bevy Server Demo init");

    // rapier_config.timestep_mode = TimestepMode::Fixed {
    //     // dt: constants::TIME_STEP,
    //     dt: 1.0 / 30.0,
    //     substeps: 1,
    // };

    // Naia Server initialization
    let server_addresses = webrtc::ServerAddrs::new(
        "127.0.0.1:14191"
            .parse()
            .expect("could not parse Signaling address/port"),
        // IP Address to listen on for UDP WebRTC data channels
        "127.0.0.1:14192"
            .parse()
            .expect("could not parse WebRTC data address/port"),
        // The public WebRTC IP address to advertise
        "http://127.0.0.1:14192",
    );
    let socket = webrtc::Socket::new(&server_addresses, server.socket_config());
    server.listen(socket);

    // Create a new, singular room, which will contain Users and Entities that they
    // can receive updates from
    let main_room_key = server.make_room().key();

    let goalie_entity = init_physics(&mut commands, &mut server, &main_room_key);
    // Resources
    commands.insert_resource(Global {
        goalie_entity,
        main_room_key,
        user_ball_map: HashMap::new(),
    })
}

pub fn init_physics(
    commands: &mut Commands,
    server: &mut Server,
    main_room_key: &RoomKey,
) -> Entity {
    log::info!("init_physics");

    //#NOTE this is not a replicated entity, the client must render this in the init function.  the
    //ground never changes.
    commands.spawn((
        Name::new("Ground"),
        TransformBundle::from(Transform::from_xyz(0.0, constants::GROUND_HEIGHT, 0.0)),
        Collider::cuboid(constants::GROUND_SIZE, 0.0, constants::GROUND_SIZE),
        RigidBody::KinematicPositionBased,
        Friction::new(100.0),
    ));

    // Create a goal rigid-body with multiple colliders attached, using Bevy hierarchy.
    let x = 0.0;
    let y = constants::GROUND_HEIGHT;
    let z = 32.0;
    let rad = 0.2;
    commands
        .spawn((
            Name::new("Goal"),
            TransformBundle::from(Transform::from_xyz(x, y, z)),
            RigidBody::KinematicPositionBased,
            CollisionGroups::new(Group::GROUP_1, Group::GROUP_2),
        ))
        .with_children(|p| {
            p.spawn((
                Name::new("FrameTop"),
                TransformBundle::from(Transform::from_xyz(0.0, rad * 10.0, 0.0)),
                Collider::cuboid(rad * 10.0, rad * 0.5, rad),
            ));
            p.spawn((
                Name::new("FrameLeft"),
                TransformBundle::from(Transform::from_xyz(rad * 10.0, rad * 5.0, 0.0)),
                Collider::cuboid(rad * 0.5, rad * 5.0, rad),
            ));
            p.spawn((
                Name::new("FrameRight"),
                TransformBundle::from(Transform::from_xyz(-rad * 10.0, rad * 5.0, 0.0)),
                Collider::cuboid(rad * 0.5, rad * 5.0, rad),
            ));
            p.spawn((
                Name::new("PointZone"),
                TransformBundle::from(Transform::from_xyz(0.0, rad * 5.0, (-rad * 0.5) - rad)),
                Sensor,
                Collider::cuboid(rad * 10.0, rad * 5.0, rad * 0.5),
                ActiveEvents::COLLISION_EVENTS,
            ));
        });

    let goalie_transform =
        TransformBundle::from_transform(Transform::from_translation(constants::GOALIE_START));
    let goalie_velocity = Velocity::zero();
    let goalie_rep_physics = RepPhysics::new_with(&goalie_transform.local, &goalie_velocity);
    let goalie = commands
        .spawn((
            Name::new("Goalie"),
            EntityKind::goalie(),
            GoalieBehavior::default(),
            goalie_rep_physics,
            // Sensor,
            TransformBundle::from_transform(Transform::from_translation(constants::GOALIE_START)),
            RigidBody::KinematicPositionBased,
            CollisionGroups::new(Group::GROUP_1, Group::GROUP_2),
            Collider::capsule_y(constants::GOALIE_HEIGHT * 0.5, constants::GOALIE_RADIUS),
            GravityScale::default(),
            Damping {
                linear_damping: 1.0,
                angular_damping: 5.0,
            },
            Restitution {
                coefficient: 1.0,
                combine_rule: CoefficientCombineRule::Average,
            },
            goalie_velocity,
        ))
        .enable_replication(server)
        .id();

    server.room_mut(main_room_key).add_entity(&goalie);

    goalie
}

pub fn run() {
    info!("powerbaby server startup");

    App::default()
        .add_plugin(TaskPoolPlugin::default())
        .add_plugin(TypeRegistrationPlugin::default())
        .add_plugin(FrameCountPlugin::default())
        .insert_resource(
            // this is needed to avoid running the server at uncapped FPS
            ScheduleRunnerSettings::run_loop(Duration::from_millis(3)),
        )
        .add_plugin(ScheduleRunnerPlugin::default())
        .add_plugin(LogPlugin::default())
        // Rapier Headless Requirements
        // https://github.com/dimforge/bevy_rapier/issues/296
        // https://github.com/dimforge/bevy_rapier/pull/306
        .add_plugin(AssetPlugin::default())
        .add_plugin(ScenePlugin::default())
        .add_plugin(TimePlugin::default())
        .add_asset::<Mesh>()
        .add_asset::<Scene>()
        .add_plugin(RapierPhysicsPlugin::<NoUserData>::default())
        .add_plugin(RngPlugin::new().with_rng_seed(0772))
        .add_plugin(ServerPlugin::new(
            ServerConfig::default(),
            protocol::protocol(),
        ))
        // Startup System
        .add_startup_system(init)
        .add_systems(
            (
                auth_events,
                connect_events,
                disconnect_events,
                error_events,
                tick_events,
                spawn_entity_events,
                despawn_entity_events,
                insert_component_events,
                update_component_events,
                remove_component_events,
            )
                .chain()
                .in_set(ReceiveEvents),
        )
        // .configure_set(ReceiveEvents.after(PhysicsSet::Writeback))
        .insert_resource(FixedTime::new_from_secs(constants::TIME_STEP))
        .edit_schedule(CoreSchedule::FixedUpdate, |schedule| {
            schedule.add_systems((goalie, magnus_effect).after(PhysicsSet::Writeback));
        })
        .add_systems((sync_physics, ball_reset, ball_score).in_set(BeforeReceiveEvents))
        .run();
}
