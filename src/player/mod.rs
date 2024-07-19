use bevy::prelude::*;

use std::f32::consts::PI;

use crate::earth::GLOBAL_SCALE_FACTOR;

#[derive(Component, Debug)]
pub struct Player {
    /// In world units per second.
    pub translation_speed: f32,

    /// In radians per pixel that the mouse was moved.
    pub rotation_speed: f32,
}

#[derive(Debug, Event)]
pub struct PlayerMoveEvent {
    pub translation: Vec3,
    pub rotation: Vec2,
    pub do_panning: bool,
}

/// Spawns a player.
pub fn setup_player(mut commands: Commands) {
    commands.spawn((
        Player {
            translation_speed: 2.0 * GLOBAL_SCALE_FACTOR,
            rotation_speed: 0.002 * PI,
        },
        Camera3dBundle {
            transform: Transform::from_xyz(0.0, 10.0 * GLOBAL_SCALE_FACTOR, 0.0)
                .looking_at(Vec3::ZERO, Vec3::Y),
            ..default()
        },
    ));
}

pub fn update_player(
    mut query: Query<(&Player, &mut Transform)>,
    mut move_events: EventReader<PlayerMoveEvent>,
    time: Res<Time>,
) {
    for event in move_events.read() {
        for (player, mut transform) in &mut query {
            // Multiply the translation by the height factor
            let height_factor = f32::max(1.0, f32::powf(transform.translation.y / 100.0, 0.8)); // Exponent at the end to make speed increase not exponential the higher you go

            let (yaw, pitch, _) = transform.rotation.to_euler(EulerRot::YXZ);
            let mut new_yaw = yaw - event.rotation.x * player.rotation_speed;
            let new_pitch =
                (pitch - event.rotation.y * player.rotation_speed).clamp(-0.5 * PI, 0.5 * PI);

            if event.do_panning {
                // should pan the camera a bit to the right
                // add yaw to right
                new_yaw = yaw - 0.0005;
            }

            let forward_x = new_yaw.sin();
            let forward_z = new_yaw.cos();
            let diff_x = event.translation.x * forward_z + event.translation.z * forward_x;
            let diff_z = -event.translation.x * forward_x + event.translation.z * forward_z;
            let diff = Vec3::new(diff_x, event.translation.y, diff_z);

            transform.translation +=
                player.translation_speed * time.delta_seconds() * diff * height_factor;
            transform.rotation = Quat::from_euler(EulerRot::YXZ, new_yaw, new_pitch, 0.0);

            if transform.translation.y < 1.5 {
                transform.translation.y = 1.5;
            }
        }
    }
}
