pub mod systems {
    use crate::components::*;
    use crate::constants::*;

    use bevy::prelude::*;
    use bevy_rapier3d::prelude::*;
    use bevy_turborand::prelude::*;

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

    pub fn goalie(
        time: Res<Time>,
        mut goalie_query: Query<(&mut Transform, &mut GoalieBehavior), Without<ExternalImpulse>>,
        ball_query: Query<(&Transform, &Ball), With<ExternalImpulse>>,
        mut rand: ResMut<GlobalRng>,
    ) {
        const SPEED: &[f32] = &[1.5, 2.0, 3.0, 4.0];
        const ACTION_TIMES: &[f32] = &[0.1, 0.01, 0.15, 0.05];
        const DIRECTION: &[f32] = &[0.0, 1.0, -1.0];

        let (mut goalie_transform, mut goalie) = goalie_query.get_single_mut().unwrap();

        // filter by balls that are 2.0 units or less away from the goalie
        // Then sort them by the MIN z axis to find the closest ball
        // to the goalie.  We then move the goalie towards the ball (if it exists).  Otherwise
        // move normally if nothing was found.
        let new_goalie_pos = ball_query
            .iter()
            .filter(|(q, _)| {
                let dist = q.translation.z - goalie_transform.translation.z;
                dist < 6.0 && dist > 0.0
            })
            .min_by(|a, b| a.0.translation.z.partial_cmp(&b.0.translation.z).unwrap())
            .map(|q| {
                let to_x = q.0.translation.x - goalie_transform.translation.x;
                let speed = goalie.speed * 3.0;
                goalie_transform.translation.x + to_x * speed * TIME_STEP
            })
            .unwrap_or_else(|| {
                goalie_transform.translation.x + goalie.direction * goalie.speed * TIME_STEP
            });
        goalie_transform.translation.x =
            new_goalie_pos.clamp(GOALIE_PATROL_MIN_X, GOALIE_PATROL_MAX_X);

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
}

pub mod components {
    use bevy::prelude::*;

    #[derive(Component, Default)]
    pub struct GoalieBehavior {
        pub seconds_left: f32,
        pub direction: f32,
        pub speed: f32,
    }

    #[derive(Component, Default)]
    pub struct Ball {
        pub shot: bool,
        pub scored: bool,
        pub force_reset: bool,
        pub shot_elapsed: f32,
    }
}

pub mod constants {
    use std::f32::consts::*;

    use bevy::prelude::*;

    // Defines the amount of time that should elapse between each step.  This is essentially
    // a "target" of 60 updates per second
    // const TIME_STEP: f32 = 1.0 / 60.0;
    pub const TIME_STEP: f32 = 1.0 / 60.0;

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
    pub const BALL_SHOT_WAIT_TIME: f32 = 2.0;

    //Camera
    pub const BIRDS_EYE_CAM: Transform = Transform::from_xyz(0.0, 17.7, 37.7);
    pub const BIRDS_EYE_CAM_LOOK: Vec3 = Vec3::new(0.0, -500.0, 0.0);
    pub const KICK_CAM: Transform = Transform::from_xyz(0.0, 1.0, 43.8);
    pub const KICK_CAM_LOOK: Vec3 = Vec3::new(0.0, -7.0, 0.0);
}

pub mod debug {
    use bevy::render::texture::Image;

    /// Creates a colorful test pattern
    pub fn uv_texture() -> Image {
        use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};
        const TEXTURE_SIZE: usize = 8;

        let mut palette: [u8; 32] = [
            255, 102, 159, 255, 255, 159, 102, 255, 236, 255, 102, 255, 121, 255, 102, 255, 102,
            255, 198, 255, 102, 198, 255, 255, 121, 102, 255, 255, 236, 102, 255, 255,
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
}
