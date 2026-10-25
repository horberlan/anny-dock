pub use loader::*;
pub mod hover;
pub mod loader;

use crate::config::Config;
use bevy::log::{error, info, warn};
use bevy::math::{Vec2, Vec3};
use std::fs;
use std::process::Command;
use bevy::prelude::*;

pub fn launch_application(class: &str) {
    match find_exec_for_class(class) {
        Some(exec) => {
            info!("Found executable: {}", exec);
            let clean_exec = exec.split_whitespace()
                .take_while(|&part| !part.starts_with('%'))
                .collect::<Vec<_>>()
                .join(" ");
                
            info!("Launching with cleaned exec: {}", clean_exec);
            
            let direct_output = Command::new("sh")
                .arg("-c")
                .arg(&clean_exec)
                .spawn();

            match direct_output {
                Ok(_) => {
                    info!("Successfully launched application: {}", class);
                }
                Err(e) => {
                    error!("Failed to launch directly, trying fallback: {:?}", e);
                    let fallback = Command::new("hyprctl")
                        .args(["dispatch", "exec", class])
                        .spawn();
                        
                    match fallback {
                        Ok(_) => info!("Successfully launched via hyprctl: {}", class),
                        Err(e) => error!("All launch attempts failed: {:?}", e),
                    }
                }
            }
        }
        _ => {
            warn!("No executable found for class: {}, trying direct launch", class);
            let output = Command::new("hyprctl")
                .args(["dispatch", "exec", class])
                .spawn();

            match output {
                Ok(_) => info!("Successfully launched via hyprctl: {}", class),
                Err(e) => error!("Failed to launch application: {:?}", e),
            }
        }
    }
}

fn find_exec_for_class(class: &str) -> Option<String> {
    // refator needed
    let binds = dirs::data_local_dir()?.join("applications/");
    let dirs = [
        "/usr/share/applications/",
        "/usr/local/share/applications/",
        binds.to_str()?,
    ];

    for dir in dirs.iter() {
        let path = std::path::Path::new(dir).join(format!("{class}.desktop"));
        if path.exists() {
            if let Ok(content) = fs::read_to_string(&path) {
                if let Some(exec) = parse_exec_only(&content) {
                    return Some(exec);
                }
            }
        }
    }

    for dir in dirs.iter() {
        if let Ok(entries) = fs::read_dir(dir) {
            for entry in entries.flatten() {
                let file_path = entry.path();
                if file_path.extension().and_then(|s| s.to_str()) == Some("desktop") {
                    if let Ok(content) = fs::read_to_string(&file_path) {
                        if let Some(exec) = parse_desktop_file(&content, class) {
                            return Some(exec);
                        }
                    }
                }
            }
        }
    }

    None
}

fn parse_exec_only(content: &str) -> Option<String> {
    let mut in_desktop_entry = false;
    let mut exec = None;
    let mut no_display = false;
    let mut terminal = false;

    for line in content.lines() {
        let line = line.trim();
        if line.starts_with('[') {
            in_desktop_entry = line == "[Desktop Entry]";
        } else if in_desktop_entry {
            if let Some((key, value)) = line.split_once('=') {
                match key.trim() {
                    "Exec" => exec = Some(value.trim().to_string()),
                    "NoDisplay" => no_display = value.trim().to_lowercase() == "true",
                    "Terminal" => terminal = value.trim().to_lowercase() == "true",
                    _ => {}
                }
            }
        }
    }

    if no_display || terminal {
        return None;
    }

    exec
}

fn parse_desktop_file(content: &str, class: &str) -> Option<String> {
    let mut in_desktop_entry = false;
    let mut startup_wm_class = None;
    let mut exec = None;

    for line in content.lines() {
        let line = line.trim();
        if line.starts_with('[') {
            in_desktop_entry = line == "[Desktop Entry]";
        } else if in_desktop_entry {
            if let Some((key, value)) = line.split_once('=') {
                match key.trim() {
                    "StartupWMClass" => startup_wm_class = Some(value.trim()),
                    "Exec" => exec = Some(value.trim()),
                    _ => {}
                }
            }
        }
    }

    if startup_wm_class == Some(class) {
        exec.map(|s| s.to_string())
    } else {
        None
    }
}

