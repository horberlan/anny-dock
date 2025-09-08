use bevy::prelude::Resource;
use serde::{Deserialize, Serialize};
use std::fs;
use std::io::{Read, Write};
use std::path::PathBuf;

#[derive(Resource, Serialize, Deserialize, Debug, Clone)]
pub struct Config {
    pub icon_size: f32,
    pub margin_x: f32,
    pub margin_y: f32,
    pub spacing: f32,
    pub z_spacing: f32,
    pub base_scale: f32,
    pub scale_factor: f32,
    pub scroll_speed: f32,
    pub visible_items: usize,
    pub tilt_y: f32,
    pub font_size: f32,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            icon_size: 56.0,
            margin_x: 85.0,
            margin_y: 50.0,
            spacing: 40.0,
            z_spacing: 2.0,
            base_scale: 1.2,
            scale_factor: 0.9,
            scroll_speed: 15.0,
            visible_items: 8,
            tilt_y: 0.25,
            font_size: 16.0,
        }
    }
}

fn get_config_path() -> Option<PathBuf> {
    dirs::config_dir()
        .map(|mut path| {
            path.push("anny-dock");
            fs::create_dir_all(&path).ok()?;
            path.push("config.toml");
            Some(path)
        })
        .flatten()
}

pub fn load_config() -> Config {
    if let Some(path) = get_config_path() {
        if path.exists() {
            let mut file = fs::File::open(path).expect("Failed to open config file");
            let mut contents = String::new();
            file.read_to_string(&mut contents)
                .expect("Failed to read config file");
            toml::from_str(&contents).expect("Failed to parse config file")
        } else {
            let config = Config::default();
            let toml_string =
                toml::to_string_pretty(&config).expect("Failed to serialize config");
            let mut file = fs::File::create(path).expect("Failed to create config file");
            file.write_all(toml_string.as_bytes())
                .expect("Failed to write to config file");
            config
        }
    } else {
        Config::default()
    }
}
