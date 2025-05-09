pub use loader::*;
pub mod hover;
pub mod loader;
pub mod config;

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

#[derive(Resource)]
pub struct IconAnimationState {
    pub is_scrolling: bool,
    pub scroll_timer: Timer,
}

impl Default for IconAnimationState {
    fn default() -> Self {
        Self {
            is_scrolling: false,
            scroll_timer: Timer::from_seconds(0.3, TimerMode::Once),
        }
    }
}

pub fn calculate_icon_transform(
    index: usize,
    start_pos: Vec2,
    direction: Vec2,
    config: &DockConfig,
    scroll_offset: Vec2,
) -> (Vec3, f32) {
    let base_offset = direction * (index as f32 * config.spacing);
    let scrolled_pos = start_pos + base_offset - scroll_offset;
    
    let x = scrolled_pos.x;
    let y = scrolled_pos.y;
    let z = -(index as f32 * config.z_spacing);
    
    let base_scale = config.base_scale * config.scale_factor.powi(index as i32);
    
    let is_scrolling = scroll_offset.length() > 0.1;
    let scale = if is_scrolling {
        config.base_scale
    } else {
        base_scale
    };
    
    (Vec3::new(x, y, z), scale)
}

