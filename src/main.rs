pub mod client;
pub mod protocol;
pub mod server;

///Guides
/// https://bevy-cheatbook.github.io/features/parent-child.html
/// https://bevyengine.org/learn/book/migration-guides/0.9-0.10/#states
/// https://bevyengine.org/news/bevy-0-10/
///3D Examples:
/// https://github.com/alexichepura/bevy_garage/blob/main/src/car.rs
use std::f32::consts::*;

use clap::Parser;

use bevy::{prelude::*, render::camera::RenderTarget, window::PrimaryWindow};
use bevy_turborand::prelude::*;
// use bevy_mod_picking::{
//     DebugCursorPickingPlugin, DebugEventsPickingPlugin, DefaultPickingPlugins, PickableBundle,
//     PickingCameraBundle, PickingEvent,
// };
use bevy_rapier3d::prelude::*;
use iyes_progress::prelude::*;

// Debugging
use bevy_debug_text_overlay::{screen_print, OverlayPlugin};
use bevy_inspector_egui::prelude::*;
use bevy_inspector_egui::quick::{ResourceInspectorPlugin, WorldInspectorPlugin};

#[derive(Debug, Parser)]
#[clap(name = "powerbaby", about = "PowerBaby Baby Shower")]
pub struct Cli {
    /// Subcommands
    #[clap(subcommand)]
    pub subcommand: Subcommand,
}

#[derive(Debug, Parser)]
pub enum Subcommand {
    /// original single player game
    Single,
    /// run client only
    Client,
    /// run server only
    Server,
    /// run standalone with server + client
    Standalone,
}

// Defines the amount of time that should elapse between each step.  This is essentially
// a "target" of 60 updates per second
// const TIME_STEP: f32 = 1.0 / 60.0;
pub const TIME_STEP: f32 = 1.0 / 30.0;

pub const BALL_RADIUS: f32 = 0.11;
pub const BALL_MASS: f32 = 0.45;
pub const BALL_START: Vec3 = Vec3::new(0.0, BALL_RADIUS * 10.0, 42.0);

pub const GROUND_HEIGHT: f32 = -0.1;
pub const GROUND_SIZE: f32 = 100.0;
pub const GOALIE_HEIGHT: f32 = 0.6;
pub const GOALIE_RADIUS: f32 = 0.4;
pub const GOALIE_START: Vec3 = Vec3::new(0.0, GOALIE_HEIGHT, 32.8);
pub const GOALIE_PATROL_MAX_X: f32 = 1.5;
pub const GOALIE_PATROL_MIN_X: f32 = -1.5;

pub const MAGNUS_AIR_DENSITY: f32 = 3.225; // kg/m^3
pub const MAGNUS_CONSTANT: f32 = 4.0 / 3.0 * PI * MAGNUS_AIR_DENSITY * 0.001331; //f32::powf(BALL_RADIUS, 3.0);
pub const BALL_SHOT_WAIT_TIME: f32 = 5.0; // wait 5 seconds

//Camera
pub const BIRDS_EYE_CAM: Transform = Transform::from_xyz(0.0, 17.7, 37.7);
pub const BIRDS_EYE_CAM_LOOK: Vec3 = Vec3::new(0.0, -500.0, 0.0);
pub const KICK_CAM: Transform = Transform::from_xyz(0.0, 1.0, 43.8);
pub const KICK_CAM_LOOK: Vec3 = Vec3::new(0.0, -7.0, 0.0);

#[derive(Resource, Reflect, InspectorOptions, Default)]
#[reflect(Resource, InspectorOptions)]
pub struct Game {
    ball_entity: Option<Entity>,
    goalie_entity: Option<Entity>,
    #[inspector(min = 0.0, max = 1.0)]
    power: f32,
    power_charge: bool,
    goal: bool,
    shot: bool,
    camera_is_birdseye: bool,
}

