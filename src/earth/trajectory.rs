//! Rendering logic of trajectories such as rivers and roads
use std::ops::RangeInclusive;
use bevy::math::{Vec2, Vec3};

use super::{assets::AssetCache, mesh_builder::MeshBuilder};


/// Returns the 4 corner points of the rectangle of the provided trajectory segment
fn get_rectangle_points(begin: Vec3, end: Vec3, width: f32) -> (Vec3, Vec3, Vec3, Vec3) {
    let direction = (end - begin).normalize();
    let perpendicular = Vec3::new(-direction.z, 0., direction.x);
    let half_width = width / 2.;

    let start_right = begin + perpendicular * half_width; //
    let start_left = begin - perpendicular * half_width;
    let end_left = end - perpendicular * half_width;
    let end_right = end + perpendicular * half_width;

    (start_right, start_left, end_left, end_right)
}

/// Generates a smoother trajectory by connecting the start point of the new segment
/// with the end point of the previous segment.
pub fn generate_trajectory_old(
    trajectory: Vec<Vec2>,
    width: f32,
    uv_range: (RangeInclusive<f32>, RangeInclusive<f32>),
    mesh_builder: &mut MeshBuilder,
    _asset_cache: &AssetCache,
) {
    let uv = Vec2::new(*uv_range.0.start(), *uv_range.1.start());
    
    for i in 0..trajectory.len() - 1 {
        let (x1, y1) = (trajectory[i].x as f32, trajectory[i].y as f32);
        let (x2, y2) = (trajectory[i+1].x as f32, trajectory[i+1].y as f32);

        let (start_right, start_left, mut end_left, mut end_right) = get_rectangle_points(
            Vec3::new(x1 as f32, 0.018, y1 as f32),
            Vec3::new(x2 as f32, 0.018, y2 as f32),
            width,
        );

        if i < trajectory.len() - 2 {
            // Replace end with the next point
            let (x3, y3) = (trajectory[i+2].x as f32, trajectory[i+2].y as f32);

            let (next_start_right, next_start_left, _discard, _discard_2) = get_rectangle_points(
                Vec3::new(x2 as f32, 0.018, y2 as f32),
                Vec3::new(x3 as f32, 0.018, y3 as f32),
                width,
            );

            // To make the road smooth, we need to adjust the end points
            // Can possibly directly use the next start points
            end_left = next_start_left;
            end_right = next_start_right;
        }

        mesh_builder.add_quad([start_right, end_right, end_left, start_left], [uv, uv, uv, uv]);
    }
}


pub fn generate_trajectory(
    trajectory: Vec<Vec2>,
    width: f32,
    y: f32,
    uv_range: (RangeInclusive<f32>, RangeInclusive<f32>),
    mesh_builder: &mut MeshBuilder,
    _asset_cache: &AssetCache,
) {
    let uv = Vec2::new(*uv_range.0.start(), *uv_range.1.start());
    
    let width = width;
    let mut last_end_left: Vec3 = Vec3::NAN;
    let mut last_end_right: Vec3 = Vec3::NAN;

    for i in 0..trajectory.len() - 1 {
        let (x1, y1) = (trajectory[i].x as f32, trajectory[i].y as f32);
        let (x2, y2) = (trajectory[i+1].x as f32, trajectory[i+1].y as f32);

        let (mut start_right, mut start_left, mut end_left, mut end_right) = get_rectangle_points(
            Vec3::new(x1 as f32, y, y1 as f32),
            Vec3::new(x2 as f32, y, y2 as f32),
            width,
        );

        if i > 0 {
            // Replace start points with the last end points
            start_left = last_end_left;
            start_right = last_end_right;
        }

        if i < trajectory.len() - 2 {
            // Replace end points with more smoother variant
            let (x3, y3) = (trajectory[i+2].x as f32, trajectory[i+2].y as f32);

            let (next_start_right, next_start_left, _discard, _discard_2) = get_rectangle_points(
                Vec3::new(x2 as f32, y, y2 as f32),
                Vec3::new(x3 as f32, y, y3 as f32),
                width,
            );

            // To make the trajectory smooth, we need to adjust the end points
            // Can possibly directly use the next start points
            end_left = (next_start_left + end_left) / 2.0;  // Take average of the two to make it more smooth
            end_right = (next_start_right + end_right) / 2.0;  
        }
        last_end_left = end_left;
        last_end_right = end_right;

        mesh_builder.add_quad([start_right, end_right, end_left, start_left], [uv, uv, uv, uv]);
    }
}

// pub fn convert_trajectory_to_bundle(
//     meshes: &mut ResMut<Assets<Mesh>>,
//     materials: &mut ResMut<Assets<StandardMaterial>>,
//     data: (Mesh, Color),
// ) -> impl Bundle {
//     let (mesh, color) = data;
//     PbrBundle {
//         mesh: meshes.add(mesh),
//         material: materials.add(color),
//         ..default()
//     }
// }
