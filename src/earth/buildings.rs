use super::assets::AssetCache;
use super::GLOBAL_SCALE_FACTOR;
use crate::data::building_type::{
    get_random_range_building, BuildingLandUseType, BuildingType, PartialBuilding, RoofShape,
};
use crate::data::geography::{BuildingFeature, GeoLocation, LandUseFeature, Offset};
use crate::earth::mesh_builder::MeshBuilder;
use crate::earth::simplification::simplify_polygon;
use wasm_bindgen::prelude::*;

use bevy::prelude::*;
use rand::Rng;
use std::collections::hash_map::HashMap;
use std::str::FromStr;

const DIST_UNIT_PER_LEVEL: f32 = 0.04 * GLOBAL_SCALE_FACTOR;
const THRESHOLD_SIMPLIFICATION: f32 = 0.00001 * GLOBAL_SCALE_FACTOR * GLOBAL_SCALE_FACTOR;
const THRESHOLD_APARTMENT_BASE_SIZE: f32 = 0.75 * GLOBAL_SCALE_FACTOR; // Residential buildings with a base smaller than this are considered houses, else apartments
const THRESHOLD_SMALL_BUILDING: f32 = 0.05 * GLOBAL_SCALE_FACTOR; // Buildings with a base smaller than this are considered small and thus can only have 1 level
const THRESHOLD_NON_RESIDENTIAL_BUILDING: f32 = 0.25 * GLOBAL_SCALE_FACTOR; // Non-residential buildings are capped for their height depending on this, so that small based buildings aren't enormous

// What tags OSM uses for buildings
const TAG_BUILDING_TYPE: &str = "building";
const TAG_BUILDING_LEVELS: &str = "building:levels";
const TAG_BUILDING_ROOF_SHAPE: &str = "roof:shape";
const TAG_BUILDING_ROOF_LEVELS: &str = "roof:levels";

pub fn create_building_data(
    node_locations: &HashMap<u64, GeoLocation>,
    building_features: &HashMap<u64, BuildingFeature>,
    landuse_features: &HashMap<u64, LandUseFeature>,
    asset_cache: &AssetCache,
    offset: &Offset,
) -> Mesh {
    let mut rng = rand::thread_rng();
    let mut partial_buildings = Vec::new();

    // Go over all land use areas related to buildings
    let building_related_landuse = get_building_land_use(landuse_features, node_locations, offset);

    // loop over building data and create partial buildings
    let mut _total_vertices = 0;
    let mut _total_vertices_simplified = 0;
    for (&id, building) in building_features {
        // Create base from nodes and fix ordering of base vertices
        let base_locations = match create_building_base(node_locations, &building, offset) {
            Some(base) => base,
            None => continue,
        };
        let base = polygon_counterclockwise_ordering(base_locations);
        _total_vertices += base.len();
        let base = simplify_polygon(base, THRESHOLD_SIMPLIFICATION);
        _total_vertices_simplified += base.len();

        // Get all the data
        let partial_building = get_partial_building_from_tags(id, building, base);

        // Add to list of partial buildings
        partial_buildings.push(partial_building);
    }

    // Go over all partial buildings and check if they are inside a land use area
    for partial_building in &mut partial_buildings {
        if partial_building.inside_area != BuildingLandUseType::Unknown
            || partial_building.building_type.is_some()
        {
            continue; // We do not case about the landuse type if we already know the building type or land use type
        }

        // Check if we are inside the polygon for any of the land use areas
        let building_point = partial_building.base[0];
        for (landuse_polygon, landuse_type) in &building_related_landuse {
            if point_in_polygon_check(landuse_polygon, building_point) {
                partial_building.inside_area = *landuse_type;
                break;
            }
        }
    }

    // Print some statistics
    // println!(
    //     "Building total vertices: {}, and after simplification: {}",
    //     total_vertices, total_vertices_simplified
    // );

    // loop over partial buildings, fill in gaps in data and create the entities
    let mut builder = MeshBuilder::new();
    for partial_building in partial_buildings {
        // Fill in gaps in data
        // let mut interpolated = false;

        // Fill in building type if necessary
        let building_type = if partial_building.building_type.is_none() {
            // We use the land use type as a proxy for the building type
            // interpolated = true;
            match partial_building.inside_area {
                BuildingLandUseType::Residential => {
                    // If the base is small we assume it is a house, otherwise an apartment building
                    if calculate_polygon_area(&partial_building.base)
                        < THRESHOLD_APARTMENT_BASE_SIZE
                    {
                        BuildingType::House
                    } else {
                        BuildingType::Apartments
                    }
                }
                BuildingLandUseType::Commercial => BuildingType::Commercial,
                BuildingLandUseType::Industrial => BuildingType::Industrial,
                BuildingLandUseType::Education => BuildingType::School,
                _ => BuildingType::Other,
            }
        } else {
            partial_building.building_type.unwrap_throw()
        };

        // Fill in number of levels, based on building type
        let number_of_levels = if partial_building.levels.is_none() {
            // interpolated = true;
            let area = calculate_polygon_area(&partial_building.base);
            if area < THRESHOLD_SMALL_BUILDING {
                1
            } else {
                let mut rng = rand::thread_rng();
                let (min, max) = get_random_range_building(building_type);

                let mut levels = rng.gen_range(min..=max);

                // Cap industrial building height based on their size
                if building_type == BuildingType::Industrial
                    || building_type == BuildingType::Commercial
                    || building_type == BuildingType::Retail
                    || building_type == BuildingType::Warehouse
                    || building_type == BuildingType::Supermarket
                    || building_type == BuildingType::Office
                    || building_type == BuildingType::Transportation
                    || building_type == BuildingType::Civic
                {
                    let cap = (area / THRESHOLD_NON_RESIDENTIAL_BUILDING).floor() as i32 + 1;
                    levels = levels.min(cap);
                }

                levels
            }
        } else {
            partial_building.levels.unwrap_throw()
        };

        let height = DIST_UNIT_PER_LEVEL
            * (number_of_levels + partial_building.roof_levels.clone().unwrap_or(0)) as f32;

        let index = rng.gen_range(0..asset_cache.get_building_texture_count());
        let uv_range = asset_cache.get_wall_uv(index);
        let uv = Vec2::new(*uv_range.0.start(), *uv_range.1.start());

        // Generate mesh from base
        builder.add_prism_from_path(&partial_building.base, height, uv);
    }

    builder.into_mesh()
}

