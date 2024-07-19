use crate::data::road_type::{road_type_to_color, RoadType};

use bevy::prelude::*;
use bevy::render::render_asset::RenderAssetUsages;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};
use bevy::render::texture::ImageSampler;

use std::ops::RangeInclusive;

use strum::IntoEnumIterator;

use super::agent::AgentType;

/// A global cache for assets that are reused between geographic features.
#[derive(Resource)]
pub struct AssetCache {
    /// The number of different colors in the building color textures.
    building_texture_count: u32,
    building_material: Handle<StandardMaterial>,

    /// The number of different colors in the building color textures.
    road_texture_count: u32,
    road_material: Handle<StandardMaterial>,
    river_material: Handle<StandardMaterial>,

    triangle_tree: Handle<Mesh>,
    complex_tree: Handle<Mesh>,
    complex_tree_simple: Handle<Mesh>,
    tree_material: Handle<StandardMaterial>,

    grass_material: Handle<StandardMaterial>,
    white_material: Handle<StandardMaterial>,

    agent_car_mesh: Handle<Mesh>,
    agent_car_mesh_simple: Handle<Mesh>,
    agent_car_material: Handle<StandardMaterial>,
    agent_car_material_simple: Handle<StandardMaterial>,
    agent_pedestrian_mesh: Handle<Mesh>,
    agent_pedestrian_mesh_simple: Handle<Mesh>,
    agent_pedestrian_material: Handle<StandardMaterial>,
}

impl AssetCache {
    /// Returns a clone of this object where all the handles to assets are
    /// weak.
    ///
    /// Used for when you don't want a clone of this cache to prevent resources
    /// from being freed.
    pub fn clone_weak(&self) -> Self {
        AssetCache {
            building_texture_count: self.building_texture_count,
            building_material: self.building_material.clone_weak(),
            road_texture_count: self.road_texture_count,
            road_material: self.road_material.clone_weak(),
            river_material: self.river_material.clone_weak(),
            triangle_tree: self.triangle_tree.clone_weak(),
            complex_tree: self.complex_tree.clone_weak(),
            complex_tree_simple: self.complex_tree_simple.clone_weak(),
            tree_material: self.tree_material.clone_weak(),
            grass_material: self.grass_material.clone_weak(),
            white_material: self.white_material.clone_weak(),
            agent_car_mesh: self.agent_car_mesh.clone_weak(),
            agent_car_mesh_simple: self.agent_car_mesh_simple.clone_weak(),
            agent_car_material: self.agent_car_material.clone_weak(),
            agent_car_material_simple: self.agent_car_material.clone_weak(),
            agent_pedestrian_mesh: self.agent_pedestrian_mesh.clone_weak(),
            agent_pedestrian_mesh_simple: self.agent_pedestrian_mesh_simple.clone_weak(),
            agent_pedestrian_material: self.agent_pedestrian_material.clone_weak(),
        }
    }

    /// Returns a handle to the material used for buildings.
    pub fn get_building_material(&self) -> Handle<StandardMaterial> {
        Handle::clone(&self.building_material)
    }

    /// Returns the number of different building styles (currently just plain
    /// colors) that are stored in the building texture atlas.
    pub fn get_building_texture_count(&self) -> u32 {
        self.building_texture_count
    }

    /// Returns for a building style index the (u, v) coordinate range in the
    /// building texture atlas.
    pub fn get_wall_uv(&self, index: u32) -> (RangeInclusive<f32>, RangeInclusive<f32>) {
        assert!(index < self.building_texture_count);
        let interval_size = 1.0 / self.building_texture_count as f32;
        let x_range = index as f32 * interval_size..=(index + 1) as f32 * interval_size;
        (x_range, 0.0..=1.0)
    }

    /// Returns a handle to the material used for roads, which uses
    /// a texture "atlas" that contains all possible colors for the road. This
    /// is necessary to combine road meshes within a chunk.
    pub fn get_road_material(&self) -> Handle<StandardMaterial> {
        Handle::clone(&self.road_material)
    }

    /// Returns a handle to the material used for roads, which uses
    /// a texture "atlas" that contains all possible colors for the road. This
    /// is necessary to combine river meshes within a chunk.
    pub fn get_river_material(&self) -> Handle<StandardMaterial> {
        Handle::clone(&self.river_material)
    }

    pub fn get_road_uv(&self, road_type: RoadType) -> (RangeInclusive<f32>, RangeInclusive<f32>) {
        let index = road_type as u32;
        assert!(index < self.road_texture_count);
        let interval_size = 1.0 / self.road_texture_count as f32;
        let x_range = index as f32 * interval_size..=(index + 1) as f32 * interval_size;
        (x_range, 0.0..=1.0)
    }

    pub fn get_river_uv(&self) -> (RangeInclusive<f32>, RangeInclusive<f32>) {
        (0.0..=1.0, 0.0..=1.0)
    }

    /// Returns a handle to the triangle tree mesh.
    pub fn get_triangle_tree_mesh(&self) -> Handle<Mesh> {
        Handle::clone(&self.triangle_tree)
    }

    /// Returns a handle to the complex tree mesh.
    pub fn get_complex_tree_mesh(&self) -> Handle<Mesh> {
        Handle::clone(&self.complex_tree)
    }

    /// Returns a handle to the simplified mesh of the complex tree.
    pub fn get_simplified_complex_tree_mesh(&self) -> Handle<Mesh> {
        Handle::clone(&self.complex_tree_simple)
    }