#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash, Default, States)]
pub enum AppState {
    #[default]
    Splash,
    Staging,
    InGame,
    Ending,
}

#[derive(Debug, Hash, PartialEq, Eq, Clone, SystemSet)]
#[system_set(base)]
pub struct GameSet;

#[derive(Debug, Hash, PartialEq, Eq, Clone, SystemSet)]
#[system_set(base)]
pub struct PhysicsSet;

pub enum ControllerEvent {
    Reset,
    Shoot {
        ray: RayIntersection,
        camera_rotation: Quat,
    },
}

fn main() {
    let cli = Cli::parse();
    match cli.subcommand {
        Subcommand::Single => game_app(),
        Subcommand::Client => client::run(),
        Subcommand::Server => server::run(),
        Subcommand::Standalone => {
            std::thread::spawn(|| server::run());
            client::run();
        }
    }
}

pub fn game_app() {
    App::new()
        .init_resource::<Game>()
        // .register_type::<Game>()
        // .register_type::<Cow<'static, str>>()
        // .register_type::<time::Duration>()
        // .register_type::<time::Instant>()
        // .add_plugin(ResourceInspectorPlugin::<Game>::default())
        .add_state::<AppState>()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "Power, Baby!".into(),
                resolution: (1920., 1080.).into(),
                fit_canvas_to_parent: true,
                prevent_default_event_handling: false,
                ..default()
            }),
            ..default()
        }))
        .add_plugin(WorldInspectorPlugin::new())
        .add_plugin(RngPlugin::new().with_rng_seed(0772))
        // .add_plugins(DefaultPickingPlugins) // <- Adds picking, interaction, and highlighting
        // .add_plugin(DebugCursorPickingPlugin) // <- Adds the debug cursor (optional)
        // .add_plugin(DebugEventsPickingPlugin) // <- Adds debug event logging (optional)
        .add_plugin(OverlayPlugin {
            font_size: 32.0,
            ..default()
        })
        .add_plugin(RapierPhysicsPlugin::<NoUserData>::default())
        .add_plugin(RapierDebugRenderPlugin::default())
        .add_plugin(
            ProgressPlugin::new(AppState::Splash)
                .continue_to(AppState::Staging)
                .track_assets(),
        )
        .add_plugin(ProgressPlugin::new(AppState::Staging).continue_to(AppState::InGame))
        .add_system(load_assets.in_schedule(OnEnter(AppState::Splash)))
        .add_system(staging.in_schedule(OnEnter(AppState::Staging)))
        //#NOTE this is an escape hatch because 'in_schedule' is not yet implemented for SystemSet
        .edit_schedule(CoreSchedule::FixedUpdate, |schedule| {
            schedule
                .add_systems((goalie, magnus_effect).in_base_set(PhysicsSet))
                .configure_set(PhysicsSet.run_if(in_state(AppState::InGame)));
        })
        .insert_resource(FixedTime::new_from_secs(TIME_STEP))
        .add_event::<ControllerEvent>()
        .add_systems((debug_overlay, controls, game_logic).in_base_set(GameSet))
        .configure_set(GameSet.run_if(in_state(AppState::InGame)))
        .add_system(bevy::window::close_on_esc)
        .run();
}

// //#TODO upstream this.  Since it's FnMut without Clone on upstream, it cannot be used in
// //'distributive_run_if'
// pub fn in_state<S: States>(state: S) -> impl Fn(Res<State<S>>) -> bool + Clone {
//     move |current_state: Res<State<S>>| current_state.0 == state
// }

#[derive(Component, Default)]
pub struct GoalieBehavior {
    seconds_left: f32,
    direction: f32,
    speed: f32,
}

#[derive(Component, Default)]
pub struct Ball {
    pub shot: bool,
    pub scored: bool,
    pub force_reset: bool,
    pub shot_elapsed: f32,
}

