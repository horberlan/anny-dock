use serde::Deserialize;

#[derive(Deserialize, Default)]
pub struct DockConfig {
    pub margin_x: f32,
    pub margin_y: f32,
    pub spacing: f32,
    pub scale: f32,
}

impl Default for DockConfig {
    fn default() -> Self {
        DockConfig {
            margin_x: 20.0,
            margin_y: 20.0,
            spacing: 60.0,
            scale: 1.0,
        }
    }
} 