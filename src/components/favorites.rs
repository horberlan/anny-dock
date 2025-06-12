use bevy::prelude::{Component, Resource};
use bevy::{
    asset::AssetServer,
    ecs::{
        entity::Entity,
        system::{Commands, Res},
    },
    hierarchy::BuildChildren,
    math::Vec3,
    render::color::Color,
    text::{Text, Text2dBundle, TextAlignment, TextStyle},
    transform::components::Transform,
    utils::default,
};

use bevy_easings::{Ease, EaseFunction, EasingType};
use bevy_svg::prelude::{Origin, Svg2dBundle};
use serde::{Deserialize, Serialize};

#[derive(Component)]
struct FavoritePin;

#[derive(Resource, Deserialize, Serialize, Clone, Default)]
pub struct Favorites(pub Vec<String>);

#[derive(Component, Debug)]
pub struct Favorite;

use crate::{ClientAddress, IconText, ASSETS_ICON_PIN_PATH, ICON_SIZE};

pub(crate) fn add_client_address(commands: &mut Commands, entity: Entity, address: String) {
    commands.entity(entity).insert(ClientAddress(address));
}

pub(crate) fn add_favorite(
    commands: &mut Commands,
    entity: Entity,
    asset_server: &Res<AssetServer>,
) {
    commands.entity(entity).insert(Favorite);
    set_favorite_pin(commands, asset_server, entity);
}

pub(crate) fn add_icon_text(
    commands: &mut Commands,
    entity: Entity,
    class: &str,
    transform: Transform,
    scale: f32,
    asset_server: &AssetServer,
) {
    const TEXT_OFFSET: f32 = 8.0;

    commands
        .spawn(Text2dBundle {
            text: Text::from_section(
                class.to_string(),
                TextStyle {
                    // font: asset_server.load(FONT_PATH),
                    font: TextStyle::default().font,
                    font_size: 8.0 * scale,
                    color: Color::WHITE,
                },
            )
            .with_alignment(TextAlignment::Center),
            transform: Transform {
                translation: Vec3::new(
                    transform.translation.x,
                    transform.translation.y - (ICON_SIZE * scale / 2.0) - TEXT_OFFSET,
                    transform.translation.z - 0.01,
                ),
                scale: Vec3::splat(scale),
                ..default()
            },
            ..default()
        })
        .insert(IconText(entity));
}

pub(crate) fn set_favorite_pin(
    commands: &mut Commands,
    asset_server: &Res<AssetServer>,
    parent_entity: Entity,
) {
    commands.entity(parent_entity).with_children(|parent| {
        let translation = Vec3::new(ICON_SIZE / 3.0, ICON_SIZE / 2.0, 0.1);

        let initial_transform = Transform {
            translation,
            scale: Vec3::splat(0.4),
            ..default()
        };

        let target_transform = Transform {
            translation,
            scale: Vec3::splat(1.0),
            ..default()
        };

        parent
            .spawn(Svg2dBundle {
                svg: asset_server.load(ASSETS_ICON_PIN_PATH),
                origin: Origin::Center,
                transform: initial_transform,
                ..Default::default()
            })
            .insert(FavoritePin)
            .insert(initial_transform.ease_to(
                target_transform,
                EaseFunction::ElasticOut,
                EasingType::PingPong {
                    duration: std::time::Duration::from_millis(400),
                    pause: Some(std::time::Duration::from_millis(50)),
                },
            ));
    });
}
