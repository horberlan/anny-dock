pub use loader::*;
pub mod hover;
pub mod loader;

use crate::config::Config;
use crate::types::Client;
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

/// Layout of the icon in `slot`: its index minus the scroll position, so 0 is
/// the first visible slot and fractional slots fall between whole ones while
/// scrolling. Spacing and scale shrink geometrically along the dock; the first
/// slot gets a size boost, and icons fade out (scale to 0) over the slot before
/// the first and the slot after the last visible one.
pub fn calculate_icon_transform(
    slot: f32,
    start_pos: Vec2,
    direction: Vec2,
    config: &Config,
) -> (Vec3, f32) {
    let scale_dampening = 0.4;
    let r = config.scale_factor + (1.0 - config.scale_factor) * scale_dampening;

    let spacing_boost = 1.2;
    let total_spacing_multiplier = if (r - 1.0).abs() < f32::EPSILON {
        slot
    } else {
        (1.0 - r.powf(slot)) / (1.0 - r)
    };

    let base_offset =
        direction * (total_spacing_multiplier * config.spacing * spacing_boost * config.base_scale);
    let position = start_pos + base_offset;
    let z = -(slot * config.z_spacing);

    let last_slot = config.visible_items as f32 - 1.0;
    let fade = (1.0 + slot).min(1.0 + last_slot - slot).clamp(0.0, 1.0);
    let scale = config.base_scale * config.scale_factor.powf(slot.max(0.0)) * front_boost(slot) * fade;

    (Vec3::new(position.x, position.y, z), scale)
}

/// Extra size of the icon in the first slot, easing out over the next slot.
pub fn front_boost(slot: f32) -> f32 {
    1.0 + (MAX_RELATIVE_SCALE - 1.0) * (1.0 - slot.abs()).clamp(0.0, 1.0)
}

/// One icon to spawn at startup.
pub struct DockEntry {
    pub class: String,
    /// The window it stands for; `None` for a favorite with no open window.
    pub client: Option<Client>,
    pub is_favorite: bool,
}

/// Startup icons, one per open window, matching how windows opened later are
/// handled. Follows `saved_order` (classes, one per icon, from the last
/// session): each entry takes the next unused window of its class, a favorite
/// with no window stays pinned in place, and closed non-favorites are skipped.
/// Favorites missing from it come next, in saved order, then every other window
/// in `hyprctl` order. The first icon of a favorite class carries the pin; its
/// other windows get plain icons.
pub fn initial_dock_entries(
    saved_order: &[String],
    favorites: &[String],
    clients: &[Client],
) -> Vec<DockEntry> {
    let mut entries: Vec<DockEntry> = Vec::new();
    let mut used_addresses: Vec<String> = Vec::new();
    let has_pin = |entries: &[DockEntry], class: &str| {
        entries.iter().any(|e| e.is_favorite && e.class == class)
    };

    let mut place = |class: &String, entries: &mut Vec<DockEntry>| {
        let is_favorite = favorites.contains(class) && !has_pin(entries, class);
        let client = clients
            .iter()
            .find(|c| c.class == *class && !used_addresses.contains(&c.address))
            .cloned();
        if let Some(client) = &client {
            used_addresses.push(client.address.clone());
        } else if !is_favorite {
            return;
        }
        entries.push(DockEntry { class: class.clone(), client, is_favorite });
    };

    for class in saved_order {
        place(class, &mut entries);
    }
    for class in favorites {
        if !has_pin(&entries, class) {
            place(class, &mut entries);
        }
    }
    // `place` takes the next unused window of the class, so walking every
    // window adds the remaining ones in `hyprctl` order.
    for client in clients {
        place(&client.class, &mut entries);
    }
    entries
}