    /// Returns the material that is used for all trees.
    pub fn get_tree_material(&self) -> Handle<StandardMaterial> {
        Handle::clone(&self.tree_material)
    }

    /// Returns a handle to material used for grass areas.
    pub fn get_grass_material(&self) -> Handle<StandardMaterial> {
        Handle::clone(&self.grass_material)
    }

    pub fn get_agent_mesh(&self, agent_type: AgentType, simple: bool) -> Handle<Mesh> {
        if simple {
            match agent_type {
                AgentType::Car => Handle::clone(&self.agent_car_mesh_simple),
                AgentType::Pedestrian => Handle::clone(&self.agent_pedestrian_mesh_simple),
            }
        } else {
            match agent_type {
                AgentType::Car => Handle::clone(&self.agent_car_mesh),
                AgentType::Pedestrian => Handle::clone(&self.agent_pedestrian_mesh),
            }
        }
    }

    pub fn get_agent_material(
        &self,
        agent_type: AgentType,
        simple: bool,
    ) -> Handle<StandardMaterial> {
        match agent_type {
            AgentType::Car => {
                if simple {
                    Handle::clone(&self.agent_car_material_simple)
                } else {
                    Handle::clone(&self.agent_car_material)
                }
            }
            AgentType::Pedestrian => Handle::clone(&self.agent_pedestrian_material),
        }
    }
}

/// A system that initializes the global asset cache for geographic features.
pub fn setup_asset_cache(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut images: ResMut<Assets<Image>>,
    asset_server: Res<AssetServer>,
) {
    // buildings
    let mut building_texture_data = Vec::new();
    for i in 0..10 {
        let color = Color::hsl(i as f32 / 10.0 * 360.0, 1.0, 0.75);
        building_texture_data.extend(color.as_rgba_u8());
    }

    let building_texture_count = (building_texture_data.len() / 4) as u32;
    let building_texture_atlas = images.add(create_color_map(building_texture_data));
    let building_material = materials.add(create_texture_material(building_texture_atlas));

    // roads
    let mut road_texture_data = Vec::new();
    for road_type in RoadType::iter() {
        road_texture_data.extend(road_type_to_color(&road_type).as_rgba_u8());
    }

    let road_texture_count = (road_texture_data.len() / 4) as u32;
    let road_texture_atlas = images.add(create_color_map(road_texture_data));
    let road_material = materials.add(create_texture_material(road_texture_atlas));

    let river_material = materials.add(StandardMaterial {
        base_color: Color::BLUE,
        cull_mode: None,
        ..default()
    });

    let white_material = materials.add(Color::WHITE);

    // trees
    let tree_texture_data = vec![
        6, 33, 3, 255, // green
        34, 15, 1, 255, // brown
    ];
    let mut tree_image = create_color_map(tree_texture_data);
    // nearest sampler because of how the UV coordinates are set up in blender
    tree_image.sampler = ImageSampler::nearest();
    let tree_atlas = images.add(tree_image);
    let tree_material = materials.add(create_texture_material(tree_atlas));
    let triangle_tree = asset_server.load("triangle-tree.glb#Mesh0/Primitive0");
    let complex_tree = asset_server.load("complex-tree.glb#Mesh0/Primitive0");
    let complex_tree_simple = asset_server.load("complex-tree-simple.glb#Mesh0/Primitive0");

    // Grass
    let grass_material = materials.add(StandardMaterial {
        base_color: Color::rgba_u8(128, 180, 10, 255), // Green from color palette
        cull_mode: None,
        ..default()
    });

    let agent_car_mesh = asset_server.load("Car.glb#Mesh0/Primitive0");
    let agent_car_material = materials.add(create_texture_material(
        asset_server.load("Car_texture.png").into(),
    ));

    let agent_car_mesh_simple = asset_server.load("Car_low.glb#Mesh0/Primitive0");
    let agent_car_material_simple = materials.add(StandardMaterial {
        base_color: Color::rgba_u8(227, 0, 6, 255), // Red from car :)
        ..default()
    });

    let agent_pedestrian_mesh =
        meshes.add(Mesh::from(Capsule3d::new(0.5, 1.0)).translated_by(Vec3::new(0.0, 1.0, 0.0)));
    let agent_pedestrian_mesh_simple = agent_pedestrian_mesh.clone();
    let agent_pedestrian_material = materials.add(StandardMaterial {
        base_color: Color::rgb(0.1, 0.2, 0.3),
        ..Default::default()
    });

    commands.insert_resource(AssetCache {
        building_texture_count,
        building_material,
        road_texture_count,
        road_material,
        river_material,
        triangle_tree,
        complex_tree_simple,
        complex_tree,
        tree_material,
        grass_material,
        white_material,
        agent_car_mesh,
        agent_car_mesh_simple,
        agent_car_material,
        agent_pedestrian_mesh,
        agent_pedestrian_mesh_simple,
        agent_car_material_simple,
        agent_pedestrian_material,
    });
}

/// Creates an image (texture) with thee given data, assumed to be RGBA.
fn create_color_map(texture_data: Vec<u8>) -> Image {
    let count = texture_data.len() as u32 / 4;
    Image::new(
        Extent3d {
            width: count,
            height: 1,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        texture_data,
        TextureFormat::Rgba8Unorm,
        RenderAssetUsages::RENDER_WORLD,
    )
}

fn create_texture_material(texture: Handle<Image>) -> StandardMaterial {
    StandardMaterial {
        base_color: Color::WHITE,
        base_color_texture: Some(texture),
        cull_mode: None,
        ..default()
    }
}
