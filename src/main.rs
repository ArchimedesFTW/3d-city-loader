
use bevy::asset::AssetMetaCheck;
use city_visualizer::plugin::CityVisualizerPlugin;

use bevy::DefaultPlugins;
use bevy::app::App;

// for FPS counter, frame time
use bevy::diagnostic::FrameTimeDiagnosticsPlugin;

// for gui
use bevy_egui::EguiPlugin;

fn main() {
    App::new()
        .insert_resource(AssetMetaCheck::Never) // For web https://github.com/bevyengine/bevy/issues/10157
        .add_plugins(DefaultPlugins)
        .add_plugins(CityVisualizerPlugin)
        .add_plugins(EguiPlugin)
        .add_plugins(FrameTimeDiagnosticsPlugin::default())
        .run();
}
