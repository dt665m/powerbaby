use core::{
    components::{Ball, GoalieBehavior},
    constants::*,
    debug::uv_texture,
    systems::{goalie, magnus_effect},
};

///Guides
/// https://bevyengine.org/learn/book/migration-guides/0.9-0.10/#states
/// https://bevyengine.org/news/bevy-0-10/
///3D Examples:
/// https://github.com/alexichepura/bevy_garage/blob/main/src/car.rs
use std::f32::consts::*;

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
// use bevy_inspector_egui::prelude::*;
// use bevy_inspector_egui::quick::{ResourceInspectorPlugin, WorldInspectorPlugin};
use bevy_inspector_egui::quick::WorldInspectorPlugin;

// //#TODO upstream this to bevy.  Since it's FnMut without Clone on upstream, it cannot be used in
// //'distributive_run_if'
// pub fn in_state<S: States>(state: S) -> impl Fn(Res<State<S>>) -> bool + Clone {
//     move |current_state: Res<State<S>>| current_state.0 == state
// }

// #[derive(Resource, Reflect, InspectorOptions, Default)]
// #[reflect(Resource, InspectorOptions)]
#[derive(Resource, Default)]
pub struct Game {
    ball_entity: Option<Entity>,
    goalie_entity: Option<Entity>,
    ground_entity: Option<Entity>,
    // #[inspector(min = 0.0, max = 1.0)]
    power: f32,
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
    Shoot { ray_normal: Vec3, ray_point: Vec3 },
}

fn main() {
    game_app();
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
        // .add_plugin(WorldInspectorPlugin::new())
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
        // .add_system(bevy::window::close_on_esc)
        .run();
}

//#TODO move server's physics systems into core and remove this giant game_logic system
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
                ray_normal,
                ray_point,
            } => {
                log::info!("SHOOT {:?} - {:?}", ray_normal, ray_point);
                let ray_normal = Vec3::new(ray_normal.x, ray_normal.y - 0.8, ray_normal.z);
                let kick_force = Vec3::new(-2.0, -3.0, -13.0);
                let impulse = ray_normal * kick_force;
                // let impulse_camera = camera_rotation.normalize() * force.neg();
                *ext_i = ExternalImpulse::at_point(impulse, *ray_point, transform.translation);
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

pub fn debug_overlay(time: Res<Time>) {
    let current_time = time.elapsed_seconds_f64();
    let at_interval = |t: f64| current_time % t < time.delta_seconds_f64();
    if at_interval(0.1) {
        let last_fps = 1.0 / time.delta_seconds();
        screen_print!(col: Color::CYAN, "fps: {last_fps:.0}");
    }
}

pub fn controls(
    game: ResMut<Game>,
    keyboard_input: Res<Input<KeyCode>>,
    mouse_buttons: ResMut<Input<MouseButton>>,
    touches: Res<Touches>,
    ball_query: Query<&Transform, With<ExternalImpulse>>,

    mut controller_events: EventWriter<ControllerEvent>,
    camera_query: Query<(&Camera, &Transform, &GlobalTransform)>,
    window: Query<&Window, With<PrimaryWindow>>,
    rapier_context: Res<RapierContext>,
) {
    if game.shot {
        return;
    }

    let Ok(ball_transform) = ball_query.get(game.ball_entity.unwrap()) else {
            return;
        };

    //#HACK only let the ball get kicked if it has already settled down from the spawn
    //point.  When prediction is implemented, We use the local predicted ball and not the
    //confirmed ball because of lag compensation.  IE, we expect the confirmed ball to be
    //already dropped by the time we actually see the updates at the client.  The predicted
    //ball is spawned after the ball entity assignment.  For now use the Confirmed Entity
    let can_shoot = ball_transform.translation.y < 0.01;

    if keyboard_input.just_pressed(KeyCode::Q) {
        controller_events.send(ControllerEvent::Reset);
    }

    let shoot = match (
        can_shoot,
        keyboard_input.pressed(KeyCode::Space),
        mouse_buttons.pressed(MouseButton::Left),
    ) {
        // just shoot straight if spacebar is pushed.  Maybe change this to click anywhere but
        // the ball?
        (true, true, _) => Some((
            Vec3::new(0.015694855, -0.011672409, 0.9998087).into(),
            Vec3::new(0.0017264052, 0.0070980787, 42.109978).into(),
        )),
        (true, false, true) => {
            capture_ball_click(camera_query, window, rapier_context, &game.ground_entity)
                .map(|(ray_normal, ray_point)| (ray_normal.into(), ray_point.into()))
        }
        (true, false, false) => {
            if let Some(pos) = touches.first_pressed_position() {
                capture_ball_touch(
                    camera_query,
                    window,
                    pos,
                    rapier_context,
                    &game.ground_entity,
                )
                .map(|(ray_normal, ray_point)| (ray_normal.into(), ray_point.into()))
            } else {
                None
            }
        }
        _ => None,
    };

    if let Some((ray_normal, ray_point)) = shoot {
        controller_events.send(ControllerEvent::Shoot {
            ray_normal,
            ray_point,
        });
    }
}

fn capture_ball_touch(
    camera_query: Query<(&Camera, &Transform, &GlobalTransform)>,
    window: Query<&Window, With<PrimaryWindow>>,
    touch_pos: Vec2,
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
    let Some(ray) = camera.viewport_to_world(camera_global_transform, touch_to_cursor_pos(touch_pos, &window)) else {
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
            .and_then(|cursor| {
                camera.viewport_to_world(camera_global_transform, cursor)})
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

fn touch_to_cursor_pos(touch_pos: Vec2, window: &Window) -> Vec2 {
    Vec2::new(touch_pos.x, window.resolution.height() - touch_pos.y)
}

pub fn staging(
    mut game: ResMut<Game>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut images: ResMut<Assets<Image>>,
    mut materials: ResMut<Assets<StandardMaterial>>,

    // null_char: Res<NullCharacter>,
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
    game.ground_entity = Some(
        commands
            .spawn((
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
            ))
            .id(),
    );

    /*
     * Create the cubes
     */
    let color = 0;
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
        base_color_texture: Some(images.add(uv_texture())),
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

// #[derive(Resource)]
// pub struct NullCharacter {
//     pub scene: Handle<Scene>,
//     pub run: Handle<AnimationClip>,
//     pub idle: Handle<AnimationClip>,
//     pub walk: Handle<AnimationClip>,
// }

pub fn load_assets(
    _assets: Res<AssetServer>,
    mut _commands: Commands,
    mut _loading: ResMut<AssetsLoading>,
) {
    // log::info!("load_assets");
    // let nc = NullCharacter {
    //     scene: assets.load("models/char_null/scene.gltf#Scene0"),
    //     run: assets.load("models/char_null/scene.gltf#Animation0"),
    //     idle: assets.load("models/char_null/scene.gltf#Animation1"),
    //     walk: assets.load("models/char_null/scene.gltf#Animation2"),
    // };
    // loading.add(&nc.scene);
    // loading.add(&nc.run);
    // loading.add(&nc.idle);
    // loading.add(&nc.walk);
    //
    // commands.insert_resource(nc);
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