pub fn goalie(
    time: Res<Time>,
    // game: Res<Game>,
    mut goalie_query: Query<(&mut Transform, &mut GoalieBehavior), Without<ExternalImpulse>>,
    ball_query: Query<(&Transform, &Ball), With<ExternalImpulse>>,
    mut rand: ResMut<GlobalRng>,
) {
    const SPEED: &[f32] = &[1.5, 2.0, 3.0, 4.0];
    const ACTION_TIMES: &[f32] = &[0.1, 0.01, 0.15, 0.05];
    const DIRECTION: &[f32] = &[0.0, 1.0, -1.0];

    let (mut goalie_transform, mut goalie) = goalie_query.get_single_mut().unwrap();

    // filter by balls that have kicked_elapsed > 0
    // Then sort them by the MIN z axis to find the closest ball
    // to the goalie.  We then move the goalie towards the ball (if it exists).  Otherwise
    // move normally if nothing was found.
    let new_goalie_pos = ball_query
        .iter()
        .filter(|(_, b)| b.shot_elapsed > 0.0)
        .min_by(|a, b| a.0.translation.z.partial_cmp(&b.0.translation.z).unwrap())
        .map(|q| {
            let to_x = q.0.translation.x - goalie_transform.translation.x;
            let speed = goalie.speed * 3.0;
            goalie_transform.translation.x + to_x * speed * TIME_STEP
        })
        .unwrap_or_else(|| {
            goalie_transform.translation.x + goalie.direction * goalie.speed * TIME_STEP
        });
    goalie_transform.translation.x = new_goalie_pos.clamp(GOALIE_PATROL_MIN_X, GOALIE_PATROL_MAX_X);

    // Reroll period
    goalie.seconds_left -= time.delta_seconds();
    if goalie.seconds_left <= 0.0 {
        let rand = rand.get_mut();
        goalie.seconds_left = *rand.sample(ACTION_TIMES).unwrap();
        goalie.speed = *rand.sample(SPEED).unwrap();

        match goalie_transform.translation.x {
            x if x == GOALIE_PATROL_MIN_X => goalie.direction = 1.0,
            x if x == GOALIE_PATROL_MAX_X => goalie.direction = -1.0,
            _ => {
                goalie.direction = *rand.sample(DIRECTION).unwrap();
            }
        }
    }
}

pub fn magnus_effect(
    time: Res<Time>,
    mut ball_query: Query<(&Transform, &mut ExternalForce, &Velocity)>,
) {
    // log::info!("magnus_effect");
    for (transform, mut ext_f, velocity) in ball_query.iter_mut() {
        if transform.translation.y > 0.21 {
            let velocity = velocity.angvel.cross(velocity.linvel);
            let magnus_force = MAGNUS_CONSTANT * time.delta_seconds();
            ext_f.force += magnus_force * velocity;
            // screen_print!(col: Color::CYAN, "velocity: {}", velocity);
            // screen_print!(col: Color::CYAN, "magnus_force: {}", magnus_force);
            // screen_print!(col: Color::CYAN, "total_force: {}", ext_f.force);
        } else {
            ext_f.force = Vec3::ZERO;
        }
        // screen_print!(col: Color::CYAN, "ball: {}", transform.translation);
    }
}