/// Classes of the icons in `order` (dock addresses), as saved between sessions:
/// window addresses change on every session, classes do not.
pub fn dock_order_classes(order: &[String], icons: &[(String, String)]) -> Vec<String> {
    order
        .iter()
        .filter_map(|address| {
            icons
                .iter()
                .find(|(icon_address, _)| icon_address == address)
                .map(|(_, class)| class.clone())
        })
        .collect()
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

/// Largest multiplier applied on top of `base_scale`: the first slot's boost
/// (`front_boost`). Hover (1.15) and drag (1.1) stay below it.
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

    fn client(class: &str, address: &str) -> Client {
        Client { class: class.into(), address: address.into(), _name: None }
    }

    fn summary(entries: &[DockEntry]) -> Vec<(String, Option<String>, bool)> {
        entries
            .iter()
            .map(|e| (e.class.clone(), e.client.as_ref().map(|c| c.address.clone()), e.is_favorite))
            .collect()
    }

    fn entry(class: &str, address: Option<&str>, favorite: bool) -> (String, Option<String>, bool) {
        (class.into(), address.map(Into::into), favorite)
    }

    #[test]
    fn every_open_window_gets_an_icon_on_startup() {
        let clients = [
            client("kitty", "0x1"),
            client("kitty", "0x2"),
            client("firefox", "0x3"),
            client("kitty", "0x4"),
            client("code-oss", "0x5"),
        ];
        let favorites = ["code-oss".to_string(), "firefox".to_string()];

        let entries = initial_dock_entries(&[], &favorites, &clients);

        assert_eq!(
            summary(&entries),
            [
                entry("code-oss", Some("0x5"), true),
                entry("firefox", Some("0x3"), true),
                entry("kitty", Some("0x1"), false),
                entry("kitty", Some("0x2"), false),
                entry("kitty", Some("0x4"), false),
            ]
        );
    }

    fn classes(names: &[&str]) -> Vec<String> {
        names.iter().map(|n| n.to_string()).collect()
    }

    #[test]
    fn saved_order_restores_the_arrangement() {
        let clients = [
            client("kitty", "0x1"),
            client("kitty", "0x2"),
            client("firefox", "0x3"),
            client("code-oss", "0x4"),
        ];
        let saved = classes(&["kitty", "firefox", "code-oss", "kitty"]);
        let favorites = classes(&["code-oss", "firefox"]);

        let entries = initial_dock_entries(&saved, &favorites, &clients);

        assert_eq!(
            summary(&entries),
            [
                entry("kitty", Some("0x1"), false),
                entry("firefox", Some("0x3"), true),
                entry("code-oss", Some("0x4"), true),
                entry("kitty", Some("0x2"), false),
            ]
        );
    }

    #[test]
    fn saved_order_skips_closed_apps_and_appends_new_ones() {
        // slack (favorite) is closed, gimp (not favorite) is closed, mpv is new,
        // and code-oss was pinned after the order was last saved.
        let clients = [client("mpv", "0x1"), client("kitty", "0x2"), client("code-oss", "0x3")];
        let saved = classes(&["gimp", "kitty", "slack"]);
        let favorites = classes(&["slack", "code-oss"]);

        let entries = initial_dock_entries(&saved, &favorites, &clients);

        assert_eq!(
            summary(&entries),
            [
                entry("kitty", Some("0x2"), false),
                entry("slack", None, true),
                entry("code-oss", Some("0x3"), true),
                entry("mpv", Some("0x1"), false),
            ]
        );
    }

    #[test]
    fn dock_order_is_saved_as_classes() {
        let order = classes(&["0x2", "pinned:slack", "0x1", "0x9"]);
        let icons = [
            ("0x1".to_string(), "kitty".to_string()),
            ("0x2".to_string(), "firefox".to_string()),
            ("pinned:slack".to_string(), "slack".to_string()),
        ];

        // 0x9 has no icon (e.g. closed this frame) and is left out.
        assert_eq!(dock_order_classes(&order, &icons), ["firefox", "slack", "kitty"]);
    }

    #[test]
    fn favorite_without_window_is_pinned_and_extra_windows_get_their_own_icon() {
        let clients = [client("firefox", "0x1"), client("firefox", "0x2")];
        let favorites = ["slack".to_string(), "firefox".to_string(), "firefox".to_string()];

        let entries = initial_dock_entries(&[], &favorites, &clients);

        assert_eq!(
            summary(&entries),
            [
                entry("slack", None, true),
                entry("firefox", Some("0x1"), true),
                entry("firefox", Some("0x2"), false),
            ]
        );
    }

    fn layout(slot: f32) -> (Vec3, f32) {
        calculate_icon_transform(slot, Vec2::ZERO, Vec2::X, &Config::default())
    }

    #[test]
    fn first_slot_sits_at_start_with_front_boost() {
        let config = Config::default();
        let (translation, scale) = layout(0.0);

        assert_eq!(translation.truncate(), Vec2::ZERO);
        assert!((scale - config.base_scale * MAX_RELATIVE_SCALE).abs() < 1e-5);
    }

    #[test]
    fn fractional_slots_move_continuously_between_whole_slots() {
        let (one, scale_one) = layout(1.0);
        let (half, scale_half) = layout(1.5);
        let (two, scale_two) = layout(2.0);

        assert!(one.x < half.x && half.x < two.x);
        assert!(scale_one > scale_half && scale_half > scale_two);
        assert!(one.z > half.z && half.z > two.z, "front items stay on top");
    }

    #[test]
    fn icons_fade_out_beyond_the_visible_slots() {
        let config = Config::default();
        let last = config.visible_items as f32 - 1.0;

        assert_eq!(layout(-1.0).1, 0.0);
        assert_eq!(layout(last + 1.0).1, 0.0);
        assert!(layout(-0.5).1 > 0.0 && layout(-0.5).1 < layout(0.0).1);
        assert!(layout(last + 0.5).1 > 0.0 && layout(last + 0.5).1 < layout(last).1);
    }

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
