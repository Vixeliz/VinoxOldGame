use bevy::prelude::*;
use bevy_egui::EguiPlugin;
use renet_visualizer::{RenetClientVisualizer, RenetVisualizerStyle};

pub struct UiPlugin;

impl Plugin for UiPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugin(EguiPlugin)
            .insert_resource(RenetClientVisualizer::<200>::new(
                RenetVisualizerStyle::default(),
            ));
    }
}
