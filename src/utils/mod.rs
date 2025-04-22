pub use loader::*;
pub mod hover;
pub mod loader;
use bevy::log::{error, info};
use bevy::math::{Vec2, Vec3};
use std::fs;
use std::process::Command;

pub fn launch_application(class: &str) {
    if let Some(exec) = find_exec_for_class(class) {
        let exec = exec.split_whitespace().next().unwrap_or(&exec);
        match Command::new(exec).spawn() {
            Ok(_) => info!("launching: {}", exec),
            Err(e) => error!("{}: {}", exec, e),
        }
    } else {
        match Command::new(class).spawn() {
            Ok(_) => info!("(fallback): {}", class),
            Err(e) => error!("{} (fallback): {}", class, e),
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
    for line in content.lines() {
        let line = line.trim();
        if line.starts_with('[') {
            in_desktop_entry = line == "[Desktop Entry]";
        } else if in_desktop_entry {
            if let Some((key, value)) = line.split_once('=') {
                if key.trim() == "Exec" {
                    return Some(value.trim().to_string());
                }
            }
        }
    }
    None
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

pub struct DockConfig {
    pub margin_x: f32,
    pub margin_y: f32,
    pub spacing: f32,
    pub z_spacing: f32,
    pub base_scale: f32,
    pub scale_factor: f32,
}

impl Default for DockConfig {
    fn default() -> Self {
        Self {
            margin_x: 50.0,
            margin_y: 50.0,
            spacing: 40.0,
            z_spacing: 2.0,
            base_scale: 1.2,
            scale_factor: 0.9,
        }
    }
}

pub fn calculate_icon_transform(
    index: usize,
    start_pos: Vec2,
    direction: Vec2,
    config: &DockConfig,
) -> (Vec3, f32) {
    let offset = direction * (index as f32 * config.spacing);
    let pos = start_pos + offset;
    let x = pos.x;
    let y = pos.y;
    let z = -(index as f32 * config.z_spacing);
    let scale = config.base_scale * config.scale_factor.powi(index as i32);
    (Vec3::new(x, y, z), scale)
}
