use core::constants::*;
/// # TODO
/// - goalie jump or bobble or shield (probably shield?)
/// - asset loading
/// - render unowned balls names (nice to have?)
///
/// #Later Learning
/// - render to texture for UI https://github.com/hallettj/redstone-designer/blob/main/src/block_picker.rs
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
use protocol::{
    messages::{Auth, KeyCommand},
    primitives::Scores,
};

use std::f32::consts::*;

// use bevy::input::InputPlugin;
// use bevy::log::LogPlugin;
use bevy::prelude::*;
// use bevy_asset_loader::prelude::*;
use bevy_rapier3d::prelude::*;

use bevy_debug_text_overlay::{screen_print, OverlayPlugin};
// use bevy_inspector_egui::prelude::*;
// use bevy_inspector_egui::quick::WorldInspectorPlugin;

use naia_bevy_client::{
    transport::webrtc, Client, ClientConfig, CommandHistory, Plugin as NaiaClientPlugin,
    ReceiveEvents,
};

#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash, Default, States)]
pub enum AppState {
    #[default]
    NameInput,
    Selection,
    InGame,
}

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

// A unit struct to help identify the FPS UI component, since there may be many Text components
#[derive(Component)]
struct UI;

#[cfg(target_arch = "wasm32")]
fn get_userinfo() -> anyhow::Result<(String, String)> {
    use anyhow::anyhow;
    use js_sys::Reflect;
    use wasm_bindgen::JsValue;

    let window = web_sys::window().ok_or_else(|| anyhow!("Can't access Window object"))?;
    let player_name = Reflect::get(&window, &JsValue::from_str("player_name"))
        .map_err(|_| anyhow!("no player_name"))?
        .as_string()
        .ok_or_else(|| anyhow!("can't convert to string"))?;
    let player_color = Reflect::get(&window, &JsValue::from_str("player_color"))
        .map_err(|_| anyhow!("no player_name"))?
        .as_string()
        .ok_or_else(|| anyhow!("can't convert to string"))?;
    Ok((player_name, player_color))
}

#[cfg(not(target_arch = "wasm32"))]
fn get_userinfo() -> anyhow::Result<(String, String)> {
    Ok(("armorous0772".to_owned(), "blue".to_owned()))
}