pub fn game_logic(
    time: Res<Time>,
    mut game: ResMut<Game>,
    mut ball_query: Query<(
        &mut Transform,
        &mut Ball,
        &mut ExternalForce,
        &mut ExternalImpulse,
        &mut Velocity,
    )>,

    mut collision_events: EventReader<CollisionEvent>,
    mut controller_events: EventReader<ControllerEvent>,
) {
    let (mut transform, mut ball, mut ext_f, mut ext_i, mut velocity) =
        ball_query.get_mut(game.ball_entity.unwrap()).unwrap();

    for _event in collision_events.iter() {
        // log::info!("Received collision event: {:?}", event);
        game.goal = true;
    }

    let mut should_reset = false;
    for event in controller_events.iter() {
        match event {
            ControllerEvent::Reset => {
                should_reset = true;
            }
            ControllerEvent::Shoot {
                ray,
                camera_rotation: _,
            } => {
                log::info!("SHOOT {:?}", ray);
                let ray_normal = Vec3::new(ray.normal.x, ray.normal.y - 0.8, ray.normal.z);
                let kick_force = Vec3::new(-2.0, -3.0, -13.0);
                let impulse = ray_normal * kick_force;
                // let impulse_camera = camera_rotation.normalize() * force.neg();
                *ext_i = ExternalImpulse::at_point(impulse, ray.point, transform.translation);
                ext_i.torque_impulse = ext_i.torque_impulse * 0.15;
                // ext_i.impulse = impulse_camera;
                game.shot = true;
                // screen_print!(col: Color::CYAN, "ray_point: {}", ray.point);
                // screen_print!(col: Color::CYAN, "ray_normal: {}", ray.normal);
                // screen_print!(col: Color::CYAN, "ray_normal adjusted: {}", ray_normal);
                // screen_print!(col: Color::CYAN, "impulse: {}", impulse);
            }
        }
    }

    if game.shot && !game.goal {
        // reset the ball if it has been shot with no goal after BALL_SHOT_WAIT_TIME seconds
        ball.shot_elapsed += time.delta_seconds();
        if ball.shot_elapsed >= BALL_SHOT_WAIT_TIME {
            should_reset = true;
        }
    }

    if should_reset {
        log::info!("BALL RESET");
        *ext_f = ExternalForce::default();
        *ext_i = ExternalImpulse::default();
        *velocity = Velocity::zero();
        *transform = Transform::from_translation(BALL_START);
        ball.shot_elapsed = 0.0;
        game.shot = false;
        game.goal = false;
    }
}

// fn trajectory_system(
//     mut commands: Commands,
//     mut materials: ResMut<Assets<StandardMaterial>>,
//     ball_query: Query<(&Transform, &RigidBodyVelocity, &RigidBodyMassProps), With<Ball>>,
// ) {
//     if let Ok((ball_transform, ball_vel, ball_mass_props)) = ball_query.single() {
//         let start_pos = ball_transform.translation;
//         let start_vel = ball_vel.linvel;
//
//         // Calculate the estimated trajectory using physics simulation
//         let mut pos = start_pos;
//         let mut vel = start_vel;
//         let gravity = Vector3::new(0.0, -9.81 * ball_mass_props.mass, 0.0);
//         let time_step = 0.1;
//         let mut trajectory_points = vec![pos];
//
//         for _ in 0..100 {
//             let acc = gravity
//                 + ball_mass_props.inv_mass
//                     * ball_mass_props.local_inertia_sqrt
//                     * ball_mass_props.local_inverse_inertia_sqrt
//                     * ball_mass_props.local_inertia
//                     * ball_mass_props.local_inverse_inertia
//                     * (ball_mass_props.local_inertia_sqrt
//                         * ball_mass_props.local_inverse_inertia_sqrt
//                         * vel)
//                         .cross(vel);
//             vel += acc * time_step;
//             pos += vel * time_step;
//             trajectory_points.push(pos);
//
//             if pos.y < 0.0 {
//                 break;
//             }
//         }
//
//         // Draw a line primitive for the estimated trajectory of the ball
//         for i in 0..trajectory_points.len() - 1 {
//             let start_pos = trajectory_points[i];
//             let end_pos = trajectory_points[i + 1];
//             commands.spawn_bundle(PbrBundle {
//                 mesh: meshes::Line::new(start_pos.into(), end_pos.into()),
//                 material: materials.add(Color::RED.into()),
//                 transform: Transform::default(),
//                 ..Default::default()
//             });
//         }
//     }
// }

