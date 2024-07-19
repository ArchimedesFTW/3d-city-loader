use std::collections::HashMap;
use bevy::prelude::*;
use crate::data::geography::{GeoLocation, Offset, RiverFeature};
use super::{assets::AssetCache, mesh_builder::MeshBuilder, trajectory::generate_trajectory};
use wasm_bindgen::prelude::*;

fn get_river_trajectory(
    node_locations: &HashMap<u64, GeoLocation>,
    river: &RiverFeature,
    offset: &Offset
) -> Option<Vec<Vec2>> {
    if river.nodes.len() < 2 {
        return None;
    }
    river
        .nodes
        .iter()
        .map(|node_id| {
            Some(node_locations.get(node_id)?.project(&offset))
        })
        .collect()
}

pub fn create_river_data(
    node_locations: &HashMap<u64, GeoLocation>,
    river_features: &HashMap<u64, RiverFeature>,
    asset_cache: &AssetCache,
    offset: &Offset
) -> Mesh {
    let mut mesh_builder = MeshBuilder::new();
    for (_, river_feature) in river_features {
        let river: Option<Vec<Vec2>> = get_river_trajectory(node_locations, river_feature, offset);

        if river.is_none() {
            continue;
        }
        let river: Vec<Vec2> = river.unwrap_throw();

        let width = determine_width(&river_feature);
        let uv_range = asset_cache.get_river_uv();

        generate_trajectory(
            river, 
            width, 
            0.005,  // Make river appear under roads and lakes to avoid z-fighting
            uv_range,
            &mut mesh_builder, 
            asset_cache,
        );
    }
    mesh_builder.into_mesh()
}

/// Determine the width of the river based on the tags, return in meters
fn determine_width(river: &RiverFeature) -> f32 {
    // Check if CEMT tag is present and use that
    if river.tags.contains_key("CEMT") {
        let class = river.tags.get("CEMT").unwrap_throw();
        return get_river_width(class);
    }

    // Get waterway tag, make switch case
    let waterway = river.tags.get("waterway").unwrap_or(&String::from("nan")).to_string();
    let mut width = match waterway.as_str() {  // Derived by looking at different places in the OSM data and here: https://wiki.openstreetmap.org/wiki/Key:waterway   
        "river" =>  2.5,
        "stream" => 0.8,
        "canal" => 2.0,
        "ditch" => 0.6,
        _ => 0.3,
    };

    if river.tags.contains_key("boat") {
        let boat = river.tags.get("boat").unwrap_throw();
        let boat_width = match boat.as_str() {
            "yes" => 2.0,
            "no" => 0.0,
            _ => 0.0,
        };
        width = width + boat_width;
    }

    if river.tags.contains_key("maxspeed") {
        let speed_tag = river.tags.get("maxspeed").unwrap_throw();
        // Convert to_factor to multiply with width
        let speed = speed_tag.parse::<f32>().unwrap_or(0.0);
        width = width * speed / 5.0;
    }

    width
}


// Create CEMT mapping to width https://en.wikipedia.org/wiki/Classification_of_European_Inland_Waterways
fn get_river_width(class: &str) -> f32 {
    let class = class.to_lowercase();
    
    // Map class to breadth in meters
    let width = match class.as_str() {
        "ra" => 2.0,
        "rb" => 3.0,
        "rc" => 4.0,
        "rd" => 4.0,
        "i" => 4.85,
        "ii" => 7.0, 
        "iii" => 8.2, 
        "iv" => 9.5,
        "va" => 11.4,  // Maas has this classification and is approx 200 meter wide
        "vb" => 11.4, 
        "via" => 22.8,
        "vib" => 22.8, // Albert Canal has this classification and is approx 100 meter wide
        "vic" => 26.8, 
        "vii" => 33.0, // 3x3 convoy
        _ => 2.0, 
    };

    // Boats should be able to pass each other with lee-way and safety
    // So we should have a width of 2x the width of the boat
    // Closer to 3x for the larger classes
    let width = width * 2.35;

    // Apply small exponential function because the width is not linear with allowed boat size
    f32::powf(width, 1.05)
}