pub fn init(
    mut commands: Commands,
    mut client: Client,
    mut meshes: ResMut<Assets<Mesh>>,
    mut images: ResMut<Assets<Image>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    audio: Res<Audio>,
    // audio_sinks: Res<Assets<AudioSink>>,
    asset_server: Res<AssetServer>, // mut materials: ResMut<Assets<ColorMaterial>>,
) {
    let music = asset_server.load("sounds/smw-world.mp3");
    audio.play_with_settings(
        music,
        PlaybackSettings {
            repeat: true,
            volume: 0.2,
            ..Default::default()
        },
    );

    info!("PowerBaby Connecting");
    let (player_name, player_color) =
        get_userinfo().unwrap_or_else(|_| ("Denis".to_owned(), "blue".to_owned()));
    info!("Player: {}, Color: {}", player_name, player_color);

    client.auth(Auth::from((player_name.clone(), player_color.clone())));
    let socket = webrtc::Socket::new(protocol::SERVER_HANDSHAKE_URL, client.socket_config());
    client.connect(socket);

    let mut blue_name = Default::default();
    let mut pink_name = Default::default();
    let mut blue_score_entity = Entity::PLACEHOLDER;
    let mut pink_score_entity = Entity::PLACEHOLDER;
    if &player_color == "blue" {
        blue_name = player_name;
    } else {
        pink_name = player_name;
    }

    // Setup Global Resource
    let mut global = Global::default();
    commands
        .spawn(NodeBundle {
            style: Style {
                size: Size::new(Val::Percent(100.0), Val::Percent(100.0)),
                justify_content: JustifyContent::SpaceBetween,
                flex_direction: FlexDirection::Row,
                margin: UiRect {
                    left: Val::Px(20.0),
                    right: Val::Px(20.0),
                    top: Val::Px(24.0),
                    ..Default::default()
                },
                ..default()
            },
            ..default()
        })
        .insert(UI)
        .with_children(|c| {
            // LEFT SIDE BLUE
            c.spawn(NodeBundle {
                style: Style {
                    align_items: AlignItems::Center,
                    flex_direction: FlexDirection::Column,
                    gap: Size {
                        height: Val::Px(10.0),
                        ..default()
                    },
                    ..default()
                },
                ..default()
            })
            .with_children(|c| {
                c.spawn(TextBundle {
                    style: Style { ..default() },
                    text: Text::from_sections([TextSection::new(
                        "Total",
                        TextStyle {
                            font: asset_server.load("fonts/color-mario.ttf"),
                            font_size: 21.0,
                            color: Color::hex("#89CFF0").unwrap(),
                        },
                    )]),
                    ..default()
                });

                global.total_blue_entity = Some(
                    c.spawn(TextBundle {
                        style: Style {
                            margin: UiRect {
                                bottom: Val::Px(15.0),
                                ..Default::default()
                            },
                            ..default()
                        },
                        text: Text::from_sections([TextSection::new(
                            "0",
                            TextStyle {
                                font: asset_server.load("fonts/color-mario.ttf"),
                                font_size: 21.0,
                                color: Color::hex("#89CFF0").unwrap(),
                            },
                        )]),
                        ..default()
                    })
                    .id(),
                );

                c.spawn(TextBundle {
                    style: Style { ..default() },
                    text: Text::from_sections([TextSection::new(
                        blue_name,
                        TextStyle {
                            font: asset_server.load("fonts/color-mario.ttf"),
                            font_size: 12.0,
                            color: Color::hex("#89CFF0").unwrap(),
                        },
                    )]),
                    ..default()
                });

                blue_score_entity = c
                    .spawn(TextBundle {
                        style: Style { ..default() },
                        text: Text::from_sections([TextSection::new(
                            "",
                            TextStyle {
                                font: asset_server.load("fonts/color-mario.ttf"),
                                font_size: 10.0,
                                color: Color::hex("#89CFF0").unwrap(),
                            },
                        )]),
                        ..default()
                    })
                    .id();
            });

            c.spawn(NodeBundle {
                style: Style {
                    align_items: AlignItems::Center,
                    flex_direction: FlexDirection::Column,
                    gap: Size {
                        height: Val::Px(10.0),
                        ..default()
                    },
                    ..default()
                },
                ..default()
            })
            .with_children(|c| {
                c.spawn(TextBundle {
                    style: Style { ..default() },
                    text: Text::from_sections([TextSection::new(
                        "Total",
                        TextStyle {
                            font: asset_server.load("fonts/color-mario.ttf"),
                            font_size: 24.0,
                            color: Color::hex("#FFB7CE").unwrap(),
                        },
                    )]),
                    ..default()
                });

                global.total_pink_entity = Some(
                    c.spawn(TextBundle {
                        style: Style {
                            margin: UiRect {
                                bottom: Val::Px(20.0),
                                ..Default::default()
                            },
                            ..default()
                        },
                        text: Text::from_sections([TextSection::new(
                            "0",
                            TextStyle {
                                font: asset_server.load("fonts/color-mario.ttf"),
                                font_size: 21.0,
                                color: Color::hex("#FFB7CE").unwrap(),
                            },
                        )]),
                        ..default()
                    })
                    .id(),
                );

                c.spawn(TextBundle {
                    style: Style { ..default() },
                    text: Text::from_sections([TextSection::new(
                        pink_name,
                        TextStyle {
                            font: asset_server.load("fonts/color-mario.ttf"),
                            font_size: 12.0,
                            color: Color::hex("#FFB7CE").unwrap(),
                        },
                    )]),
                    ..default()
                });

                pink_score_entity = c
                    .spawn(TextBundle {
                        style: Style { ..default() },
                        text: Text::from_sections([TextSection::new(
                            "",
                            TextStyle {
                                font: asset_server.load("fonts/color-mario.ttf"),
                                font_size: 24.0,
                                color: Color::hex("#FFB7CE").unwrap(),
                            },
                        )]),
                        ..default()
                    })
                    .id();
            });
        });

    if &player_color == "blue" {
        global.own_score_entity = Some(blue_score_entity);
    } else {
        global.own_score_entity = Some(pink_score_entity);
    }

    // // Test ui
    // commands
    //     .spawn(ButtonBundle {
    //         style: Style {
    //             justify_content: JustifyContent::Center,
    //             align_items: AlignItems::Center,
    //             position_type: PositionType::Absolute,
    //             position: UiRect {
    //                 left: Val::Px(50.0),
    //                 right: Val::Px(50.0),
    //                 top: Val::Auto,
    //                 bottom: Val::Px(150.0),
    //             },
    //             ..default()
    //         },
    //         ..default()
    //     })
    //     .with_children(|b| {
    //         b.spawn(
    //             TextBundle::from_section(
    //                 "Test Button",
    //                 TextStyle {
    //                     font: asset_server.load("fonts/FiraSans-Bold.ttf"),
    //                     font_size: 30.0,
    //                     color: Color::BLACK,
    //                 },
    //             )
    //             .with_text_alignment(TextAlignment::Center),
    //         );
    //     });

    // let ball_texture_handle = images.add(core::debug::uv_texture());
    let ball_texture_handle = asset_server.load("images/yoshiegg.png");
    let ball_texture_pink_handle = asset_server.load("images/yoshiegg_pink.png");
    let ball_mesh_handle = meshes.add(
        shape::Icosphere {
            radius: BALL_RADIUS,
            ..Default::default()
        }
        .try_into()
        .unwrap(),
    );
    global.goalie_scene = asset_server.load("models/yoshi/scene.gltf#Scene0");
    // let goalie_mesh_handle = meshes.add(
    //     shape::Capsule {
    //         radius: GOALIE_RADIUS,
    //         rings: 0,
    //         depth: GOALIE_HEIGHT,
    //         latitudes: 16,
    //         longitudes: 32,
    //         uv_profile: shape::CapsuleUvProfile::Aspect,
    //     }
    //     .into(),
    // );
    global.ball_texture = ball_texture_handle.clone();
    global.ball_texture_pink = ball_texture_pink_handle.clone();
    global.ball_mesh = ball_mesh_handle;
    // global.goalie_mesh = goalie_mesh_handle;
    global.debug_material = materials.add(StandardMaterial {
        base_color_texture: Some(ball_texture_handle),
        alpha_mode: AlphaMode::Blend,
        ..default()
    });
    global.kick_sound = asset_server.load("sounds/yoshi_throws.wav");
    global.frame_deny_sound = asset_server.load("sounds/fireball.wav");
    global.goalie_deny_sound = asset_server.load("sounds/yoshi_bounce.wav");
    global.pink_goal_sound = asset_server.load("sounds/coin_pink.wav");
    global.blue_goal_sound = asset_server.load("sounds/coin_blue.wav");

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
        transform: KICK_CAM.looking_at(KICK_CAM_LOOK, Vec3::Y),
        ..Default::default()
    },));

    commands.spawn(DirectionalLightBundle {
        directional_light: DirectionalLight {
            // shadows_enabled: false,
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
                // SceneBundle {
                //     scene: asset_server.load("models/field/scene.gltf#Scene0"),
                //     transform: Transform::from_xyz(0.0, GROUND_HEIGHT, 0.0),
                //     ..Default::default()
                // },
                PbrBundle {
                    mesh: meshes.add(shape::Plane::from_size(GROUND_SIZE * 2.0).into()),
                    material: ground_material,
                    transform: Transform::from_xyz(0.0, GROUND_HEIGHT, 0.0),
                    ..default()
                },
                Collider::cuboid(GROUND_SIZE, 0.0, GROUND_SIZE),
                RigidBody::KinematicPositionBased,
                Friction::new(100.0),
            ))
            .id(),
    );

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

