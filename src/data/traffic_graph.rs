use std::{collections::HashMap, convert::Infallible};
use wasm_bindgen::prelude::*;

use bevy::{
    ecs::system::{ResMut, Resource},
    math::Vec2,
};

use petgraph::{
    graph::{Graph, NodeIndex},
    Directed,
};

use crate::earth::agent::{agent_speed_on_road_type, AgentType, REFERENCE_SPEED};

use super::{
    geography::{GeoLocation, Offset, RoadFeature},
    road_type::RoadType,
}; // maybe use StableGraph in the future if we want to delete singular edges/nodes

/// The cost multiplier for disallowed edges for their agent type.
/// This is a very high number to discourage agents from using these edges.
const COST_MULTIPLIER_DISALLOWED: f32 = 100.0;

/// Directed graph structure for agents to travel in the world.
#[derive(Debug, Resource, Clone)]
pub struct TrafficGraph {
    graph: Graph<Vec2, (f32, RoadType), Directed, u32>, // Vertices hold their location in the plane, edges weighted by distance (and holds road type)
    hashmap: HashMap<u64, NodeIndex<u32>>,              // Maps OSM vertex IDs to graph indices
}

impl Default for TrafficGraph {
    fn default() -> Self {
        TrafficGraph {
            graph: Graph::new(),
            hashmap: HashMap::new(),
        }
    }
}

impl TrafficGraph {
    /// Add a vertex to the graph. Checks if the vertex already exists.
    pub fn add_node(&mut self, osm_id: u64, location: Vec2) -> NodeIndex<u32> {
        if let Some(index) = self.hashmap.get(&osm_id) {
            *index
        } else {
            let index = self.graph.add_node(location);
            self.hashmap.insert(osm_id, index);
            index
        }
    }

    /// Add an edge to the graph. Represents a way to travel between two vertices.
    pub fn add_connection(
        &mut self,
        from_index: u64,
        from_location: Vec2,
        to_index: u64,
        to_location: Vec2,
        oneway: OneWay,
        road_type: RoadType,
    ) {
        // Calculate the Euclidean distance between the two vertices
        let distance = (from_location - to_location).length();

        // We use update instead of add to not allow parallel edges
        let from_index = self.add_node(from_index, from_location);
        let to_index = self.add_node(to_index, to_location);
        match oneway {
            OneWay::Yes => {
                self.graph
                    .add_edge(from_index, to_index, (distance, road_type));
            }
            OneWay::No => {
                self.graph
                    .add_edge(from_index, to_index, (distance, road_type));
                self.graph
                    .add_edge(to_index, from_index, (distance, road_type));
            }
            OneWay::Reversed => {
                self.graph
                    .add_edge(to_index, from_index, (distance, road_type));
            }
        }
    }

    /// Get the index of a vertex in the graph for a given OSM node.
    pub fn get_index(&self, osm_id: u64) -> Option<NodeIndex<u32>> {
        self.hashmap.get(&osm_id).copied()
    }

    // Get the shortest path between two vertices in the graph, based on their node IDs
    pub fn get_shortest_path(
        &self,
        from_index: NodeIndex,
        to_index: NodeIndex,
        agent_type: AgentType,
    ) -> Option<Vec<NodeIndex>> {
        let goal_location = self.graph[to_index];

        let path = petgraph::algo::astar(
            &self.graph,
            from_index,
            |node| node == to_index,
            |edge| {
                let road_type = edge.weight().1;
                let mut weight = edge.weight().0; // Starting weight is the distance

                // See if road type is allowed for agent type
                if !road_type_allowed_for_agent_type(road_type, agent_type) {
                    weight = weight * COST_MULTIPLIER_DISALLOWED;
                }

                // Account for speed multiplier
                weight = weight / agent_speed_on_road_type(REFERENCE_SPEED, agent_type, road_type);

                // return
                weight
            },
            |node| {
                let location = self.graph[node];
                (goal_location - location).length()
            },
        )?;

        Some(path.1) // Discard the cost
    }

