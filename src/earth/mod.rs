use crate::common::{handle_compute_tasks, spawn_compute_task, AsyncComputation, StatusEvent};

use crate::data::geography::{GeoData, GeoLocation, Offset};
use crate::data::traffic_graph::{update_traffic_graph, TrafficGraph};
use crate::earth::agent::create_agents;
use crate::earth::assets::AssetCache;
use crate::earth::buildings::create_building_data;
use crate::earth::lakes::update_lake;
use crate::earth::rivers::create_river_data;
use crate::earth::roads::create_road_data;
use crate::earth::terrain::create_terrain_data;
use crate::lod::{DEFAULT_LOD_DISTANCE_SQUARED, DEFAULT_REMOVE_DISTANCE_SQUARED, LOD};
use crate::player::Player;
use wasm_bindgen::prelude::*;

use bevy::prelude::*;
use noise::{NoiseFn, Perlin};

use std::cmp::min;
use std::f32::consts::PI;
use std::sync::Arc;

use self::agent::Agent;

use std::cmp::Ordering;
pub mod agent;
pub mod assets;
pub mod buildings;
pub mod lakes;
pub mod mesh_builder;
pub mod rivers;
pub mod roads;
pub mod simplification;
pub mod terrain;
pub mod trajectory;

pub const GLOBAL_SCALE_FACTOR: f32 = 100.0;

pub const CHANCE_COMPLEX_TREE: f64 = 0.0;

/// Sets up an empty earth.
pub fn setup_earth(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    // light
    let rotation = Quat::from_rotation_x(-PI / 3.0);
    commands.spawn(DirectionalLightBundle {
        directional_light: DirectionalLight {
            shadows_enabled: true,
            illuminance: 10_000.0,
            ..default()
        },
        transform: Transform::from_xyz(0.0, 10.0, 0.0).with_rotation(rotation),
        ..default()
    });

    // add a tiny plane, just to have some sort of reference frame
    commands
        .spawn(PbrBundle {
            mesh: meshes.add(
                Plane3d::default()
                    .mesh()
                    .size(BASE_PLANE_SIZE, BASE_PLANE_SIZE),
            ),
            material: materials.add(Color::WHITE),
            transform: Transform::from_translation(Vec3::new(0.0, 0.0, 0.0)),
            ..default()
        })
        .insert(GeoFeature { id: 0 });
}

const BASE_PLANE_SIZE: f32 = 10.0 * GLOBAL_SCALE_FACTOR;

/// An event that adds new geographic data to the world.
#[derive(Debug, Event)]
pub struct GeoDataEvent {
    pub data: Arc<GeoData>,
}

// Max distance is set to euclidian distance from Eindhoven to Izmir. Might be adjusted down if too many artifacts persist
pub const MAX_DISTANCE: f64 = 0.083291353581523;