#[derive(Clone)]
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

    //Scoring
    pub my_score: u32,
    pub total_pink: u32,
    pub total_blue: u32,
    pub own_score_entity: Option<Entity>,
    pub total_pink_entity: Option<Entity>,
    pub total_blue_entity: Option<Entity>,

    pub owned_entity: Option<OwnedEntity>,
    pub ground_entity: Option<Entity>,
    pub queued_command: Option<KeyCommand>,
    pub command_history: CommandHistory<KeyCommand>,

    pub debug_material: Handle<StandardMaterial>,
    pub ground_material: Handle<StandardMaterial>,
    pub goal_material: Handle<StandardMaterial>,
    pub ball_texture: Handle<Image>,
    pub ball_texture_pink: Handle<Image>,
    pub ball_mesh: Handle<Mesh>,
    pub goalie_scene: Handle<Scene>,
    // pub goalie_mesh: Handle<Mesh>,
    pub kick_sound: Handle<AudioSource>,
    pub frame_deny_sound: Handle<AudioSource>,
    pub goalie_deny_sound: Handle<AudioSource>,
    pub pink_goal_sound: Handle<AudioSource>,
    pub blue_goal_sound: Handle<AudioSource>,
}

mod components {
    use bevy::prelude::*;

    #[derive(Component)]
    pub struct Predicted;

