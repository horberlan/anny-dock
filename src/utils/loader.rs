use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};
use bevy::{
    log::{info, warn},
    render::texture::Image,
};

use image::io::Reader as ImageReader;
use std::path::Path;
use std::process::Command;
use xdgkit::icon_finder;

use crate::components::Favorites;
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

pub fn load_favorites() -> Favorites {
    match std::fs::read_to_string("favorites.json") {
        Ok(data) => serde_json::from_str(&data).unwrap_or_default(),
        Err(_) => Favorites::default(),
    }
}

pub fn save_favorites(favorites: &Favorites) {
    if let Ok(json) = serde_json::to_string(favorites) {
        let _ = std::fs::write("favorites.json", json);
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
}