    pub fn reset(&mut self) {
        self.graph.clear();
        self.hashmap.clear();
    }

    pub fn get_size(&self) -> usize {
        self.graph.node_count()
    }

    pub fn get_node_location(&self, index: NodeIndex) -> Vec2 {
        self.graph[index]
    }

    pub fn get_random_node_index(&self) -> NodeIndex {
        let index = rand::random::<usize>() % self.graph.node_count();
        let node = self.graph.node_indices().nth(index).unwrap_throw();
        node
    }

    pub fn get_road_type(&self, from_index: NodeIndex, to_index: NodeIndex) -> RoadType {
        let edge = self.graph.find_edge(from_index, to_index);
        match edge {
            Some(edge) => {
                let road_type = self.graph[edge].1;
                road_type
            }
            None => return RoadType::NotCovered,
        }
    }
}

/// Should be made to work with async tasks, but for now it's synchronous.
pub fn update_traffic_graph(
    node_locations: &HashMap<u64, GeoLocation>,
    road_features: &HashMap<u64, RoadFeature>,
    graph: &mut ResMut<TrafficGraph>,
    offset: &Offset,
) {
    // Loop over roads and add the connections to the graph
    for (_, road) in road_features.iter() {
        let mut last_vertex_osm_id: Option<u64> = None;
        let mut last_vertex_location: Option<Vec2> = None;

        let oneway = match road.tags.get("oneway") {
            Some(value) => value.parse().unwrap_throw(),
            None => OneWay::No,
        };

        let road_type = match road.tags.get("highway") {
            Some(value) => value.parse().unwrap_throw(),
            None => RoadType::NotCovered,
        };

        for osm_vertex_id in road.nodes.iter() {
            let geolocation = match node_locations.get(osm_vertex_id) {
                Some(location) => location,
                None => continue, // We do not know the location of this node, should never happen
            };
            let location = geolocation.project(&offset);

            // Add node
            graph.add_node(*osm_vertex_id, location);

            // Add edge
            if let Some(last_node) = last_vertex_osm_id {
                let last_location = last_vertex_location.unwrap_throw();
                graph.add_connection(
                    last_node,
                    last_location,
                    *osm_vertex_id,
                    location,
                    oneway,
                    road_type,
                );
            }

            last_vertex_osm_id = Some(*osm_vertex_id);
            last_vertex_location = Some(location);
        }
    }
}

/// Represents if a road is one-way, two-way, or one-way with a reversed direction.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum OneWay {
    Yes,
    No,
    Reversed,
}

impl std::str::FromStr for OneWay {
    type Err = Infallible;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "yes" => Ok(OneWay::Yes),
            "true" => Ok(OneWay::Yes),
            "1" => Ok(OneWay::Yes),
            "no" => Ok(OneWay::No),
            "false" => Ok(OneWay::No),
            "0" => Ok(OneWay::No),
            "" => Ok(OneWay::No),
            "-1" => Ok(OneWay::Reversed),
            "reverse" => Ok(OneWay::Reversed),
            _ => Ok(OneWay::No), // If we don't know, assume two-way
        }
    }
}

fn road_type_allowed_for_agent_type(road_type: RoadType, agent_type: AgentType) -> bool {
    match agent_type {
        AgentType::Car => match road_type {
            RoadType::Motorway => true,
            RoadType::Trunk => true,
            RoadType::Primary => true,
            RoadType::Secondary => true,
            RoadType::Tertiary => true,
            RoadType::Residential => true,
            RoadType::TrunkLink => true,
            RoadType::PrimaryLink => true,
            RoadType::SecondaryLink => true,
            RoadType::TertiaryLink => true,
            RoadType::MotorwayLink => true,
            RoadType::NotCovered => true,
            _ => false,
        },
        AgentType::Pedestrian => match road_type {
            RoadType::Tertiary => true,
            RoadType::Residential => true,
            RoadType::Footway => true,
            RoadType::Steps => true,
            RoadType::Path => true,
            RoadType::Unclassified => true,
            RoadType::NotCovered => true,
            _ => false,
        },
    }
}