pub fn debug_overlay(time: Res<Time>, game: Res<Game>) {
    let current_time = time.elapsed_seconds_f64();
    let at_interval = |t: f64| current_time % t < time.delta_seconds_f64();
    if at_interval(0.1) {
        let last_fps = 0.0 / time.delta_seconds();
        screen_print!(col: Color::CYAN, "fps: {last_fps:.0}");
    }
    if game.shot {
        let col = Color::FUCHSIA;
        screen_print!(sec: 0.5, col: col, "power: {}", game.power);
        // screen_print!(sec: 0.5, col: col, "shot_elapsed: {}", game.shot_elapsed);
        screen_print!(sec: 0.5, col: col, "goal: {}", game.goal);
    }
}

pub fn controls(
    time: Res<Time>,
    mut game: ResMut<Game>,
    // mut set: ParamSet<(
    //     Query<(&mut Transform, &GlobalTransform), With<Camera3d>>,
    //     Query<(
    //         &mut Transform,
    //         &mut ExternalForce,
    //         &mut ExternalImpulse,
    //         &mut Velocity,
    //     )>,
    // )>,
    mut mouse_buttons: ResMut<Input<MouseButton>>,
    mut keyboard_input: ResMut<Input<KeyCode>>,

    mut camera_query: Query<(&Camera, &mut Transform, &GlobalTransform)>,
    // mut ball_query: Query<&Transform, With<ExternalImpulse>>,

    // cursor ray
    window: Query<&Window, With<PrimaryWindow>>,
    //#TODO figure out how to clone out the necessary entities in the Rapier Context
    //in order to calculate the
    rapier_context: Res<RapierContext>,

    mut controller_events: EventWriter<ControllerEvent>,
) {
    let (camera, mut camera_transform, camera_global_transform) = camera_query.single_mut();

    if keyboard_input.just_pressed(KeyCode::C) {
        game.camera_is_birdseye = !game.camera_is_birdseye;
        if game.camera_is_birdseye {
            *camera_transform = BIRDS_EYE_CAM.looking_at(BIRDS_EYE_CAM_LOOK, Vec3::Y);
        } else {
            *camera_transform = KICK_CAM.looking_at(KICK_CAM_LOOK, Vec3::Y);
        }
    }

    // let mut ball_query = set.p1();
    // let (mut transform, mut ext_f, mut ext_i, mut velocity) =
    //     ball_query.get_mut(game.ball_entity.unwrap()).unwrap();
    // screen_print!(col: Color::CYAN, "Ball: {:?}", transform.translation);
    // // screen_print!(col: Color::CYAN, "Ball LV: {:?}", velocity.linvel);
    // // screen_print!(col: Color::CYAN, "Ball AV: {:?}", velocity.angvel);

    if keyboard_input.just_pressed(KeyCode::Q) {
        controller_events.send(ControllerEvent::Reset);
    }

    if mouse_buttons.pressed(MouseButton::Left) {
        // log::info!("SHOT CHARGING");
    }

    if game.shot {
        return;
    }

    if keyboard_input.pressed(KeyCode::Left) {
        camera_transform.rotate_around(
            BALL_START,
            Quat::from_euler(EulerRot::XYZ, 0.0, time.delta_seconds(), 0.0),
        );
    } else if keyboard_input.pressed(KeyCode::Right) {
        camera_transform.rotate_around(
            BALL_START,
            Quat::from_euler(EulerRot::XYZ, 0.0, -time.delta_seconds(), 0.0),
        );
    }

    // get the window that the camera is displaying to (or the primary window)
    let window = if let RenderTarget::Window(id) = camera.target {
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
        return
    };
    // screen_print!(col: Color::CYAN, "Ray Orig: {:?}", ray.origin);
    // screen_print!(col: Color::CYAN, "Ray Dir: {:?}", ray.direction);

    if let Some((entity, intersection)) = rapier_context.cast_ray_and_get_normal(
        ray.origin,
        ray.direction,
        100.0,
        true,
        Default::default(),
    ) {
        if entity != game.ball_entity.unwrap() {
            return;
        }

        if !game.shot
            && (keyboard_input.just_released(KeyCode::Space)
                || mouse_buttons.just_released(MouseButton::Left))
        {
            controller_events.send(ControllerEvent::Shoot {
                ray: intersection,
                camera_rotation: camera_transform.rotation,
            });
        }
    }
}

