//! Defines internal in-memory geographic data structures and routines for
//! converting external formats to them.
//! 
//! These routines are "pure", meaning that they do not make any calls to
//! external APIs or things like that.

use crate::common::{DataFormat, AppError};
use crate::earth::GLOBAL_SCALE_FACTOR;
use wasm_bindgen::prelude::*;

use bevy::ecs::system::Resource;
use bevy::math::Vec2;

use serde_json::{Map, Number, Value as JsonValue};

use std::collections::hash_map::HashMap;
use std::f64::consts::PI;

/// A collection of geographic data.
#[derive(Debug)]
pub struct GeoData {
    pub node_locations: HashMap<u64, GeoLocation>,
    pub chunks: HashMap<ChunkIndex, Chunk>,
}

impl GeoData {
    /// Returns whether there are 0 features and nodes.
    pub fn is_empty(&self) -> bool {
        self.node_locations.is_empty() &&
        self.chunks.is_empty()
    }
}

/// The nodes and features that lie within a chunk.
#[derive(Debug, Default)]
pub struct Chunk {
    pub nodes: HashMap<u64, GeoNode>,
    pub building_features: HashMap<u64, BuildingFeature>,
    pub road_features: HashMap<u64, RoadFeature>,
    pub land_use_features: HashMap<u64, LandUseFeature>,
    pub lake_features: HashMap<u64, LakeFeature>,
    pub river_features: HashMap<u64, RiverFeature>,
}

/// An identifier/index for a chunk.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct ChunkIndex {
    pub x: i64,
    pub z: i64,
}

impl ChunkIndex {
    /// Returns the index of the chunk that the given 2D world coordinates lie
    /// inside of.
    pub fn from_vec2(coords: Vec2) -> Self {
        ChunkIndex {
            x: (coords.x / CHUNK_SIZE).floor() as i64,
            z: (coords.y / CHUNK_SIZE).floor() as i64,
        }
    }
}
const CHUNK_SIZE: f32 = 8.0 * GLOBAL_SCALE_FACTOR as f32;

#[derive(Clone, Debug, Copy, Resource)]
pub struct Offset {
    pub x: f64,
    pub y: f64,
}

impl Default for Offset {
    fn default() -> Self {
        Offset {
            x: f64::NEG_INFINITY,
            y: f64::NEG_INFINITY,
        }
    }
}

/// A single point on the surface of the earth.
#[derive(Clone, Debug)]
pub struct GeoLocation {
    /// West to east.
    pub longitude: f64,
    /// North to south.
    pub latitude: f64,
}

const LATITUDAL_SCALE_FACTOR: f64 = 64000.0 * (GLOBAL_SCALE_FACTOR as f64);
const LONGITUDAL_SCALE_FACTOR: f64 = 64000.0 * (GLOBAL_SCALE_FACTOR as f64);

impl GeoLocation {
    /// Convert from geographic coordinates to XZ coordinates on a plane.
    /// 
    /// One longitudal degree is approximately 110.6 km at the Equator at sea
    /// level, but this strongly varies depending on the location on earth
    /// (hence a rather complicated calculation is required). One latitudal
    /// degree is approximately 111.3 km at the Equator at sea level.
    pub fn project(&self, offset: &Offset) -> Vec2 {
        // not entirely sure if this is the best projection function
        let x = ((self.longitude + 180.0) / 360.0 - offset.x)* LONGITUDAL_SCALE_FACTOR;
        let lat_radians = (self.latitude) / 180.0 * PI;
        let y = ((1.0 - lat_radians.tan().asinh() / PI) / 2.0 - offset.y)* LATITUDAL_SCALE_FACTOR;
        Vec2::new(x as f32, y as f32)
    }

    /// Perform the same projection as `project`, but without scaling the result.
    /// This is to calculate the enables accurate relative positioning of points for recentering
    pub fn project_no_scale(&self) -> (f64, f64) {
        // not entirely sure if this is the best projection function
        let x = (self.longitude + 180.0) / 360.0;
        let lat_radians = (self.latitude) / 180.0 * PI;
        let y = (1.0 - lat_radians.tan().asinh() / PI) / 2.0;
        (x as f64, y as f64)
    }
}

/// A single point on earth that carries some associated information.
#[derive(Debug)]
pub struct GeoNode {
    pub tags: HashMap<String, String>,
}

/// A map feature that models a building.
#[derive(Debug)]
pub struct BuildingFeature {
    pub nodes: Vec<u64>,
    pub tags: HashMap<String, String>,
}

/// A map feature that models a road.
#[derive(Debug)]
pub struct RoadFeature {
    pub nodes: Vec<u64>,
    pub tags: HashMap<String, String>,
}

/// A map feature that models the land use of an area.
#[derive(Debug)]
pub struct LandUseFeature {
    pub nodes: Vec<u64>,
    pub tags: HashMap<String, String>,
}

