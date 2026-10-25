use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};
use bevy::{
    log::{info, warn},
    render::texture::Image,
};

use image::io::Reader as ImageReader;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use xdgkit::icon_finder;

use crate::components::Favorites;
use crate::types::HyprIpcEvent;
use crate::Client;


pub fn get_current_clients() -> Result<Vec<Client>, std::io::Error> {
    let output = Command::new("hyprctl").args(["clients", "-j"]).output()?;

    if !output.status.success() {
        return Err(std::io::Error::new(
            std::io::ErrorKind::Other,
            "Failed to execute hyprctl",
        ));
    }

    let clients: Vec<Client> = serde_json::from_slice(&output.stdout)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;

    Ok(clients)
}

/// Parses a line from Hyprland's `.socket2.sock`. Events carry window addresses
/// without the `0x` prefix that `hyprctl clients -j` uses, so they are
/// normalized to match the addresses loaded at startup.
pub fn parse_hypr_event(line: &str) -> Option<HyprIpcEvent> {
    if let Some(rest) = line.strip_prefix("openwindow>>") {
        let mut parts = rest.splitn(4, ',');
        let (address, workspace, class, title) =
            (parts.next()?, parts.next()?, parts.next()?, parts.next()?);
        Some(HyprIpcEvent::OpenWindow {
            address: normalize_address(address),
            workspace: workspace.to_string(),
            class: class.to_string(),
            title: title.to_string(),
        })
    } else if let Some(rest) = line.strip_prefix("closewindow>>") {
        Some(HyprIpcEvent::CloseWindow {
            address: normalize_address(rest.trim()),
        })
    } else {
        Some(HyprIpcEvent::Other)
    }
}

fn normalize_address(address: &str) -> String {
    if address.starts_with("0x") {
        address.to_string()
    } else {
        format!("0x{address}")
    }
}

/// Older versions kept favorites in the working directory, so they depended on
/// where the dock was launched from.
const LEGACY_FAVORITES_PATH: &str = "favorites.json";

fn favorites_path() -> Option<PathBuf> {
    let dir = dirs::config_dir()?.join("anny-dock");
    fs::create_dir_all(&dir).ok()?;
    Some(dir.join("favorites.json"))
}

pub fn load_favorites() -> Favorites {
    match favorites_path() {
        Some(path) => load_favorites_from(&path, Path::new(LEGACY_FAVORITES_PATH)),
        None => read_favorites(Path::new(LEGACY_FAVORITES_PATH)).unwrap_or_default(),
    }
}

pub fn save_favorites(favorites: &Favorites) {
    if let Some(path) = favorites_path() {
        save_favorites_to(&path, favorites);
    }
}

/// Reads `path`; when it does not exist yet, copies the legacy file there
/// (leaving the legacy file in place).
fn load_favorites_from(path: &Path, legacy_path: &Path) -> Favorites {
    if path.exists() {
        return read_favorites(path).unwrap_or_default();
    }
    match read_favorites(legacy_path) {
        Some(favorites) => {
            info!("Migrating favorites from {:?} to {:?}", legacy_path, path);
            save_favorites_to(path, &favorites);
            favorites
        }
        None => Favorites::default(),
    }
}

fn read_favorites(path: &Path) -> Option<Favorites> {
    serde_json::from_str(&fs::read_to_string(path).ok()?).ok()
}

fn save_favorites_to(path: &Path, favorites: &Favorites) {
    if let Some(parent) = path.parent() {
        let _ = fs::create_dir_all(parent);
    }
    if let Ok(json) = serde_json::to_string(favorites) {
        if let Err(e) = fs::write(path, json) {
            warn!("Failed to save favorites to {:?}: {:?}", path, e);
        }
    }
}

pub fn get_icon_path(class: &str) -> String {
    let lowercase = class.to_lowercase();
    match icon_finder::find_icon(lowercase, 56, 1) {
        Some(path) => {
            info!(
                "icon found for {}: {}",
                class,
                path.to_string_lossy().to_string()
            );
            path.to_string_lossy().to_string()
        }
        _ => {
            warn!("No icons found for {}, using fallback", class);
            "memory://fallback_icon".to_string()
        }
    }
}

pub fn load_icon(path: &Path, size: u32) -> Option<Image> {
    if let Some(ext) = path.extension() {
        if ext == "svg" {
            return load_svg_image(path, size);
        }
    }

    if let Ok(reader) = ImageReader::open(path) {
        if let Ok(img) = reader.decode() {
            let rgba_img = img.to_rgba8();
            let (width, height) = rgba_img.dimensions();
            let data = rgba_img.into_raw();

            let image = Image::new_fill(
                Extent3d {
                    width,
                    height,
                    depth_or_array_layers: 1,
                },
                TextureDimension::D2,
                &data,
                TextureFormat::Rgba8UnormSrgb,
            );
            return Some(image);
        }
    }

    None
}

