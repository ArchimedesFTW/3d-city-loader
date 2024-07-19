use crate::common::StatusEvent;
use crate::data::geography::Offset;
use crate::data::loading::{update_data_queries, update_query_tasks, DataQueryEvent};
use crate::data::traffic_graph::TrafficGraph;
use crate::earth::agent::update_agents;
use crate::earth::assets::setup_asset_cache;
use crate::earth::{
    setup_earth, update_agent_generation_tasks, update_building_generation_tasks, update_earth, update_river_generation_tasks, update_road_generation_tasks, update_terrain_generation_tasks, GeoDataEvent
};
use crate::lod::lod_system;
use crate::player::{setup_player, update_player, PlayerMoveEvent};
use crate::ui::{setup_ui, update_notifications, update_ui, UiState};

use crate::fps::{setup_fps, update_fps};

use bevy::prelude::*;
use bevy_mod_reqwest::ReqwestPlugin;

pub struct CityVisualizerPlugin;

impl Plugin for CityVisualizerPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(ReqwestPlugin::default())
            .add_systems(Startup, setup_asset_cache.before(setup_earth))
            .add_systems(Startup, setup_earth)
            .add_systems(Startup, setup_ui)
            .add_systems(Startup, setup_player)
            .init_resource::<TrafficGraph>()
            .add_systems(Update, update_data_queries)
            .add_event::<DataQueryEvent>()
            .add_systems(Update, update_query_tasks)
            .add_systems(Update, update_earth)
            .add_event::<GeoDataEvent>()
            .add_systems(Update, update_building_generation_tasks)
            .add_systems(Update, update_road_generation_tasks)
            .add_systems(Update, update_river_generation_tasks)
            .add_systems(Update, update_terrain_generation_tasks)
            .add_systems(Update, update_agent_generation_tasks)
            .add_systems(Update, update_ui)
            .add_systems(Update, update_agents)
            .add_event::<StatusEvent>()
            .init_resource::<UiState>()
            .add_systems(Update, update_notifications)
            .add_systems(Update, update_player)
            .add_systems(Update, lod_system)
            .add_event::<PlayerMoveEvent>()
            .add_systems(Startup, setup_fps)
            .add_systems(Update, update_fps)
            .init_resource::<Offset>();
    }
}