#[derive(Debug)]
pub struct LakeFeature {
    pub nodes: Vec<u64>,
    pub tags: HashMap<String, String>,
}

#[derive(Debug)]
pub struct RiverFeature {
    pub nodes: Vec<u64>,
    pub tags: HashMap<String, String>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FeatureType {
    Building,
    Road,
    LandUse,
    Lake,
    River
}

/// Converts a Serde JSON value to the internal `GeoData` data structure.
/// 
/// The locations of nodes that were in the JSON data but not in
/// `node_locations` yet will be added to this map. Nodes will also be added
/// to the `nodes` field on the result iff they have any tags.
/// 
/// # See also
/// [OSM JSON format], [Overpass JSON format] (almost the same)
/// 
/// [OSM JSON format]: https://wiki.openstreetmap.org/wiki/OSM_JSON
/// [Overpass JSON format]: https://dev.overpass-api.de/output_formats.html#json
pub fn convert_osm_json(
    json: JsonValue,
) -> Result<GeoData, AppError> {
    // good example: https://api.openstreetmap.org/api/0.6/relation/10000000/full.json
    let root_object = match json {
        JsonValue::Object(object) => object,
        _ => return error("OSM JSON root must be an object"),
    };

    let elements = match root_object.get("elements") {
        Some(JsonValue::Array(array)) => array,
        _ => return error(
            "OSM JSON root needs to have an `elements` key that is an array",
        ),
    };

    // we do two passes: one purely for node locations and one for other
    // features, because we need to find the chunk that a feature lies in by
    // the location of its nodes
    let mut node_locations = HashMap::new();

    for element in elements {
        let element_object = match element {
            JsonValue::Object(object) => object,
            _ => return error(
                "an element in the `elements` array must be an object",
            ),
        };

        if get_element_type(element_object)? == "node" {
            let id = get_id(element_object)?;

            // if a node doesn't have "lon" and "lat", we ignore
            let longitude = match element_object.get("lon") {
                Some(JsonValue::Number(number)) => truncate_to_f64(number),
                _ => continue,
            };
            let latitude = match element_object.get("lat") {
                Some(JsonValue::Number(number)) => truncate_to_f64(number),
                _ => continue,
            };
            let location = GeoLocation { longitude, latitude };
            node_locations.insert(id, location);
        }
    }

    let mut chunks = HashMap::new();

    for element in elements {
        let element_object = element.as_object().unwrap_throw();

        let element_type = get_element_type(element_object)?;
        let id = get_id(element_object)?;
        let tags = get_tags(element_object)?;

        // the "type"s that exist and their formats:
        // "type": "node", "id": num, [ "lon": num, "lat": num, "tags": <...> ]
        // "type": "way", "id": num, [ "tags": obj, ] "nodes": array
        // "type": "relation", "id": num, "members": array, [ "tags": obj ]
        // tags that seem interesting: landuse, highway, religion, historic,
        // addr:*, building:*, name
        match element_type {
            "node" => {
                if !tags.is_empty() {
                    let location = match node_locations.get(&id) {
                        Some(location) => location,
                        None => return error("node has tags but no location"),
                    };
                    let chunk = ChunkIndex::from_vec2(location.project( &Offset { x: 0.0, y: 0.0 }));  // TODO verify if this works once offset is changed
                    chunks.entry(chunk)
                        .or_insert(Chunk::default())
                        .nodes.insert(id, GeoNode { tags });
                }
            },
            "way" => {
                // confusingly, things like buildings are also "way"s
                let nodes_field = match element_object.get("nodes") {
                    Some(JsonValue::Array(array)) => array,
                    _ => return error(
                        "a \"way\" element must have a `nodes` key",
                    ),
                };
                let nodes = match parse_u64_array(nodes_field) {
                    Some(nodes) => nodes,
                    None => return error(
                        "`nodes` array must not contain non-integral values",
                    ),
                };

                if let Some(feature_type) = find_feature_type(&tags) {
                    // to determine in what chunk a feature lies, we take the
                    // average of the locations of its nodes
                    let mut sum_lon = 0.0;
                    let mut sum_lat = 0.0;
                    let mut count = 0usize;
                    for id in &nodes {
                        if let Some(location) = node_locations.get(id) {
                            sum_lon += location.longitude;
                            sum_lat += location.latitude;
                            count += 1;
                        }
                        // we ignore node IDs that are not in the data
                    }

                    if count == 0 {
                        continue;
                    }

                    let avg = GeoLocation {
                        longitude: sum_lon / count as f64,
                        latitude: sum_lat / count as f64,
                    };
                    let index = ChunkIndex::from_vec2(avg.project(&Offset { x: 0.0, y: 0.0 }));  // TODO verify
                    let chunk = chunks.entry(index)
                        .or_insert(Chunk::default());
                    match feature_type {
                        FeatureType::Building => {
                            chunk.building_features
                                .insert(id, BuildingFeature { nodes, tags });
                        },
                        FeatureType::Road => {
                            chunk.road_features
                                .insert(id, RoadFeature { nodes, tags });
                        },
                        FeatureType::LandUse => {
                            chunk.land_use_features
                                .insert(id, LandUseFeature { nodes, tags });
                        },
                        FeatureType::Lake => {
                            chunk.lake_features
                                .insert(id, LakeFeature { nodes, tags });
                        },
                        FeatureType::River => {
                            chunk.river_features
                                .insert(id, RiverFeature { nodes, tags });
                        },
                    }
                }

                // Adjusted conditions for lakes and rivers based on new definitions
                // https://wiki.openstreetmap.org/wiki/Key:water
                // Remove this if merge successful
                // && ["pond", "harbour", "lake", "lagoon", "reflecting_pool", "oxbow", "stream"].iter().any(|&value| tags.get("water").map_or(false, |water| water == value))
                // else if tags.get("natural") == Some(&String::from("water"))  {
                //     lake_features.insert(id, WaterFeature { nodes, tags: tags.clone() });
                // } else if ["river", "canal", "ditch", "stream"].iter().any(|&value| tags.get("waterway").map_or(false, |waterway| waterway == value)) {
                //     river_features.insert(id, WaterFeature { nodes, tags: tags.clone() });
                // }
            },
            "relation" => {
                // ignore for now
                // TODO forest have rings and the rings have nodes this is a relation
            },
            _ => {},
        }
    }

    Ok(GeoData { node_locations, chunks })
}

/// For an element in the JSON "elements" array, returns the "type" field if it
/// is there and it's a string.
fn get_element_type<'a>(
    element_object: &'a Map<String, JsonValue>,
) -> Result<&'a str, AppError> {
    match element_object.get("type") {
        Some(JsonValue::String(string)) => Ok(string),
        _ => Err(error(
            "an element must have a `type` tag that is a string",
        ).unwrap_err()),
    }
}