    #[derive(Component)]
    pub struct Confirmed;

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
            // self.next_w = next_w;
        }

        pub(crate) fn interpolate(&mut self, interpolation: f32) {
            if self.interp >= 1.0 || interpolation == 0.0 {
                return;
            }
            if self.interp < interpolation {
                self.interp = interpolation;

                // let lerped = slerp(
                //     Quat::from_xyzw(self.last_x, self.last_y, self.last_z, self.last_w),
                //     Quat::from_xyzw(self.next_x, self.next_y, self.next_z, self.next_w),
                //     self.interp,
                // );
                //
                // self.interp_x = lerped.x;
                // self.interp_y = lerped.y;
                // self.interp_z = lerped.z;
                // self.interp_w = lerped.w;

                self.interp_x = self.last_x + (self.next_x - self.last_x) * self.interp;
                self.interp_y = self.last_y + (self.next_y - self.last_y) * self.interp;
                self.interp_z = self.last_z + (self.next_z - self.last_z) * self.interp;
                self.interp_w = self.last_w + (self.next_w - self.last_w) * self.interp;
            }
        }
    }

    // fn lerp(a: Quat, b: Quat, s: f32) -> Quat {
    //     use std::ops::{Add, Mul, Sub};
    //     let start = a;
    //     let dot = start.dot(b);
    //     let bias = if dot >= 0.0 { 1.0 } else { -1.0 };
    //     let interpolated = a.add(b.mul(bias).sub(start).mul(s));
    //     interpolated
    // }
    //
    // pub fn slerp(a: Quat, mut b: Quat, s: f32) -> Quat {
    //     use std::ops::{Add, Mul};
    //     const DOT_THRESHOLD: f32 = 0.9995;
    //
    //     // Note that a rotation can be represented by two quaternions: `q` and
    //     // `-q`. The slerp path between `q` and `end` will be different from the
    //     // path between `-q` and `end`. One path will take the long way around and
    //     // one will take the short way. In order to correct for this, the `dot`
    //     // product between `self` and `end` should be positive. If the `dot`
    //     // product is negative, slerp between `self` and `-end`.
    //     let mut dot = a.dot(b);
    //     if dot < 0.0 {
    //         b = -b;
    //         dot = -dot;
    //     }
    //
    //     if dot > DOT_THRESHOLD {
    //         // assumes lerp returns a normalized quaternion
    //         lerp(a, b, s)
    //     } else {
    //         let theta = dot.acos_approx();
    //
    //         let scale1 = (theta * (1.0 - s)).sin();
    //         let scale2 = (theta * s).sin();
    //         let theta_sin = theta.sin();
    //
    //         a.mul(scale1).add(b.mul(scale2)).mul(theta_sin.recip())
    //     }
    // }
    //
    // pub(crate) trait FloatEx {
    //     /// Returns a very close approximation of `self.clamp(-1.0, 1.0).acos()`.
    //     fn acos_approx(self) -> Self;
    // }
    //
    // impl FloatEx for f32 {
    //     #[inline(always)]
    //     fn acos_approx(self) -> Self {
    //         // Based on https://github.com/microsoft/DirectXMath `XMScalarAcos`
    //         // Clamp input to [-1,1].
    //         let nonnegative = self >= 0.0;
    //         let x = self.abs();
    //         let mut omx = 1.0 - x;
    //         if omx < 0.0 {
    //             omx = 0.0;
    //         }
    //         let root = omx.sqrt();
    //
    //         // 7-degree minimax approximation
    //         #[allow(clippy::approx_constant)]
    //         let mut result = ((((((-0.001_262_491_1 * x + 0.006_670_09) * x - 0.017_088_126)
    //             * x
    //             + 0.030_891_88)
    //             * x
    //             - 0.050_174_303)
    //             * x
    //             + 0.088_978_99)
    //             * x
    //             - 0.214_598_8)
    //             * x
    //             + 1.570_796_3;
    //         result *= root;
    //
    //         // acos(x) = pi - acos(-x) when x < 0
    //         if nonnegative {
    //             result
    //         } else {
    //             std::f32::consts::PI - result
    //         }
    //     }
    // }
}