pub fn staging(
    mut game: ResMut<Game>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut images: ResMut<Assets<Image>>,
    mut materials: ResMut<Assets<StandardMaterial>>,

    null_char: Res<NullCharacter>,
    mut commands: Commands,
) {
    log::info!("staging");

    commands.spawn((Camera3dBundle {
        transform: KICK_CAM.looking_at(KICK_CAM_LOOK, Vec3::Y),
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

    /*
     * Ground
     */
    commands.spawn((
        Name::new("Ground"),
        PbrBundle {
            mesh: meshes.add(shape::Plane::from_size(GROUND_SIZE * 2.0).into()),
            material: materials.add(StandardMaterial {
                base_color: Color::SEA_GREEN,
                perceptual_roughness: 1.0,
                ..default()
            }),
            transform: Transform::from_xyz(0.0, GROUND_HEIGHT, 0.0),
            ..default()
        },
        Collider::cuboid(GROUND_SIZE, 0.0, GROUND_SIZE),
        RigidBody::KinematicPositionBased,
        Friction::new(100.0),
    ));

    /*
     * Create the cubes
     */
    let mut color = 0;
    let colors = [
        Color::hsl(220.0, 1.0, 0.3),
        Color::hsl(180.0, 1.0, 0.3),
        Color::hsl(260.0, 1.0, 0.7),
    ];

    // Create a goal rigid-body with multiple colliders attached, using Bevy hierarchy.
    let x = 0.0;
    let y = GROUND_HEIGHT;
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
            RigidBody::KinematicPositionBased,
            CollisionGroups::new(Group::GROUP_1, Group::GROUP_2),
        ))
        .with_children(|p| {
            p.spawn((
                Name::new("FrameTop"),
                // TransformBundle::from(Transform::from_xyz(0.0, rad * 10.0, 0.0)),
                PbrBundle {
                    mesh: meshes
                        .add(shape::Box::new(rad * 10.0 * 2.0, rad * 0.5 * 2.0, rad * 2.0).into()),
                    material: materials.add(StandardMaterial {
                        base_color: Color::WHITE,
                        perceptual_roughness: 1.0,
                        ..default()
                    }),
                    transform: Transform::from_xyz(0.0, rad * 10.0, 0.0),
                    ..default()
                },
                Collider::cuboid(rad * 10.0, rad * 0.5, rad),
                ColliderDebugColor(colors[color % 3]),
            ));
            p.spawn((
                Name::new("FrameLeft"),
                // TransformBundle::from(Transform::from_xyz(rad * 10.0, rad * 5.0, 0.0)),
                PbrBundle {
                    mesh: meshes
                        .add(shape::Box::new(rad * 0.5 * 2.0, rad * 5.0 * 2.0, rad * 2.0).into()),
                    material: materials.add(StandardMaterial {
                        base_color: Color::WHITE,
                        perceptual_roughness: 1.0,
                        ..default()
                    }),
                    transform: Transform::from_xyz(rad * 10.0, rad * 5.0, 0.0),
                    ..default()
                },
                Collider::cuboid(rad * 0.5, rad * 5.0, rad),
                ColliderDebugColor(colors[color % 3]),
            ));
            p.spawn((
                Name::new("FrameRight"),
                // TransformBundle::from(Transform::from_xyz(-rad * 10.0, rad * 5.0, 0.0)),
                PbrBundle {
                    mesh: meshes
                        .add(shape::Box::new(rad * 0.5 * 2.0, rad * 5.0 * 2.0, rad * 2.0).into()),
                    material: materials.add(StandardMaterial {
                        base_color: Color::WHITE,
                        perceptual_roughness: 1.0,
                        ..default()
                    }),
                    transform: Transform::from_xyz(-rad * 10.0, rad * 5.0, 0.0),
                    ..default()
                },
                Collider::cuboid(rad * 0.5, rad * 5.0, rad),
                ColliderDebugColor(colors[color % 3]),
            ));
            p.spawn((
                Name::new("PointZone"),
                TransformBundle::from(Transform::from_xyz(0.0, rad * 5.0, (-rad * 0.5) - rad)),
                Sensor,
                Collider::cuboid(rad * 10.0, rad * 5.0, rad * 0.5),
                ColliderDebugColor(colors[1]),
                ActiveEvents::COLLISION_EVENTS,
            ));
        });

    let debug_material = materials.add(StandardMaterial {
        base_color_texture: Some(images.add(uv_debug_texture())),
        ..default()
    });

    game.goalie_entity = Some(
        commands
            .spawn((
                Name::new("Goalie"),
                GoalieBehavior::default(),
                // Sensor,
                // TransformBundle::from_transform(Transform::from_translation(GOALIE_START)),
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
                    material: debug_material.clone(),
                    transform: Transform::from_translation(GOALIE_START),
                    ..default()
                },
                RigidBody::KinematicPositionBased,
                CollisionGroups::new(Group::GROUP_1, Group::GROUP_2),
                Collider::capsule_y(GOALIE_HEIGHT * 0.5, GOALIE_RADIUS),
                ColliderDebugColor(colors[2]),
                GravityScale::default(),
                Damping {
                    linear_damping: 1.0,
                    angular_damping: 5.0,
                },
                Restitution {
                    coefficient: 1.0,
                    combine_rule: CoefficientCombineRule::Average,
                },
            ))
            .id(),
    );

    game.ball_entity = Some(
        commands
            .spawn((
                Name::new("Ball"),
                Ball::default(),
                PbrBundle {
                    mesh: meshes.add(
                        shape::UVSphere {
                            radius: BALL_RADIUS,
                            sectors: 36,
                            stacks: 18,
                        }
                        .into(),
                    ),
                    material: debug_material.clone(),
                    transform: Transform::from_translation(BALL_START),
                    ..default()
                },
                RigidBody::Dynamic,
                CollisionGroups::new(Group::GROUP_2, Group::GROUP_1),
                Collider::ball(BALL_RADIUS),
                ColliderMassProperties::Mass(BALL_MASS),
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
                Sleeping::disabled(),
            ))
            .id(),
    );
}

