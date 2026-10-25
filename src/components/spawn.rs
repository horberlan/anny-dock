use bevy::{
    asset::{AssetServer, Assets},
    core::Name,
    ecs::{entity::Entity, system::Commands},
    log::error,
    math::Vec2,
    render::{
        color::Color,
        render_resource::{Extent3d, TextureDimension, TextureFormat},
        texture::Image,
    },
    sprite::{Sprite, SpriteBundle},
    transform::components::Transform,
    utils::default,
};
use std::path::Path;

use crate::{
    config::Config,
    utils::{get_icon_path, hover::HoverState, icon_texture_size, image_from_pixmap, load_icon},
    ClientClass, ClientIcon, HoverTarget,
};

static FALLBACK_ICON_SVG: &[u8] = include_bytes!("../../assets/icons/dock_icon.svg");

fn load_svg_from_bytes(svg_bytes: &[u8], target_size: u32) -> Option<Image> {
    use resvg::render;
    use tiny_skia::Pixmap;
    use usvg::{Options, Tree};

    let opts = Options::default();
    let tree = match Tree::from_data(svg_bytes, &opts) {
        Ok(tree) => tree,
        Err(e) => {
            error!("Failed to parse SVG: {}", e);
            return None;
        }
    };

    let orig_size = tree.size.to_screen_size();
    let orig_width = orig_size.width() as f32;
    let orig_height = orig_size.height() as f32;

    let scale_factor = if orig_width > orig_height {
        target_size as f32 / orig_width
    } else {
        target_size as f32 / orig_height
    };

    let final_width = (orig_width * scale_factor).ceil() as u32;
    let final_height = (orig_height * scale_factor).ceil() as u32;

    let mut pixmap = match Pixmap::new(final_width, final_height) {
        Some(p) => p,
        _ => {
            error!("Failed to create pixmap");
            return None;
        }
    };

    if render(
        &tree,
        usvg::FitTo::Size(final_width, final_height),
        tiny_skia::Transform::default(),
        pixmap.as_mut(),
    )
    .is_none()
    {
        error!("Failed to render SVG to pixmap");
        return None;
    }

    Some(image_from_pixmap(&pixmap))
}

pub(crate) fn spawn_icon_entity(
    commands: &mut Commands,
    images: &mut Assets<Image>,
    _asset_server: &AssetServer,
    class: &str,
    transform: Transform,
    scale: f32,
    alpha: f32,
    index: usize,
    config: &Config,
    window_scale_factor: f32,
) -> Entity {
    let icon_path = get_icon_path(class);
    let texture_size = icon_texture_size(config, window_scale_factor);

    let handle = if icon_path == "memory://fallback_icon" {
        if let Some(img) = load_svg_from_bytes(FALLBACK_ICON_SVG, texture_size) {
            images.add(img)
        } else {
            error!("Failed to render fallback SVG icon!");
            let img = Image::new_fill(
                Extent3d {
                    width: 56,
                    height: 56,
                    depth_or_array_layers: 1,
                },
                TextureDimension::D2,
                &[255, 0, 0, 255],
                TextureFormat::Rgba8Unorm,
            );
            images.add(img)
        }
    } else {
        let path = Path::new(&icon_path);
        if let Some(img) = load_icon(path, texture_size) {
            images.add(img)
        } else {
            error!("Failed to load icon for {}, using fallback", class);
            if let Some(img) = load_svg_from_bytes(FALLBACK_ICON_SVG, texture_size) {
                images.add(img)
            } else {
                let img = Image::new_fill(
                    Extent3d {
                        width: 56,
                        height: 56,
                        depth_or_array_layers: 1,
                    },
                    TextureDimension::D2,
                    &[255, 0, 0, 255],
                    TextureFormat::Rgba8Unorm,
                );
                images.add(img)
            }
        }
    };

    let color = Color::rgba(1.0, 1.0, 1.0, alpha);
    commands
        .spawn(SpriteBundle {
            texture: handle,
            transform,
            // Textures are rasterized above display size; pin the world size so
            // any texture resolution renders at `icon_size` before scaling.
            sprite: Sprite {
                color,
                custom_size: Some(Vec2::splat(config.icon_size)),
                ..default()
            },
            ..default()
        })
        .insert(ClientIcon)
        .insert(ClientClass(class.to_string()))
        .insert(HoverTarget {
            original_position: transform.translation.truncate(),
            original_z: transform.translation.z,
            original_scale: scale,
            index,
            is_hovered: false,
            hover_exit_timer: None,
        })
        .insert(HoverState::default())
        .insert(Name::new(class.to_string()))
        .id()
}