mod events {
    use super::components::{Confirmed, InterpPos, InterpRot, Predicted};
    use super::{Global, OwnedEntity};
    use crate::AppState;
    use core::{components::Ball, constants::*};

    use protocol::{
        channels::{EntityAssignmentChannel, GameStateChannel, PlayerCommandChannel},
        components::{EntityKind, EntityKindValue, Player, RepPhysics, UpdateWith},
        messages::{EntityAssignment, EventKind, KeyCommand, PlayerEvent, TotalScoreState},
        primitives::PlayColor,
    };

    use bevy::{pbr::NotShadowReceiver, prelude::*};
    use bevy_rapier3d::prelude::*;

    //sequence_greater_than, Tick
    use naia_bevy_client::{
        events::{
            ClientTickEvent, ConnectEvent, DespawnEntityEvent, DisconnectEvent,
            InsertComponentEvents, MessageEvents, RejectEvent, RemoveComponentEvents,
            SpawnEntityEvent, UpdateComponentEvents,
        },
        Client, CommandsExt,
    };

    pub fn connect_events(
        // mut commands: Commands,
        global: Res<Global>,
        client: Client,
        mut next_state: ResMut<NextState<AppState>>,
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
        mut materials: ResMut<Assets<StandardMaterial>>,
        audio: Res<Audio>,

        mut commands: Commands,
        mut event_reader: EventReader<MessageEvents>,

        ball_query: Query<(&RepPhysics, &Handle<StandardMaterial>)>,
        mut text_query: Query<&mut Text>,
    ) {
        for events in event_reader.iter() {
            for message in events.read::<EntityAssignmentChannel, EntityAssignment>() {
                handle_entity_assignment(
                    &mut global,
                    &client,
                    &mut commands,
                    &ball_query,
                    &mut materials,
                    message,
                );
            }
            for message in events.read::<GameStateChannel, PlayerEvent>() {
                handle_player_event(&mut global, &client, &audio, &mut text_query, message);
            }
            for message in events.read::<GameStateChannel, TotalScoreState>() {
                global.total_pink += message.pink;
                global.total_blue += message.blue;
                text_query
                    .get_mut(global.total_pink_entity.unwrap())
                    .unwrap()
                    .sections[0]
                    .value = global.total_pink.to_string();
                text_query
                    .get_mut(global.total_blue_entity.unwrap())
                    .unwrap()
                    .sections[0]
                    .value = global.total_blue.to_string();
                log::info!(
                    "TOTAL SCORE SNAPSHOT: Pink {}, Blue {}",
                    message.pink,
                    message.blue,
                );
            }
        }
    }