/// Creates a colorful test pattern
pub fn uv_debug_texture() -> Image {
    use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};
    const TEXTURE_SIZE: usize = 8;

    let mut palette: [u8; 32] = [
        255, 102, 159, 255, 255, 159, 102, 255, 236, 255, 102, 255, 121, 255, 102, 255, 102, 255,
        198, 255, 102, 198, 255, 255, 121, 102, 255, 255, 236, 102, 255, 255,
    ];

    let mut texture_data = [0; TEXTURE_SIZE * TEXTURE_SIZE * 4];
    for y in 0..TEXTURE_SIZE {
        let offset = TEXTURE_SIZE * y * 4;
        texture_data[offset..(offset + TEXTURE_SIZE * 4)].copy_from_slice(&palette);
        palette.rotate_right(4);
    }

    Image::new_fill(
        Extent3d {
            width: TEXTURE_SIZE as u32,
            height: TEXTURE_SIZE as u32,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        &texture_data,
        TextureFormat::Rgba8UnormSrgb,
    )
}

#[derive(Resource)]
pub struct NullCharacter {
    scene: Handle<Scene>,
    run: Handle<AnimationClip>,
    idle: Handle<AnimationClip>,
    walk: Handle<AnimationClip>,
}

pub fn load_assets(
    assets: Res<AssetServer>,
    mut commands: Commands,
    mut loading: ResMut<AssetsLoading>,
) {
    log::info!("load_assets");
    let nc = NullCharacter {
        scene: assets.load("models/char_null/scene.gltf#Scene0"),
        run: assets.load("models/char_null/scene.gltf#Animation0"),
        idle: assets.load("models/char_null/scene.gltf#Animation1"),
        walk: assets.load("models/char_null/scene.gltf#Animation2"),
    };
    loading.add(&nc.scene);
    loading.add(&nc.run);
    loading.add(&nc.idle);
    loading.add(&nc.walk);

    commands.insert_resource(nc);
}