// Note: this is kinda of an awful way to do this, better would be some precomputed spatial data structure with fast queries
fn point_in_polygon_check(polygon: &Vec<Vec2>, point: Vec2) -> bool {
    let mut inside = false;
    
    for i in 0..polygon.len() {
        let j = (i + 1) % polygon.len();
        if (polygon[i].y > point.y) != (polygon[j].y > point.y)
            && point.x
                < (polygon[j].x - polygon[i].x) * (point.y - polygon[i].y)
                    / (polygon[j].y - polygon[i].y)
                    + polygon[i].x
        {
            inside = !inside;
        }
    }
    inside
}

/// Filters all landuse areas to ones useful for identifying buildings, sorts them by size and simplifies them.
fn get_building_land_use(
    landuse_features: &HashMap<u64, LandUseFeature>,
    node_locations: &HashMap<u64, GeoLocation>,
    offset: &Offset,
) -> Vec<(Vec<Vec2>, BuildingLandUseType)> {
    let mut building_related_landuse = Vec::new();

    // Go over all land use areas
    for landuse_feature in landuse_features.values() {
        let landuse_type = match landuse_feature.tags.get("landuse") {
            Some(s) => BuildingLandUseType::from_str(s).unwrap_or(BuildingLandUseType::Unknown),
            None => BuildingLandUseType::Unknown,
        };

        // Check if the land use area is related to a building
        if landuse_type != BuildingLandUseType::Unknown {
            // Turn the land use area into a polygon
            let polygon: Vec<Vec2> = landuse_feature
                .nodes
                .iter()
                .filter_map(|node_id| {
                    node_locations
                        .get(node_id)
                        .map(|node| node.project(&offset))
                })
                .collect();
            // Simplify the polygon
            let polygon = simplify_polygon(polygon, THRESHOLD_SIMPLIFICATION);

            building_related_landuse.push((polygon, landuse_type));
        }
    }

    // Order land use by number of vertices in polygon as a proxy for size, from largest to smallest
    building_related_landuse.sort_by(|a, b| b.0.len().cmp(&a.0.len()));

    building_related_landuse
}

/// Makes sure the vertices of a polygon are in counter-clockwise order.
fn polygon_counterclockwise_ordering(mut base: Vec<Vec2>) -> Vec<Vec2> {
    // Check if the base is in clockwise order
    let mut sum = 0.0;
    for i in 0..base.len() {
        let p1 = base[i];
        let p2 = base[(i + 1) % base.len()];
        sum += (p2.x - p1.x) * (p2.y + p1.y);
    }

    // Reverse if necessary
    if sum > 0.0 {
        base.reverse();
    }

    base
}

/// Base needs to be in counter-clockwise order.
fn get_partial_building_from_tags(
    id: u64,
    building: &BuildingFeature,
    base: Vec<Vec2>,
) -> PartialBuilding {
    let building_type = match building.tags.get(TAG_BUILDING_TYPE) {
        Some(s) => match BuildingType::from_str(s) {
            Ok(building_type) => Some(building_type),
            Err(_) => None,
        },
        None => None,
    };

    PartialBuilding {
        id,
        // Get building type
        building_type: building_type,
        // Get building height
        levels: match building.tags.get(TAG_BUILDING_LEVELS) {
            Some(s) => match s.parse() {
                Ok(i) => Some(i),
                Err(_) => None,
            },
            None => None,
        },
        // Fill in base from before
        base,
        // Get roof type
        roof_shape: match building.tags.get(TAG_BUILDING_ROOF_SHAPE) {
            Some(s) => RoofShape::from_str(s).ok(),
            None => None,
        },
        // Get roof levels
        roof_levels: match building.tags.get(TAG_BUILDING_ROOF_LEVELS) {
            Some(s) => match s.parse() {
                Ok(i) => Some(i),
                Err(_) => None,
            },
            None => None,
        },
        inside_area: if building_type.is_some() {
            BuildingLandUseType::NOTNECESSARY
        } else {
            BuildingLandUseType::Unknown
        },
    }
}

/// Creates a building base from a list of nodes. Returns None if any of the nodes are not found or the number of nodes is less than 3
fn create_building_base(
    node_locations: &HashMap<u64, GeoLocation>,
    building: &BuildingFeature,
    offset: &Offset,
) -> Option<Vec<Vec2>> {
    if building.nodes.len() < 3 {
        return None;
    }
    building
        .nodes
        .iter()
        .map(|node_id| Some(node_locations.get(node_id)?.project(&offset)))
        .collect()
}

#[derive(Component)]
pub struct Building {
    pub building_type: BuildingType,
    pub interpolated: bool, // Is true when the building contains any interpolated data
}

fn calculate_polygon_area(polygon: &Vec<Vec2>) -> f32 {
    let mut area = 0.0;
    for i in 0..polygon.len() {
        let j = (i + 1) % polygon.len();
        area += polygon[i].x * polygon[j].y;
        area -= polygon[j].x * polygon[i].y;
    }
    area / 2.0
}