#[derive(Resource)]
pub struct IconAnimationState {
    pub _is_scrolling: bool,
    pub _scroll_timer: Timer,
}

impl Default for IconAnimationState {
    fn default() -> Self {
        Self {
            _is_scrolling: false,
            _scroll_timer: Timer::from_seconds(0.3, TimerMode::Once),
        }
    }
}

pub fn calculate_icon_transform(
    index: usize,
    start_pos: Vec2,
    direction: Vec2,
    config: &Config,
    scroll_offset: Vec2,
) -> (Vec3, f32) {
    let scale_dampening = 0.4;
    let r = config.scale_factor + (1.0 - config.scale_factor) * scale_dampening;

    let spacing_boost = 1.2;
    let i = index as f32;

    let total_spacing_multiplier = if (r - 1.0).abs() < f32::EPSILON {
        i
    } else {
        (1.0 - r.powf(i)) / (1.0 - r)
    };

    let base_offset =
        direction * (total_spacing_multiplier * config.spacing * spacing_boost * config.base_scale);
    let scrolled_pos = start_pos + base_offset - scroll_offset;

    let x = scrolled_pos.x;
    let y = scrolled_pos.y;
    let z = -(index as f32 * config.z_spacing);

    let base_scale = config.base_scale * r.powi(index as i32);

    let is_scrolling = scroll_offset.length() > 0.1;
    let scale = if is_scrolling {
        config.base_scale
    } else {
        base_scale
    };

    (Vec3::new(x, y, z), scale)
}

/// Start position of the first icon and the unit direction the dock grows in.
pub fn dock_axis(window: &Window, config: &Config) -> (Vec2, Vec2) {
    let start_pos = Vec2::new(
        -window.width() / 2.0 + config.margin_x,
        -window.height() / 2.0 + config.margin_y,
    );
    let center = Vec2::new(0.0, window.height() * config.tilt_y);
    (start_pos, (center - start_pos).normalize_or_zero())
}

/// Largest multiplier applied on top of `base_scale`: the first visible item in
/// `hover_animation_system` (1.2). Hover (1.15) and drag (1.1) stay below it.
const MAX_RELATIVE_SCALE: f32 = 1.2;

/// Largest `Transform` scale an icon (and its title) can reach on screen.
pub fn max_icon_display_scale(config: &Config) -> f32 {
    config.base_scale * MAX_RELATIVE_SCALE
}

/// Texture side, in physical pixels, needed to draw an icon at its largest
/// display size without upscaling.
pub fn icon_texture_size(config: &Config, window_scale_factor: f32) -> u32 {
    (config.icon_size * max_icon_display_scale(config) * window_scale_factor).ceil() as u32
}

/// Text2d glyphs ignore `Transform.scale`, so titles are rasterized at the
/// largest display scale and only ever scaled down.
pub fn title_transform_scale(icon_scale: f32, config: &Config) -> f32 {
    icon_scale / max_icon_display_scale(config)
}

pub fn update_sprite_alpha(sprite: &mut Sprite, is_pinned: bool, is_running: bool) {
    let alpha = if is_pinned && !is_running {
        0.5
    } else {
        1.0
    };
    sprite.color.set_a(alpha);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn icon_texture_covers_largest_display_size() {
        let config = Config::default();
        let largest = config.icon_size * max_icon_display_scale(&config);
        let size = icon_texture_size(&config, 1.0) as f32;
        assert!(size >= largest, "texture {size} smaller than display {largest}");
        assert!(size < largest + 1.0, "texture {size} oversized for display {largest}");
    }

    #[test]
    fn icon_texture_follows_window_scale_factor() {
        let config = Config::default();
        let largest = config.icon_size * max_icon_display_scale(&config) * 2.0;
        assert!(icon_texture_size(&config, 2.0) as f32 >= largest);
    }

    #[test]
    fn title_is_never_upscaled() {
        let config = Config::default();
        let max = max_icon_display_scale(&config);
        assert!((title_transform_scale(max, &config) - 1.0).abs() < f32::EPSILON);
        assert!(title_transform_scale(config.base_scale, &config) < 1.0);
    }
}
