use std::sync::Arc;
use wasm_bindgen::prelude::*;

use bevy::{
    ecs::{
        component::Component,
        entity::Entity,
        system::{Query, Res},
    },
    math::{vec2, Quat, Vec3},
    time::Time,
    transform::components::Transform,
};
use petgraph::graph::NodeIndex;

use crate::data::{
    road_type::{road_type_to_width, RoadType},
    traffic_graph::TrafficGraph,
};

use super::GLOBAL_SCALE_FACTOR;

/// Number between 0 and 1 that determines the split between pedestrian and car agents. Higher means more cars.
const PEDESTRIAN_CAR_SPLIT: f32 = 0.5;

/// Reference speed for agents. This is the speed of a pedestrian.
pub const REFERENCE_SPEED: f32 = 0.01 * GLOBAL_SCALE_FACTOR;

/// Agents move through the world. They can be cars or pedestrians.
/// They have a position (implicit), a destination node id, and a path to follow.
#[derive(Component, Debug)]
pub struct Agent {
    /// The type of agent
    pub agent_type: AgentType,

    /// The destination node index (in the graph) of the agent
    pub destination: NodeIndex,

    /// The path the agent is following
    pub path: Vec<NodeIndex>,

    /// What the next node in the path is
    pub path_index: usize,

    /// Next location and the road type cached
    pub next_path_location_road: Option<(Vec3, RoadType)>,
}

/// Note could be made more efficient by caching destination locations and only updating when needed.
pub fn update_agents(
    time: Res<Time>,
    mut agents: Query<(Entity, &mut Agent, &mut Transform)>,
    traffic_graph: Res<TrafficGraph>,
) {
    for (_, mut agent, mut transform) in agents.iter_mut() {
        // If the agent has reached the destination, get a new path and reset
        if agent.path_index >= agent.path.len() - 1 {
            // Reverse the path to get the path from end to start
            agent.destination = agent.path[0];
            agent.path.reverse();

            // Reset index
            agent.path_index = 0;

            continue;
        }

        let current_agent_location = transform.translation;

        if agent.next_path_location_road.is_none() {
            // Get the next node in the path
            let current_node = agent.path[agent.path_index];
            let next_node = agent.path[agent.path_index + 1];

            // Get the road type of the road between the current node and the next node
            let road_type = traffic_graph.get_road_type(current_node, next_node);

            // Get the location of where to travel towards, next node location
            // with an offset to stay on the right side of the road

            let current_node_location = traffic_graph.get_node_location(current_node);
            let next_node_location = traffic_graph.get_node_location(next_node);
            // Do offset based on angle between current node, next node and the one after that
            // We want the cars to drive on the right side of the road, so we calculate the angle
            // and then offset the car to the right side of the road.
            let offset = match agent.path.get(agent.path_index + 2) {
                Some(next_next_node) => {
                    let next_next_location = traffic_graph.get_node_location(*next_next_node);

                    // We want the angle between the vector of current->next to next->next-next. Angle is [0, 2*PI]
                    let vector_this_next = next_node_location - current_node_location;
                    let angle = (vector_this_next)
                        .angle_between(next_next_location - next_node_location)
                        + std::f32::consts::PI / 2.0;

                    let road_width =
                        road_type_to_width(&road_type) * 0.01 as f32 * GLOBAL_SCALE_FACTOR; // TODO: account for lanes

                    // We now rotate a vector perpendicular to the current->next vector by the angle
                    let perpendicular = vec2(-vector_this_next.y, vector_this_next.x)
                        .normalize_or_zero()
                        * road_width;

                    Vec3::new(
                        perpendicular.x * angle.cos(),
                        0.0,
                        perpendicular.y * angle.sin(),
                    )
                }
                None => Vec3::ZERO,
            };

            // Cache location and road type so we do not have to query graph again next time
            agent.next_path_location_road = Some((
                Vec3::new(next_node_location.x, 0.0, next_node_location.y) + offset,
                road_type,
            ));
        }

        // Get cached location
        let cached = agent.next_path_location_road.unwrap_throw();
        let next_location = cached.0;
        let road_type = cached.1;

        // Calculate the direction the agent should move in
        let direction = (next_location - current_agent_location).normalize();

        // Get appropriate speed for the agent based on road type
        let speed = agent_speed_on_road_type(REFERENCE_SPEED, agent.agent_type, road_type);

        // Move the agent towards the next node
        transform.translation += direction * speed * time.delta_seconds();

        // Update rotation towards direction (linear interpolation)
        let rotation = transform.rotation;
        let target_rotation = Quat::from_rotation_y(direction.x.atan2(direction.z));
        transform.rotation = rotation.slerp(target_rotation, (time.delta_seconds() * 3.0).min(1.0));

        // If the agent has reached the next node, move to the next node in the path
        if (transform.translation - next_location).length() < speed * time.delta_seconds() {
            // Update index
            agent.path_index += 1;
            // Reset cached location
            agent.next_path_location_road = None;
        }
    }
}

/// Adds a number of agents to the world, starting at a random node going towards a random node.
pub fn create_agents(
    number_of_agents: i32,
    traffic_graph: Arc<TrafficGraph>,
) -> Vec<(Vec3, Agent)> {
    let mut agents = Vec::new();

    for _ in 0..number_of_agents {
        let start_node = traffic_graph.get_random_node_index();
        let end_node = traffic_graph.get_random_node_index();

        // 50% chance of being a pedestrian or car
        let agent_type = if rand::random::<f32>() < PEDESTRIAN_CAR_SPLIT {
            AgentType::Car
        } else {
            AgentType::Pedestrian
        };

        let maybe_path: Option<Vec<NodeIndex>> =
            traffic_graph.get_shortest_path(start_node, end_node, agent_type);

        if maybe_path.is_none() {
            // Should only very rarely happen, start and end are in different connected components
            continue;
        }
        let path = maybe_path.unwrap_throw();

        let location_2d = traffic_graph.get_node_location(start_node);
        let location = Vec3::new(location_2d.x, 0.0, location_2d.y);

        let agent = Agent {
            agent_type: agent_type,
            destination: end_node,
            path: path,
            path_index: 0,
            next_path_location_road: None,
        };

        agents.push((location, agent));
    }

    agents
}

#[derive(Debug, Clone, Copy)]
pub enum AgentType {
    Car,
    Pedestrian,
}

/// Reference speed is the average speed of a pedestrian; about 5 km/h in real life.
/// Note that specific speeds might be in the data but we do not gather this as of now.
pub fn agent_speed_on_road_type(
    reference_speed: f32,
    agent_type: AgentType,
    road_type: RoadType,
) -> f32 {
    match agent_type {
        AgentType::Car => {
            // Reference speed is pedestrian speed (5 km/h) times a multiplier based on road type.
            let multiplier = match road_type {
                RoadType::Motorway => 24.0,
                RoadType::Trunk => 24.0,
                RoadType::Primary => 20.0,
                RoadType::Secondary => 16.0,
                RoadType::Tertiary => 12.0,
                RoadType::Residential => 6.0, // about 30 km/h
                RoadType::Unclassified => 6.0,
                _ => 6.0,
            };
            multiplier * reference_speed
        }
        AgentType::Pedestrian => reference_speed, // Pedestrians always move at speed 1
    }
}