/// A system that updates the world when new data should be added. // TODO: how does this work with removals?
pub fn update_earth(
    mut commands: Commands,
    mut players: Query<(&Player, &mut Transform)>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut geo_data_events: EventReader<GeoDataEvent>,
    mut status_events: EventWriter<StatusEvent>,
    asset_cache: Res<AssetCache>,
    mut offset_resource: ResMut<Offset>,
    geo_query: Query<(Entity, &GeoFeature)>,
    agent_query: Query<(Entity, &Agent)>,
    mut traffic_graph: ResMut<TrafficGraph>,
) {
    for event in geo_data_events.read() {
        let old_traffic_graph_size = traffic_graph.get_size();

        // Get the current offset
        let mut offset = offset_resource.clone();

        // First compute center and bounds
        let (_, avg, _) = find_bounds(&event.data);
        let offset_candidate: Offset = Offset {
            x: avg.project_no_scale().0,
            y: avg.project_no_scale().1,
        };

        // Calculate the Euclidean distance between the current offset and the candidate offset
        let distance = ((offset_candidate.x - offset.x).powi(2)
            + (offset_candidate.y - offset.y).powi(2))
        .sqrt();

        if distance > MAX_DISTANCE {
            delete_all(&mut commands, &geo_query, &agent_query, &mut traffic_graph);
            println!("Too far away, deleting old data"); // TODO possibly notify the user

            // Update offset
            *offset_resource = offset_candidate;
            offset = offset_candidate;
        }

        for (index, _) in &event.data.chunks {
            // Update buildings, handle result in `update_building_generation_tasks`
            let data = Arc::clone(&event.data);
            let index_clone = index.clone(); // for borrow checking purposes
            let asset_cache_ref = asset_cache.clone_weak();
            spawn_compute_task(&mut commands, async move {
                let chunk = data.chunks.get(&index_clone).unwrap_throw();
                let mesh = create_building_data(
                    &data.node_locations,
                    &chunk.building_features,
                    &chunk.land_use_features,
                    &asset_cache_ref,
                    &offset,
                );
                BuildingCreation(mesh)
            });

            // Update roads, handle result in `update_road_generation_tasks`
            let data = Arc::clone(&event.data);
            let index_clone = index.clone();
            let asset_cache_ref = asset_cache.clone_weak();
            spawn_compute_task(&mut commands, async move {
                let chunk = data.chunks.get(&index_clone).unwrap_throw();
                let mesh = create_road_data(
                    &data.node_locations,
                    &chunk.road_features,
                    &asset_cache_ref,
                    &offset,
                );
                RoadCreation(mesh)
            });

            // Update traffic network graph
            let data = Arc::clone(&event.data);
            let index_clone = index.clone();
            // spawn_compute_task(&mut commands, async move {
            {
                let chunk = data.chunks.get(&index_clone).unwrap_throw();
                update_traffic_graph(
                    &data.node_locations,
                    &chunk.road_features,
                    &mut traffic_graph,
                    &offset,
                );
            }
            // });

            // Update rivers
            let data = Arc::clone(&event.data);
            let index_clone = index.clone();
            let asset_cache_ref = asset_cache.clone_weak();
            spawn_compute_task(&mut commands, async move {
                let chunk = data.chunks.get(&index_clone).unwrap_throw();
                let mesh = create_river_data(
                    &data.node_locations,
                    &chunk.river_features,
                    &asset_cache_ref,
                    &offset,
                );
                RiverCreation(mesh)
            });

            let data = Arc::clone(&event.data);
            let index_clone = index.clone();
            let chunk = data.chunks.get(&index_clone).unwrap_throw();
            update_lake(
                &mut commands,
                &mut meshes,
                &mut materials,
                &data.node_locations,
                &chunk.lake_features,
                &offset,
            );

            // Update terrain, handle result in `update_terrain_generation_tasks`
            let data = Arc::clone(&event.data);
            let index_clone = index.clone();
            spawn_compute_task(&mut commands, async move {
                let chunk = data.chunks.get(&index_clone).unwrap_throw();
                let (tree_transforms, grass_areas) =
                    create_terrain_data(&data.node_locations, &chunk.land_use_features, &offset);
                TerrainCreation(tree_transforms, grass_areas)
            });
        }

        // Print size of traffic graph
        println!("Updated traffic graph size: {}", traffic_graph.get_size());

        // Spawn agents tasks async in batches of 100
        let graph_arc = Arc::new((*traffic_graph).clone()); // This is not a great way to do it, but the graph
                                                            // needs to stay mutable for next iteration while also having the data available for the agents
        let diff = if old_traffic_graph_size > traffic_graph.get_size() {
            0 // or some other default value
        } else {
            (traffic_graph.get_size() - old_traffic_graph_size) / 100
        };
        // Only add diff if positive, else add 0
        let mut agent_spawns_left = diff.max(0) as i32;

        while agent_spawns_left > 0 {
            let graph = graph_arc.clone();
            let spawns = min(100, agent_spawns_left);
            spawn_compute_task(&mut commands, async move {
                let agents = create_agents(spawns, graph);

                AgentCreation(agents)
            });
            agent_spawns_left -= 100;
        }

        // Add a plane underneath
        let (min, avg, max) = find_bounds(&event.data);
        let min = min.project(&offset);
        let max = max.project(&offset);
        let x_size = (max.x - min.x).abs();
        let z_size = (max.y - min.y).abs();
        let mid_x = (min.x + max.x) / 2.0;
        let mid_z = (min.y + max.y) / 2.0;
        commands
            .spawn(PbrBundle {
                mesh: meshes.add(Plane3d::default().mesh().size(x_size, z_size)),
                material: materials.add(Color::WHITE),
                transform: Transform::from_translation(Vec3::new(mid_x, -0.1, mid_z)), // A bit below the ground, since we have rounding errors
                ..default()
            })
            .insert(GeoFeature { id: 0 });

        status_events.send(StatusEvent::Update(
            "Successfully added data, teleporting player".to_owned(),
        ));

        // Teleport player to average of nodes
        let new_position = avg.project(&offset);
        eprintln!("in the world that's {:?}", new_position);
        for (_, mut transform) in &mut players {
            transform.translation.x = new_position.x;
            transform.translation.z = new_position.y;
            if transform.translation.y <= 0.0 {
                transform.translation.y = 5.0;
            }
        }
    }
}

