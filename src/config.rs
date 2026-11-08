use bevy::log::warn;
use bevy::prelude::Resource;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Resource, Serialize, Deserialize, Debug, Clone)]
#[serde(default)]
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
    /// Kept last: TOML writes tables after plain values.
    pub layer_shell: LayerShellConfig,
}

/// How the dock's layer surface is placed (ignored when running as a window).
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Default)]
#[serde(default)]
pub struct LayerShellConfig {
    pub layer: DockLayer,
    /// Monitor name as listed by `hyprctl monitors` (e.g. "eDP-1"); empty lets
    /// the compositor choose, usually the focused monitor.
    pub output: String,
    pub keyboard: DockKeyboard,
}

/// Stacking layer: `top` sits above windows, `overlay` also above fullscreen.
#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum DockLayer {
    Background,
    Bottom,
    #[default]
    Top,
    Overlay,
}

/// Keyboard focus: `on_demand` when the dock is clicked, `exclusive` grabs it
/// while the dock runs, `none` disables the dock's shortcuts.
#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum DockKeyboard {
    None,
    Exclusive,
    #[default]
    OnDemand,
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
            layer_shell: LayerShellConfig::default(),
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
    match get_config_path() {
        Some(path) => load_config_from(&path),
        None => Config::default(),
    }
}

/// Reads `path`, creating it with the defaults when missing. An unreadable or
/// invalid file falls back to the defaults and is left untouched, so a typo
/// never costs the user their settings.
fn load_config_from(path: &Path) -> Config {
    if !path.exists() {
        let config = Config::default();
        match toml::to_string_pretty(&config) {
            Ok(toml_string) => {
                if let Err(e) = fs::write(path, toml_string) {
                    warn!("Failed to create config file {:?}: {}", path, e);
                }
            }
            Err(e) => warn!("Failed to serialize default config: {}", e),
        }
        return config;
    }
    match fs::read_to_string(path).map_err(|e| e.to_string()).and_then(|contents| {
        parse_config(&contents).map_err(|e| e.to_string())
    }) {
        Ok(config) => config,
        Err(e) => {
            warn!("Invalid config file {:?}, using defaults: {}", path, e);
            Config::default()
        }
    }
}

/// Fields missing from the file take their default value (`#[serde(default)]`),
/// so configs written by older versions keep working.
fn parse_config(contents: &str) -> Result<Config, toml::de::Error> {
    toml::from_str(contents)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn missing_fields_take_default_values() {
        // A config written before `tilt_y` existed.
        let config = parse_config("icon_size = 64.0\nmargin_x = 10.0\n").unwrap();

        assert_eq!(config.icon_size, 64.0);
        assert_eq!(config.margin_x, 10.0);
        assert_eq!(config.tilt_y, Config::default().tilt_y);
        assert_eq!(config.visible_items, Config::default().visible_items);
    }

    #[test]
    fn empty_file_is_the_default_config() {
        let config = parse_config("").unwrap();

        assert_eq!(config.icon_size, Config::default().icon_size);
    }

    #[test]
    fn invalid_file_falls_back_to_defaults_without_overwriting_it() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("config.toml");
        fs::write(&path, "icon_size = \"big\"").unwrap();

        let config = load_config_from(&path);

        assert_eq!(config.icon_size, Config::default().icon_size);
        assert_eq!(fs::read_to_string(&path).unwrap(), "icon_size = \"big\"");
    }

    #[test]
    fn missing_file_is_created_with_defaults() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("config.toml");

        let config = load_config_from(&path);

        assert_eq!(config.icon_size, Config::default().icon_size);
        assert_eq!(parse_config(&fs::read_to_string(&path).unwrap()).unwrap().icon_size, config.icon_size);
    }

    #[test]
    fn layer_shell_defaults() {
        let config = parse_config("").unwrap();

        assert_eq!(config.layer_shell.layer, DockLayer::Top);
        assert_eq!(config.layer_shell.keyboard, DockKeyboard::OnDemand);
        assert_eq!(config.layer_shell.output, "");
    }

    #[test]
    fn layer_shell_section_is_parsed() {
        let config = parse_config(
            "icon_size = 50.0\n[layer_shell]\nlayer = \"overlay\"\noutput = \"HDMI-A-1\"\nkeyboard = \"none\"\n",
        )
        .unwrap();

        assert_eq!(config.layer_shell.layer, DockLayer::Overlay);
        assert_eq!(config.layer_shell.output, "HDMI-A-1");
        assert_eq!(config.layer_shell.keyboard, DockKeyboard::None);
    }

    #[test]
    fn unknown_layer_is_rejected() {
        assert!(parse_config("[layer_shell]\nlayer = \"floating\"\n").is_err());
    }

    #[test]
    fn default_config_round_trips_through_toml() {
        let written = toml::to_string_pretty(&Config::default()).unwrap();

        assert_eq!(parse_config(&written).unwrap().layer_shell, LayerShellConfig::default());
    }
}