pub fn load_svg_image(path: &Path, size: u32) -> Option<Image> {
    let svg_data = std::fs::read(path).ok()?;
    let opt = usvg::Options::default();
    let tree = usvg::Tree::from_data(&svg_data, &opt).ok()?;

    let mut pixmap = tiny_skia::Pixmap::new(size, size)?;
    resvg::render(
        &tree,
        usvg::FitTo::Size(size, size),
        tiny_skia::Transform::default(),
        pixmap.as_mut(),
    )?;

    Some(image_from_pixmap(&pixmap))
}

/// tiny-skia stores premultiplied RGBA, while Bevy sprites blend with straight
/// alpha; uploading it as-is darkens antialiased edges.
pub fn image_from_pixmap(pixmap: &tiny_skia::Pixmap) -> Image {
    let data: Vec<u8> = pixmap
        .pixels()
        .iter()
        .flat_map(|pixel| {
            let color = pixel.demultiply();
            [color.red(), color.green(), color.blue(), color.alpha()]
        })
        .collect();

    Image::new(
        Extent3d {
            width: pixmap.width(),
            height: pixmap.height(),
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        data,
        TextureFormat::Rgba8UnormSrgb,
    )
}

pub fn load_clients() -> Vec<Client> {
    get_current_clients().unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn image_from_pixmap_stores_straight_alpha_srgb() {
        let mut pixmap = tiny_skia::Pixmap::new(1, 1).unwrap();
        pixmap.fill(tiny_skia::Color::from_rgba8(255, 0, 0, 128));

        let image = image_from_pixmap(&pixmap);

        assert_eq!(&image.data[..4], &[255, 0, 0, 128]);
        assert_eq!(image.texture_descriptor.format, TextureFormat::Rgba8UnormSrgb);
    }

    #[test]
    fn open_window_event_uses_hyprctl_address_format() {
        let event = parse_hypr_event("openwindow>>555f420d44a0,1,firefox,Feed, Inbox — Firefox");

        match event {
            Some(HyprIpcEvent::OpenWindow { address, workspace, class, title }) => {
                assert_eq!(address, "0x555f420d44a0");
                assert_eq!(workspace, "1");
                assert_eq!(class, "firefox");
                assert_eq!(title, "Feed, Inbox — Firefox");
            }
            other => panic!("unexpected event: {other:?}"),
        }
    }

    #[test]
    fn close_window_event_uses_hyprctl_address_format() {
        match parse_hypr_event("closewindow>>555f420d44a0") {
            Some(HyprIpcEvent::CloseWindow { address }) => assert_eq!(address, "0x555f420d44a0"),
            other => panic!("unexpected event: {other:?}"),
        }
    }

    #[test]
    fn prefixed_address_is_kept() {
        match parse_hypr_event("closewindow>>0x555f420d44a0") {
            Some(HyprIpcEvent::CloseWindow { address }) => assert_eq!(address, "0x555f420d44a0"),
            other => panic!("unexpected event: {other:?}"),
        }
    }

    #[test]
    fn unknown_and_malformed_events() {
        assert!(matches!(parse_hypr_event("workspace>>2"), Some(HyprIpcEvent::Other)));
        assert!(parse_hypr_event("openwindow>>555f420d44a0,1").is_none());
    }

    fn favorites(classes: &[&str]) -> Favorites {
        Favorites(classes.iter().map(|c| c.to_string()).collect())
    }

    #[test]
    fn favorites_load_from_config_file() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("config/favorites.json");
        let legacy = dir.path().join("favorites.json");
        save_favorites_to(&path, &favorites(&["kitty", "firefox"]));
        save_favorites_to(&legacy, &favorites(&["legacy"]));

        assert_eq!(load_favorites_from(&path, &legacy).0, ["kitty", "firefox"]);
    }

    #[test]
    fn legacy_favorites_are_migrated_when_config_file_is_missing() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("config/favorites.json");
        let legacy = dir.path().join("favorites.json");
        save_favorites_to(&legacy, &favorites(&["firefox"]));

        assert_eq!(load_favorites_from(&path, &legacy).0, ["firefox"]);
        assert_eq!(read_favorites(&path).unwrap().0, ["firefox"]);
        assert!(legacy.exists(), "legacy file must not be deleted");
    }

    #[test]
    fn invalid_config_file_does_not_resurrect_legacy_favorites() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("favorites.json.new");
        let legacy = dir.path().join("favorites.json");
        std::fs::write(&path, "not json").unwrap();
        save_favorites_to(&legacy, &favorites(&["legacy"]));

        assert!(load_favorites_from(&path, &legacy).0.is_empty());
    }

    #[test]
    fn no_favorites_anywhere_is_empty() {
        let dir = tempfile::tempdir().unwrap();
        let favorites = load_favorites_from(&dir.path().join("a.json"), &dir.path().join("b.json"));

        assert!(favorites.0.is_empty());
    }
}