fn delete_all(
    commands: &mut Commands,
    geo_query: &Query<(Entity, &GeoFeature)>,
    agent_query: &Query<(Entity, &Agent)>,
    traffic_graph: &mut ResMut<TrafficGraph>,
) {
    delete_geo_features(commands, geo_query);
    delete_agents(commands, agent_query);
    traffic_graph.reset();
}

fn delete_geo_features(commands: &mut Commands, query: &Query<(Entity, &GeoFeature)>) {
    for (entity, _) in query.iter() {
        commands.entity(entity).despawn_recursive();
    }
}

fn delete_agents(commands: &mut Commands, query: &Query<(Entity, &Agent)>) {
    for (entity, _) in query.iter() {
        commands.entity(entity).despawn_recursive();
    }
}

/// Returns (-lat-lon corner, the median location, +lat+lon corner).
fn find_bounds(data: &GeoData) -> (GeoLocation, GeoLocation, GeoLocation) {
    // Initialize min and max values
    let mut min_lat = f64::MAX;
    let mut max_lat = f64::MIN;
    let mut min_lon = f64::MAX;
    let mut max_lon = f64::MIN;

    // Vector to store locations and their sums
    let mut loc_sums: Vec<(&GeoLocation, f64)> = Vec::new();

    // Populate the min, max values and the vector with locations and their sums
    for (_, location) in &data.node_locations {
        min_lat = min_lat.min(location.latitude);
        max_lat = max_lat.max(location.latitude);
        min_lon = min_lon.min(location.longitude);
        max_lon = max_lon.max(location.longitude);
        loc_sums.push((location, location.latitude + location.longitude));
    }

    // Sort locations by their sum
    loc_sums.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(Ordering::Equal));

    // Calculate the median location based on sorted sums
    let median_location = if !data.node_locations.is_empty() {
        let mid = loc_sums.len() / 2;
        if loc_sums.len() % 2 == 0 {
            // Even number of elements, choose the lower middle element for simplicity
            loc_sums[mid - 1].0
        } else {
            // Odd number of elements, median is the middle element
            loc_sums[mid].0
        }
    } else {
        // Handle empty data case by returning a default location
        &GeoLocation {
            latitude: 0.0,
            longitude: 0.0,
        }
    };

    (
        GeoLocation {
            latitude: min_lat,
            longitude: min_lon,
        },
        GeoLocation {
            latitude: median_location.latitude,
            longitude: median_location.longitude,
        },
        GeoLocation {
            latitude: max_lat,
            longitude: max_lon,
        },
    )
}

/// A system that polls building generations tasks that are not yet fulfilled.
pub fn update_building_generation_tasks(
    mut commands: Commands,
    query: Query<(Entity, &mut AsyncComputation<BuildingCreation>)>,
    mut meshes: ResMut<Assets<Mesh>>,
    asset_cache: Res<AssetCache>,
) {
    handle_compute_tasks(&mut commands, query, move |commands, data| {
        let BuildingCreation(mesh) = data;
        commands
            .spawn(PbrBundle {
                mesh: meshes.add(mesh),
                material: asset_cache.get_building_material(),
                ..default()
            })
            .insert(GeoFeature { id: 0 });
    })
}

/// A type for storing data generated by building generation tasks.
pub struct BuildingCreation(Mesh);

/// A system that polls road generation tasks that are not yet fulfilled.
pub fn update_road_generation_tasks(
    mut commands: Commands,
    query: Query<(Entity, &mut AsyncComputation<RoadCreation>)>,
    mut meshes: ResMut<Assets<Mesh>>,
    asset_cache: Res<AssetCache>,
) {
    handle_compute_tasks(&mut commands, query, move |commands, data| {
        let RoadCreation(mesh) = data;
        let entity_bundle = PbrBundle {
            mesh: meshes.add(mesh),
            material: asset_cache.get_road_material(), // TODO use this or generalize to trajectory
            ..default()
        };
        commands.spawn(entity_bundle).insert(GeoFeature { id: 0 });
    });
}

