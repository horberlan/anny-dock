use serde::Deserialize;
use bevy::prelude::Resource;

#[derive(Resource)]
pub struct DockConfig {
    pub margin_x: f32,
    pub margin_y: f32,
    pub spacing: f32,
    pub z_spacing: f32,
    pub base_scale: f32,
    pub scale_factor: f32,
    pub scroll_speed: f32,
    pub visible_items: usize,
    pub tilt_y: f32,
}

impl Default for DockConfig {
    fn default() -> Self {
        Self {
            margin_x: 85.0,
            margin_y: 50.0,
            spacing: 40.0,
            z_spacing: 2.0,
            base_scale: 1.2,
            scale_factor: 0.9,
            scroll_speed: 15.0,
            visible_items: 8,
            tilt_y: 0.25,
        }
    }
}