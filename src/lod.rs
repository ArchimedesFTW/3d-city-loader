use bevy::ecs::component::Component;
use bevy::prelude::*;
use wasm_bindgen::prelude::*;

use crate::earth::GLOBAL_SCALE_FACTOR;
use crate::player;

/// A level of detail system.
/// Entities with the LOD component will have their high quality mesh replaced with lower quality ones or entirely removed if they are too far away.

/// Squared distance at which agents do not render
const DEFAULT_REMOVE_DIST: f32 = 10.0;
pub const DEFAULT_REMOVE_DISTANCE_SQUARED: f32 =
    (DEFAULT_REMOVE_DIST * GLOBAL_SCALE_FACTOR) * (DEFAULT_REMOVE_DIST * GLOBAL_SCALE_FACTOR);

/// Squared distance low quality agents are rendered
const DEFAULT_LOD_DIST: f32 = 5.0;
pub const DEFAULT_LOD_DISTANCE_SQUARED: f32 =
    (DEFAULT_LOD_DIST * GLOBAL_SCALE_FACTOR) * (DEFAULT_LOD_DIST * GLOBAL_SCALE_FACTOR);

#[derive(Component, Debug)]
pub struct LOD {
    /// The squared distance at which the mesh will be removed, squared distance used for performance
    pub remove_distance_squared: f32,
    /// The squared distance at which the entity will be replaced with a lower quality mesh
    pub lod_distance_distance_squared: f32,

    /// High quality mesh
    pub high_quality_mesh: Handle<Mesh>,

    /// High quality material
    pub high_quality_material: Handle<StandardMaterial>,

    /// Low quality mesh
    pub low_quality_mesh: Handle<Mesh>,

    /// Low quality material
    pub low_quality_material: Handle<StandardMaterial>,
}

/// Updates LOD of entities.
pub fn lod_system(
    mut lod_query: Query<(&LOD, &mut Handle<Mesh>, &mut Handle<StandardMaterial>, &Transform)>,
    player_query: Query<(&player::Player, &Transform)>,
) {
    // Get player position
    if player_query.iter().next().is_none() {
        return;
    }
    let player_transform = player_query.iter().next().unwrap_throw().1;

    // Update LOD
    let empty_mesh: Handle<Mesh> = Handle::default();
    for (lod, mut mesh, mut material, transform) in lod_query.iter_mut() {
        let distance_sq = Vec3::distance_squared(
            transform.translation,
            player_transform.translation,
        );

        if distance_sq > lod.remove_distance_squared {
            if *mesh != empty_mesh {
                *mesh = empty_mesh.clone();
            }
        } else if distance_sq > lod.lod_distance_distance_squared {
            if *mesh != lod.low_quality_mesh {
                *mesh = lod.low_quality_mesh.clone();
                *material = lod.low_quality_material.clone();
            }
        } else {
            if *mesh != lod.high_quality_mesh {
                *mesh = lod.high_quality_mesh.clone();
                *material = lod.high_quality_material.clone();
            }
        }
    }
}