    fn handle_entity_assignment(
        global: &mut ResMut<Global>,
        client: &Client,
        commands: &mut Commands,
        ball_query: &Query<(&RepPhysics, &Handle<StandardMaterial>)>,
        materials: &mut ResMut<Assets<StandardMaterial>>,
        message: EntityAssignment,
    ) {
        let assign = message.assign;
        let entity = message.entity.get(client).unwrap();
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
                            mesh: global.ball_mesh.clone(),
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
                        Collider::ball(BALL_RADIUS),
                        // ColliderMassProperties::Mass(BALL_MASS),
                        // Velocity::zero(),
                        // Friction::new(5.0),
                        // ExternalForce::default(),
                        // ExternalImpulse::default(),
                        // GravityScale::default(),
                        // Damping {
                        //     linear_damping: 1.0,
                        //     angular_damping: 2.0,
                        // },
                        // Restitution {
                        //     coefficient: 1.0,
                        //     combine_rule: CoefficientCombineRule::Average,
                        // },
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

    fn handle_player_event(
        global: &mut ResMut<Global>,
        client: &Client,
        audio: &Res<Audio>,
        text_query: &mut Query<&mut Text>,
        message: PlayerEvent,
    ) {
        match message.kind {
            EventKind::BlueScored => {
                if let (Some(owned), Some(entity)) =
                    (global.owned_entity.clone(), message.entity.get(client))
                {
                    if owned.confirmed == entity {
                        global.my_score += 1;
                        text_query
                            .get_mut(global.own_score_entity.unwrap())
                            .unwrap()
                            .sections[0]
                            .value = global.my_score.to_string();
                    }
                }
                global.total_blue += 1;
                text_query
                    .get_mut(global.total_blue_entity.unwrap())
                    .unwrap()
                    .sections[0]
                    .value = global.total_blue.to_string();
                audio.play(global.blue_goal_sound.clone());
            }
            EventKind::PinkScored => {
                if let (Some(owned), Some(entity)) =
                    (global.owned_entity.clone(), message.entity.get(client))
                {
                    if owned.confirmed == entity {
                        global.my_score += 1;
                        text_query
                            .get_mut(global.own_score_entity.unwrap())
                            .unwrap()
                            .sections[0]
                            .value = global.my_score.to_string();
                    }
                }
                global.total_pink += 1;
                text_query
                    .get_mut(global.total_pink_entity.unwrap())
                    .unwrap()
                    .sections[0]
                    .value = global.total_pink.to_string();
                audio.play(global.pink_goal_sound.clone());
            }
            //do nothing for now
            EventKind::ScoreSnapshot(_n) => {}
            EventKind::Kicked => {
                audio.play(global.kick_sound.clone());
            }
            EventKind::DeniedGoalie => {
                audio.play(global.goalie_deny_sound.clone());
            }
            EventKind::DeniedFrame => {
                audio.play(global.frame_deny_sound.clone());
            }
        }
    }

    // pub fn spawn_entity_events(mut event_reader: EventReader<SpawnEntityEvent>) {
    //     for SpawnEntityEvent(_entity) in event_reader.iter() {
    //         info!("spawned entity");
    //     }
    // }
    //
    // pub fn despawn_entity_events(mut event_reader: EventReader<DespawnEntityEvent>) {
    //     for DespawnEntityEvent(_entity) in event_reader.iter() {
    //         info!("despawned entity");
    //     }
    // }

    #[derive(Component)]
    pub struct GoalieRemote;

    #[derive(Component)]
    pub struct Goalie;

    pub fn insert_component_events(
        global: Res<Global>,
        mut meshes: ResMut<Assets<Mesh>>,
        mut materials: ResMut<Assets<StandardMaterial>>,

        mut commands: Commands,
        mut event_reader: EventReader<InsertComponentEvents>,

        kind_query: Query<&EntityKind>,
        rep_physics_query: Query<&RepPhysics>,
        player_query: Query<&Player>,
    ) {
        for events in event_reader.iter() {
            for entity in events.read::<Player>() {
                let player = player_query.get(entity).unwrap();
                let rep_physics = rep_physics_query.get(entity).unwrap();

                let texture = if let PlayColor::Blue = *player.color {
                    global.ball_texture.clone()
                } else {
                    global.ball_texture_pink.clone()
                };

                commands.entity(entity).insert((
                    PbrBundle {
                        mesh: global.ball_mesh.clone(),
                        material: materials.add(StandardMaterial {
                            base_color: Color::rgba(1.0, 1.0, 1.0, 0.2),
                            base_color_texture: Some(texture),
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
                    NotShadowReceiver,
                ));
            }

            for entity in events.read::<RepPhysics>() {
                let kind = kind_query.get(entity).unwrap();
                let rep_physics = rep_physics_query.get(entity).unwrap();

                log::info!("entity: {:?}", *kind.value);
                match *kind.value {
                    EntityKindValue::Goalie => {
                        commands.entity(entity).insert((
                            Name::new("Goalie"),
                            Transform {
                                translation: Vec3::new(
                                    // 0.0, 0.0, 0.0,
                                    *rep_physics.translation_x,
                                    GOALIE_START.y,
                                    GOALIE_START.z,
                                ),
                                ..Default::default()
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
                            GoalieRemote,
                        ));
                        commands.spawn((
                            SceneBundle {
                                scene: global.goalie_scene.clone(),
                                transform: Transform {
                                    translation: Vec3::new(
                                        *rep_physics.translation_x,
                                        GROUND_HEIGHT,
                                        GOALIE_START.z,
                                    ),
                                    rotation: Quat::from_rotation_y(4.71239),
                                    ..Default::default()
                                },
                                ..Default::default()
                            },
                            Goalie,
                        ));
                    }
                    EntityKindValue::Ball => {
                        commands.entity(entity).insert((
                            Name::new("Ball"),
                            Ball::default(),
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
                }
            }
        }
    }

    pub fn update_component_events(
        // mut global: ResMut<Global>,
        mut _event_reader: EventReader<UpdateComponentEvents>,
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
        for _events in event_reader.iter() {
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
        let Some(_predicted_entity) = global
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
    use super::components::Confirmed;
    use super::Global;
    use core::constants::*;
    use protocol::messages::KeyCommand;

    use bevy::{prelude::*, render::camera::RenderTarget, window::PrimaryWindow};
    use bevy_rapier3d::prelude::*;
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
                *camera_transform = BIRDS_EYE_CAM.looking_at(BIRDS_EYE_CAM_LOOK, Vec3::Y);
            } else {
                *camera_transform = KICK_CAM.looking_at(KICK_CAM_LOOK, Vec3::Y);
            }
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
    }

    pub fn ball(
        mut global: ResMut<Global>,
        client: Client,
        keyboard_input: Res<Input<KeyCode>>,
        mouse_buttons: ResMut<Input<MouseButton>>,
        touches: Res<Touches>,
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
            // just shoot straight if spacebar is pushed.  Maybe change this to click anywhere but
            // the ball?
            (true, true, _) => Some((
                Vec3::new(0.015694855, -0.011672409, 0.9998087).into(),
                Vec3::new(0.0017264052, 0.0070980787, 42.109978).into(),
            )),
            (true, false, true) => {
                capture_ball_click(camera_query, window, rapier_context, &global.ground_entity)
                    .map(|(ray_normal, ray_point)| (ray_normal.into(), ray_point.into()))
            }
            (true, false, false) => {
                if let Some(pos) = touches.first_pressed_position() {
                    capture_ball_touch(
                        camera_query,
                        window,
                        pos,
                        rapier_context,
                        &global.ground_entity,
                    )
                    .map(|(ray_normal, ray_point)| (ray_normal.into(), ray_point.into()))
                } else {
                    None
                }
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
}

pub mod sync {
    use super::components::{Confirmed, InterpPos, InterpRot};
    use protocol::components::{RepPhysics, UpdateWith};

    use bevy::prelude::*;
    use bevy_rapier3d::prelude::*;
    use naia_bevy_client::Client;

    pub fn serverside_entities(
        client: Client,
        mut query: Query<
            // (&RepPhysics, &mut InterpPos, &mut Transform),
            (
                &RepPhysics,
                &mut InterpPos,
                &mut InterpRot,
                &mut Transform,
                &EntityKind,
            ),
            (With<Confirmed>, Without<super::events::Goalie>),
        >,
        mut query_yoshi: Query<&mut Transform, With<super::events::Goalie>>,
    ) {
        for (rep_physics, mut interp, mut interp_rot, mut transform, kind) in query.iter_mut() {
            // for (rep_physics, mut interp, mut transform) in query.iter_mut() {
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
            // transform.rotation.w = interp_rot.interp_w;
            transform.rotation.w = *rep_physics.rotation_w;

            if let EntityKindValue::Goalie = *kind.value {
                if let Ok(mut yoshi) = query_yoshi.get_single_mut() {
                    yoshi.translation.x = transform.translation.x;
                }
            }
        }
    }

    //sync rapier systems:
    //It should run after rapier systems and before naia’s ‘ReceiveEvents’ set. And in the
    //client the exactly opposite. Naia events, then sync system and then rapier systems.

    use protocol::components::{EntityKind, EntityKindValue};

    // in client after naia 'ReceiveEvents' systems before physics systems
    pub fn sync_from_naia_to_rapier(
        mut query: Query<(&mut Transform, &mut Velocity, &RepPhysics, &EntityKind)>,
        mut query_yoshi: Query<&mut Transform, With<super::events::Goalie>>,
    ) {
        for (mut transform, mut velocity, physics_properties, kind) in query.iter_mut() {
            transform.update_with(physics_properties);
            log::info!("YO {:?}", *kind.value);
            if let EntityKindValue::Goalie = *kind.value {
                log::info!("YO");
                if let Ok(mut yoshi) = query_yoshi.get_single_mut() {
                    yoshi.translation.x = transform.translation.x;
                }
            }
            // velocity.update_with(physics_properties);
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
        .add_state::<AppState>()
        .add_plugins(
            DefaultPlugins.set(WindowPlugin {
                primary_window: Some(Window {
                    title: "Power, Baby! (ONLINE)".into(),
                    resolution: bevy::window::WindowResolution::new(480.0, 720.0)
                        .with_scale_factor_override(8.0),
                    // resolution: (480.0, 720.0).into(),
                    // resolution: (480.0, 720.0).into(),
                    // scale_factor: 2.0,
                    // mode: window::WindowMode::SizedFullscreen,
                    // resolution: (1920., 1080.).into(),
                    // fit_canvas_to_parent: true,
                    prevent_default_event_handling: false,
                    canvas: Some("canvas".to_owned()),
                    ..default()
                }),
                ..default()
            }),
        )
        // .add_plugin(WorldInspectorPlugin::new())
        .add_plugin(OverlayPlugin {
            font_size: 32.0,
            ..default()
        })
        .add_plugin(RapierPhysicsPlugin::<NoUserData>::default())
        .add_plugin(NaiaClientPlugin::new(
            ClientConfig::default(),
            protocol::protocol(),
        ))
        // Background Color
        // .insert_resource(ClearColor(Color::hex("#87CEEB").unwrap()))
        .add_startup_system(init)
        .add_systems(
            (
                events::connect_events,
                events::disconnect_events,
                events::reject_events,
                // events::spawn_entity_events,
                // events::despawn_entity_events,
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
                // input::camera,
                input::ball,
                // button_handler,
                // name_input,
                sync::serverside_entities,
                debug_overlay,
            )
                .chain()
                .in_set(MainLoop),
        )
        // .configure_set(ReceiveEvents.run_if(in_state(AppState::InGame)))
        .configure_set(Tick.after(ReceiveEvents))
        .configure_set(MainLoop.after(Tick))
        // .configure_set(Tick.after(ReceiveEvents).run_if(in_state(AppState::InGame)))
        // .configure_set(MainLoop.after(Tick).run_if(in_state(AppState::InGame)))
        .add_system(bevy::window::close_on_esc)
        // Run App
        .run();
}

// fn name_input(
//     mut global: ResMut<Global>,
//     mut char_evr: EventReader<ReceivedCharacter>,
//     keys: Res<Input<KeyCode>>,
//
//     mut query: Query<(Entity, &mut Children), With<UI>>,
//     mut text_child: Query<&mut Text>,
// ) {
//     let (entity, children) = query.get_single_mut().unwrap();
//     for &child in children.iter() {
//         // get the health of each child unit
//         let mut text = text_child.get_mut(child).unwrap();
//         let section = &mut text.sections[1].value;
//
//         for ev in char_evr.iter() {
//             if section.len() < 32 {
//                 if !ev.char.is_whitespace() {
//                     section.push(ev.char);
//                 }
//             }
//         }
//
//         if keys.just_pressed(KeyCode::Delete) {
//             if section.len() > 0 {
//                 section.pop();
//             }
//         }
//
//         if keys.just_pressed(KeyCode::Return) {
//             // log::info!("Return!");
//             // let window = web_sys::window().expect("no global `window` exists");
//             // window
//             //     .location()
//             //     .set_href("https://power-baby.com")
//             //     .expect("location exists");
//             // window.href();
//
//             // println!("Text input: {}", *string);
//             // string.clear();
//         }
//     }
//     // for (parent, mut text) in query.iter_mut() {
//     // log::info!("wowoww");
//     // let section = &mut text.sections[0].value;
//     // }
// }

// fn user_selected(mut next_state: ResMut<NextState<AppState>>, keyboard_input: Res<Input<KeyCode>>) {
//     if keyboard_input.just_pressed(KeyCode::Space) {
//         next_state.set(AppState::InGame);
//     }
// }

// fn button_handler(
//     mut interaction_query: Query<
//         (&Interaction, &mut BackgroundColor),
//         (Changed<Interaction>, With<Button>),
//     >,
//     mut window_query: Query<&mut Window, With<window::PrimaryWindow>>,
// ) {
//     for (interaction, mut color) in &mut interaction_query {
//         match *interaction {
//             Interaction::Clicked => {
//                 let w = window_query.get_single_mut().unwrap();
//                 log::info!("WINDOW {}x{} ({})", w.width(), w.height(), w.scale_factor());
//                 *color = Color::BLUE.into();
//             }
//             Interaction::Hovered => {
//                 *color = Color::GRAY.into();
//             }
//             Interaction::None => {
//                 *color = Color::WHITE.into();
//             }
//         }
//         match *interaction {
//             Interaction::Clicked => {}
//             _ => (),
//         }
//     }
// }