pub fn update_terrain_generation_tasks(
    mut commands: Commands,
    query: Query<(Entity, &mut AsyncComputation<TerrainCreation>)>,
    mut meshes: ResMut<Assets<Mesh>>,
    asset_cache: Res<AssetCache>,
) {
    handle_compute_tasks(&mut commands, query, move |commands, data| {
        let TerrainCreation(tree_transforms, grass_areas) = data;
        let perlin = Perlin::new(rand::random::<u32>());
        for transform in tree_transforms {
            // Get meshes, randomly pick between simple and complex trees
            let hq_mesh;
            let simple_mesh;
            if perlin.get(
                transform
                    .translation
                    .to_array()
                    .map(|val| val / GLOBAL_SCALE_FACTOR)
                    .map(f64::from),
            ) < CHANCE_COMPLEX_TREE
            {
                hq_mesh = asset_cache.get_complex_tree_mesh();
                simple_mesh = asset_cache.get_simplified_complex_tree_mesh();
            } else {
                hq_mesh = asset_cache.get_triangle_tree_mesh();
                simple_mesh = hq_mesh.clone();
            }

            // Spawn tree
            commands
                .spawn(PbrBundle {
                    mesh: hq_mesh.clone(),
                    material: asset_cache.get_tree_material(),
                    transform,
                    ..default()
                })
                .insert(GeoFeature { id: 0 })
                .insert(LOD {
                    remove_distance_squared: 2.0 * DEFAULT_REMOVE_DISTANCE_SQUARED,
                    lod_distance_distance_squared: DEFAULT_LOD_DISTANCE_SQUARED,
                    high_quality_mesh: hq_mesh,
                    high_quality_material: asset_cache.get_tree_material(),
                    low_quality_mesh: simple_mesh,
                    low_quality_material: asset_cache.get_tree_material(),
                });
        }
        for grass_area in grass_areas {
            commands
                .spawn(PbrBundle {
                    mesh: meshes.add(grass_area),
                    material: asset_cache.get_grass_material(),
                    ..Default::default()
                })
                .insert(GeoFeature { id: 0 });
        }
    });
}

/// A type for storing data generated by terrain generation tasks.
pub struct TerrainCreation(Vec<Transform>, Vec<Mesh>);

/// A system that polls road generation tasks that are not yet fulfilled.
pub fn update_river_generation_tasks(
    mut commands: Commands,
    query: Query<(Entity, &mut AsyncComputation<RiverCreation>)>,
    mut meshes: ResMut<Assets<Mesh>>,
    asset_cache: Res<AssetCache>,
) {
    handle_compute_tasks(&mut commands, query, move |commands, data| {
        let RiverCreation(mesh) = data;
        let entity_bundle = PbrBundle {
            mesh: meshes.add(mesh),
            material: asset_cache.get_river_material(), // TODO use this or generalize to trajectory
            ..default()
        };
        commands.spawn(entity_bundle).insert(GeoFeature { id: 0 });
    });
}

/// A type for storing data generated by async generation tasks.
pub struct RoadCreation(Mesh);

pub struct RiverCreation(Mesh);

/// Result of agent creation, is start location + agent component
pub struct AgentCreation(Vec<(Vec3, Agent)>);

/// A system that polls agent generation tasks that are not yet fulfilled.
pub fn update_agent_generation_tasks(
    mut commands: Commands,
    query: Query<(Entity, &mut AsyncComputation<AgentCreation>)>,
    asset_cache: Res<AssetCache>,
) {
    handle_compute_tasks(&mut commands, query, move |commands, data| {
        for agent_tuple in data.0 {
            let (start_location, agent) = agent_tuple;
            let agent_type = agent.agent_type;
            commands
                .spawn(PbrBundle {
                    mesh: asset_cache.get_agent_mesh(agent.agent_type, true),
                    material: asset_cache.get_agent_material(agent.agent_type, true),
                    transform: Transform::from_translation(start_location),
                    ..default()
                })
                .insert(agent)
                .insert(LOD {
                    remove_distance_squared: DEFAULT_REMOVE_DISTANCE_SQUARED,
                    lod_distance_distance_squared: DEFAULT_LOD_DISTANCE_SQUARED,
                    high_quality_mesh: asset_cache.get_agent_mesh(agent_type, false),
                    high_quality_material: asset_cache.get_agent_material(agent_type, false),
                    low_quality_mesh: asset_cache.get_agent_mesh(agent_type, true),
                    low_quality_material: asset_cache.get_agent_material(agent_type, true),
                });
        }
    });
}

/// Marks an entity as a geographic feature, saving its unique identifier.
#[derive(Component)]
#[allow(dead_code)]
pub struct GeoFeature {
    #[allow(dead_code)]
    id: u64,
}