fn get_id(
    element_object: &Map<String, JsonValue>,
) -> Result<u64, AppError> {
    match element_object.get("id") {
        Some(JsonValue::Number(number)) if number.is_u64() => {
            Ok(number.as_u64().unwrap_throw())
        },
        _ => {
            Err(error(
                "an element must have an `id` tag that is a nonnegative integer",
            ).unwrap_err())
        },
    }
}

/// For en element in the JSON "elements" array, returns a map of its tags,
/// or an error if not all key-values pairs are from string to string.
fn get_tags(
    element_object: &Map<String, JsonValue>,
) -> Result<HashMap<String, String>, AppError> {
    let tags_field = match element_object.get("tags") {
        Some(JsonValue::Object(object)) => object,
        Some(_) => return Err(error("`tags` field must be an object").unwrap_err()),
        None => return Ok(HashMap::new()),
    };

    let mut result = HashMap::new();
    for (key, value) in tags_field {
        let string = match value {
            JsonValue::String(string) => string,
            _ => return Err(error(
                "`tags` field must be a map from strings to strings",
            ).unwrap_err()),
        };
        result.insert(key.clone(), string.clone());
    }
    Ok(result)
}

fn find_feature_type(
    tags: &HashMap<String, String>,
) -> Option<FeatureType> {
    if tags.contains_key("building") {
        Some(FeatureType::Building)
    } else if tags.contains_key("waterway") {
        Some(FeatureType::River)
    } else if tags.contains_key("highway") {
        Some(FeatureType::Road)
    } else if tags.contains_key("landuse") {
        Some(FeatureType::LandUse)
    } else if tags.get("natural") == Some(&String::from("water")) {
        Some(FeatureType::Lake)
    }
    else {
        None
    }
}

/// Converts an array of JSON values to an array of `u64`, or returns `None` if not all
/// values are nonnegative integers that fit in a `u64`.
fn parse_u64_array(object: &Vec<JsonValue>) -> Option<Vec<u64>> {
    let mut result = Vec::new();
    for value in object {
        if let Some(u) = value.as_u64() {
            result.push(u);
        } else {
            return None;
        }
    }
    Some(result)
}

/// Converts a JSON number
fn truncate_to_f64(number: &Number) -> f64 {
    if let Some(f) = number.as_f64() {
        f
    } else if let Some(i) = number.as_i64() {
        i as f64
    } else if let Some(u) = number.as_u64() {
        u as f64
    } else {
        unreachable!()
    }
}

fn error(message: &str) -> Result<GeoData, AppError> {
    Err(AppError::DataSyntax {
        format: DataFormat::OsmJson,
        line: None,
        character: None,
        message: message.to_owned(),
    })
}
