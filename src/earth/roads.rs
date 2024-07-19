use bevy::prelude::*;
use std::collections::hash_map::HashMap;
use std::str::FromStr;
use wasm_bindgen::prelude::*;

use crate::data::geography::{GeoLocation, Offset, RoadFeature};
use crate::data::road_type::{
    road_type_to_default_lanes, road_type_to_width, RoadType, road_type_to_random_height
};
use crate::earth::assets::AssetCache;
use crate::earth::mesh_builder::MeshBuilder;
use super::trajectory::generate_trajectory;
use super::GLOBAL_SCALE_FACTOR;

/// Creates a building base from a list of nodes. 
/// Returns None if any of the nodes are not found or the number of nodes is less than 3
fn create_road_base(
    node_locations: &HashMap<u64, GeoLocation>,
    road: &RoadFeature,
    offset: &Offset
) -> Option<Vec<Vec2>> {
    if road.nodes.len() < 2 {
        return None;
    }
    road
        .nodes
        .iter()
        .map(|node_id| {
            Some(node_locations.get(node_id)?.project(&offset))
        })
        .collect()
}

/// Converts the road features in the given chunks to data that can be drawn in
/// the world (meshes and materials).
pub fn create_road_data(
    node_locations: &HashMap<u64, GeoLocation>,
    road_features: &HashMap<u64, RoadFeature>,
    asset_cache: &AssetCache,
    offset: &Offset
) -> Mesh {
    let mut mesh_builder = MeshBuilder::new();
    for (_, road_feature) in road_features {
        let road: Option<Vec<Vec2>> = create_road_base(node_locations, road_feature, offset);

        if road.is_none() {
            continue;
        }
        let road: Vec<Vec2> = road.unwrap_throw();
        // println!("Road: {:?}", road);

        // Convert to road type
        let road_type = RoadType::from_str(&road_feature.tags["highway"])
            .unwrap_or(RoadType::NotCovered);
        
        // Ridiculous high value will be fixed
        let lanes = road_feature.tags.get("lanes")
            .and_then(|s| s.parse::<u32>().ok())
            .unwrap_or(road_type_to_default_lanes(&road_type));  // Ridiculous high value will be fixed
        
        let width = road_type_to_width(&road_type) * 0.01 * lanes as f32 * GLOBAL_SCALE_FACTOR;
        let uv_range = asset_cache.get_road_uv(road_type);
        let y = road_type_to_random_height(&road_type); 

        generate_trajectory(
            road, 
            width,             
            y,  // Make road appear under buildings to avoid z-fighting
            uv_range,
            &mut mesh_builder, 
            asset_cache,
        );
    }
    mesh_builder.into_mesh()
}
