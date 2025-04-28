use bevy::{
    asset::{AssetServer, Assets},
    core::Name,
    ecs::{entity::Entity, system::Commands},
    log::error,
    render::{color::Color, texture::Image},
    sprite::{Sprite, SpriteBundle},
    transform::components::Transform,
    utils::default,
};
use std::path::Path;

use crate::{
    utils::{get_icon_path, hover::HoverState, load_icon},
    ClientClass, ClientIcon, HoverTarget,
};

pub(crate) fn spawn_icon_entity(
    commands: &mut Commands,
    images: &mut Assets<Image>,
    _asset_server: &AssetServer,
    class: &str,
    transform: Transform,
    scale: f32,
    alpha: f32,
    index: usize,
) -> Entity {
    let icon_path = get_icon_path(class);
    let path = Path::new(&icon_path);
    if let Some(img) = load_icon(path) {
        let handle = images.add(img);
        let color = Color::rgba(1.0, 1.0, 1.0, alpha);
        commands
            .spawn(SpriteBundle {
                texture: handle,
                transform,
                sprite: Sprite { color, ..default() },
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
    } else {
        error!("Failed to load icon for {}", class);
        commands.spawn_empty().id()
    }
}