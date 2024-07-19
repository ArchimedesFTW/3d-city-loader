use std::collections::HashMap;
use bevy::prelude::*;

// Import randon


use crate::data::geography::{GeoLocation, LakeFeature, Offset};
use crate::earth::{GeoFeature, GLOBAL_SCALE_FACTOR};
use crate::earth::mesh_builder::MeshBuilder;
use crate::earth::simplification::simplify_polygon;

const LAKE_SIMPLIFICATION_THRESHOLD: f32 = 0.00001 * GLOBAL_SCALE_FACTOR * GLOBAL_SCALE_FACTOR;

fn generate_lake(
    commands: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    materials: &mut ResMut<Assets<StandardMaterial>>,
    node_locations: &HashMap<u64, GeoLocation>,
    lake: &LakeFeature,
    offset: &Offset
) {

    let area: Vec<Vec2> = lake
        .nodes
        .iter()
        .filter_map(|node_id| node_locations.get(node_id).map(|node| node.project(offset)))
        .collect();

    let area_simplified = simplify_polygon(area, LAKE_SIMPLIFICATION_THRESHOLD);
    let points: Vec<_> = area_simplified.iter()
        .map(|vec2| geo::Point::new(vec2.x as f64, vec2.y as f64))
        .collect();
    let polygon = geo::Polygon::new(points.into(), vec![]);

    let mut mesh_builder = MeshBuilder::new();
    let uv = Vec2::new(0.0, 0.0);  // TODO tweak?
    mesh_builder.add_polygon_xz(&polygon, 0.009, uv);  // Up normal
    let mesh = mesh_builder.into_mesh();

    let lake_material: Handle<StandardMaterial> = materials.add(StandardMaterial {
        base_color: Color::BLUE,
        cull_mode: None,
        ..default()
    });
    
    commands.spawn(PbrBundle {
        mesh: meshes.add(mesh),
        material: lake_material,
        ..Default::default()
    }).insert(GeoFeature { id: 0 });
}

// Define or import the generate_terrain function here
pub fn update_lake(
    commands: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    materials: &mut ResMut<Assets<StandardMaterial>>,
    node_locations: &HashMap<u64, GeoLocation>,
    lake_features: &HashMap<u64, LakeFeature>,
    offset: &Offset
) {
    for (_id, lake) in lake_features.iter() {

        generate_lake(commands, meshes, materials, node_locations, lake, &offset);
    }
}