// // Once the scene is loaded, start the animation
// fn setup_scene_once_loaded(
//     animations: Res<Animations>,
//     mut player: Query<&mut AnimationPlayer>,
//     mut done: Local<bool>,
// ) {
//     if !*done {
//         if let Ok(mut player) = player.get_single_mut() {
//             player.play(animations.0[0].clone_weak()).repeat();
//             *done = true;
//         }
//     }
// }

// fn animate_light_direction(
//     time: Res<Time>,
//     mut query: Query<&mut Transform, With<DirectionalLight>>,
// ) {
//     for mut transform in &mut query {
//         transform.rotation = Quat::from_euler(
//             EulerRot::ZYX,
//             0.0,
//             time.elapsed_seconds() * PI / 5.0,
//             -FRAC_PI_4,
//         );
//     }
// }

// fn keyboard_animation_control(
//     keyboard_input: Res<Input<KeyCode>>,
//     mut animation_player: Query<&mut AnimationPlayer>,
//     null_char: Res<NullCharacter>,
//     mut current_animation: Local<usize>,
// ) {
//     if let Ok(mut player) = animation_player.get_single_mut() {
//         if keyboard_input.just_pressed(KeyCode::Space) {
//             if player.is_paused() {
//                 player.resume();
//             } else {
//                 player.pause();
//             }
//         }
//
//         if keyboard_input.just_pressed(KeyCode::Up) {
//             let speed = player.speed();
//             player.set_speed(speed * 1.2);
//         }
//
//         if keyboard_input.just_pressed(KeyCode::Down) {
//             let speed = player.speed();
//             player.set_speed(speed * 0.8);
//         }
//
//         if keyboard_input.just_pressed(KeyCode::Left) {
//             let elapsed = player.elapsed();
//             player.set_elapsed(elapsed - 0.1);
//         }
//
//         if keyboard_input.just_pressed(KeyCode::Right) {
//             let elapsed = player.elapsed();
//             player.set_elapsed(elapsed + 0.1);
//         }
//
//         // if keyboard_input.just_pressed(KeyCode::Return) {
//         //     *current_animation = (*current_animation + 1) % animations.0.len();
//         //     player
//         //         .play(animations.0[*current_animation].clone_weak())
//         //         .repeat();
//         // }
//     }
// }

// fn loading_progress(counter: Res<ProgressCounter>) {
//     // Get the overall loading progress
//     let progress = counter.progress();
//
//     // we can use `progress.done` and `progress.total`,
//     // or convert it to a float:
//     let float_progress: f32 = progress.into();
//     log::info!("loading_progress {:?}", float_progress);
// }
//
// fn spawn_player(commands: &mut Commands) {
//     commands
//         .spawn(SceneBundle {
//             // scene: assets.load("models/stadium/scene.gltf#Scene0"),
//             // scene: assets.load("models/firework/scene.gltf#Scene0"),
//             scene: assets.load("models/char_null/scene.gltf#Scene0"),
//             // scene: assets.load("models/wolf/scene.gltf#Scene0"),
//             // scene: assets.load("models/balloon_chest/scene.gltf#Scene0"),
//             // scene: assets.load("models/ballooned_yoshi/scene.gltf#Scene0"),
//             transform: Transform {
//                 scale: Vec3::splat(0.1),
//                 ..default()
//             },
//             ..default()
//         })
//         .insert(AnimationPlayer::default());
//
// }
