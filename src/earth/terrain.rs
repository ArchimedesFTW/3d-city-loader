
use crate::data::geography::{GeoLocation, LandUseFeature, Offset};
use crate::earth::GLOBAL_SCALE_FACTOR;
use crate::earth::mesh_builder::MeshBuilder;
use crate::earth::simplification::simplify_polygon;
use wasm_bindgen::prelude::*;

use std::collections::HashMap;

use bevy::prelude::*;

// Import randon
use rand::Rng;

// Amount of trees per area
const DENSITY: f32 = 0.05;

// NOTE: higher than for e.g. buildings
const TERRAIN_SIMPLIFICATION_THRESHOLD: f32 = 0.0001 * GLOBAL_SCALE_FACTOR * GLOBAL_SCALE_FACTOR;

// Used following color scheme:
// https://www.schemecolor.com/tree-green-brown.php

// https://www.youtube.com/watch?v=xVRHYWfAJkI
fn generate_trees(
    node_locations: &HashMap<u64, GeoLocation>,
    feature: &LandUseFeature,
    offset: &Offset,
    tree_transforms: &mut Vec<Transform>,
) {
    let area: Vec<Vec2> = feature
        .nodes
        .iter()
        .filter_map(|node_id| node_locations.get(node_id).map(|node| node.project(&offset)))
        .collect();
    let area_simplified = simplify_polygon(area, TERRAIN_SIMPLIFICATION_THRESHOLD);

    let points = get_random_points_in_polygon(&area_simplified, DENSITY);  

    for point in points.iter() {
        let rotation =
            Quat::from_rotation_y(rand::thread_rng().gen_range(0.0..std::f32::consts::PI));

        let scale: f32 = rand::thread_rng().gen_range(0.015..0.025) * GLOBAL_SCALE_FACTOR; 
        let scale: Vec3 = Vec3::new(scale, scale, scale);

        // Random position
        let position = Vec3::new(point.x as f32, 0.0, point.y as f32);

        // Random tree
        let transform = Transform::from_translation(position)
            .with_rotation(rotation)
            .with_scale(scale);
        tree_transforms.push(transform);
    }  
}

/// Get random points in a polygon, with a given density.
/// Compute area of each triangle,
/// Pick triangle based on area
/// Pick random point in that triangle with another algorithm.
fn get_random_points_in_polygon(
    area: &Vec<Vec2>,
    density: f32,
) -> Vec<Vec2> {
    // Compute triangulation and total area only once for efficiency
    let triangles = get_triangles(area);
    
    let total_area = triangles.iter().map(|triangle| {
        get_area_triangle(*triangle)
    }).sum::<f32>();

    let num_points = (total_area * density) as usize;

    // Generate random points
    let mut points = Vec::new();
    for _ in 0..num_points {
        points.push(get_random_point(&triangles, total_area));
    }
    points
}

/// Get the area of a triangle
fn get_area_triangle(
    triangle: [Vec3; 3]
) -> f32 {
    let a = triangle[0];
    let b = triangle[1];
    let c = triangle[2];

    0.5 * ((a.x*(b.z-c.z) + b.x*(c.z-a.z) + c.x*(a.z-b.z)) as f32).abs()
}

/// Pick a random point in a triangle
fn get_random_point(
    triangles: &Vec<[Vec3; 3]>,
    total_area: f32,
) -> Vec2 {
    let mut rng = rand::thread_rng();

    // Pick a random triangle based on area
    let mut area_sum = 0.0;
    let mut triangle_index = 0;
    let random_area = rng.gen_range(0.0..total_area);
    for (i, triangle) in triangles.iter().enumerate() {
        let area = get_area_triangle(*triangle);        

        area_sum += area;
        if area_sum >= random_area {
            triangle_index = i;
            break;
        }
    }

    // Pick a random point in the triangle
    get_random_point_in_triangle(triangles[triangle_index])
}

/// Get a random point in a triangle
/// 
/// https://graphics.stanford.edu/courses/cs468-08-fall/pdf/osada.pdf
/// 
/// Use the formula:
/// P = (1 − √r1) A + √r1(1 − r2) B + √r1 r2 C (1)
/// r1 and r2 are random numbers between 0 and 1
fn get_random_point_in_triangle(
    triangle: [Vec3; 3]
) -> Vec2 {
    let mut rng = rand::thread_rng();

    let a = triangle[0];
    let b = triangle[1];
    let c = triangle[2];

    let r1: f32 = rng.gen_range(0.0..1.0);
    let r2: f32 = rng.gen_range(0.0..1.0);
    
    // Only need sqrt of r1
    let sqrt_r1 = r1.sqrt();

    let x = (1.0 - sqrt_r1) * a.x + sqrt_r1 * (1.0 - r2) * b.x + sqrt_r1 * r2 * c.x;
    let z = (1.0 - sqrt_r1) * a.z + sqrt_r1 * (1.0 - r2) * b.z + sqrt_r1 * r2 * c.z;

    Vec2::new(x, z)
}

/// Get the triangulation of the area
fn get_triangles(
    area: &Vec<Vec2>,
) -> Vec<[Vec3; 3]> {
    let points: Vec<_> = area.iter().map(|vec2| geo::Point::new(vec2.x as f64, vec2.y as f64)).collect();
    let polygon = geo::Polygon::new(points.into(), vec![]);

    // Triangulate
    let mut mesh_builder = MeshBuilder::new();
    let uv = Vec2::new(0.0, 0.0);  // TODO tweak?
    mesh_builder.add_polygon_xz(&polygon, 0.0, uv);  // Up normal
    mesh_builder.get_triangles()
}

/// Creates the terrain data within one chunk. Returns a list of transforms for
/// trees that have to be placed, and a list of meshes for grass areas.
pub fn create_terrain_data(
    node_locations: &HashMap<u64, GeoLocation>,
    land_use_features: &HashMap<u64, LandUseFeature>,
    offset: &Offset
) -> (Vec<Transform>, Vec<Mesh>) {
    let mut tree_transforms = Vec::new();
    let mut grass_areas = Vec::new();
    for (_, feature) in land_use_features {
        let landuse = feature.tags.get("landuse").unwrap_throw();

        // Generate trees
        if landuse == "forest" || landuse == "wood" {
            // NOTE: it turns out that combining the meshes into one does not
            // improve rendering performance, because instancing in bevy is
            // pretty good when the meshes and materials are all equal
            generate_trees(
                node_locations,
                feature,
                offset,
                &mut tree_transforms,
            );
        }

        // Generate grass area
        if landuse == "forest" || landuse == "wood" || landuse == "grass" {
            grass_areas.push(generate_area(node_locations, feature, offset));
        }
    }
    (tree_transforms, grass_areas)
}

fn generate_area(
    node_locations: &HashMap<u64, GeoLocation>,
    feature: &LandUseFeature,
    offset: &Offset,
) -> Mesh {
    let area: Vec<Vec2> = feature
        .nodes
        .iter()
        .filter_map(|node_id| node_locations.get(node_id).map(|node| node.project(&offset)))
        .collect();

    // Render green area plane
    let temp: Vec<_> = area.iter().map(|vec2| geo::Point::new(vec2.x as f64, vec2.y as f64)).collect();
    let polygon = geo::Polygon::new(temp.into(), vec![]);

    let mut mesh_builder = MeshBuilder::new();
    let uv = Vec2::new(0.0, 0.0);  // TODO tweak?
    mesh_builder.add_polygon_xz(&polygon, 0.002, uv);  // Render under lakes
    mesh_builder.into_mesh()
}
