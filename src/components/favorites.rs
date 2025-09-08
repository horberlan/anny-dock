use bevy::prelude::{*};
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};
use bevy::sprite::SpriteBundle;

use serde::{Deserialize, Serialize};

#[derive(Component)]
pub struct FavoritePin;

#[derive(Resource, Deserialize, Serialize, Clone, Default)]
pub struct Favorites(pub Vec<String>);

#[derive(Component, Debug)]
pub struct Favorite;

use crate::config::Config;
use crate::types::{ClientAddress, IconTitleText};

pub(crate) fn add_client_address(commands: &mut Commands, entity: Entity, address: String) {
    commands.entity(entity).insert(ClientAddress(address));
}

pub(crate) fn add_favorite(
    commands: &mut Commands,
    entity: Entity,
    images: &mut Assets<Image>,
    config: &Res<Config>,
) {
    commands.entity(entity).insert(Favorite);
    set_favorite_pin(commands, images, entity, config);
}

pub(crate) fn add_icon_text(
    commands: &mut Commands,
    parent_icon: Entity,
    class: &str,
    _asset_server: &AssetServer,
    config: &Res<Config>,
) {
    const TEXT_OFFSET: f32 = 8.0;

    commands.entity(parent_icon).with_children(|parent| {
        parent.spawn((
            Text2dBundle {
                text: Text::from_section(
                    class.to_string(),
                    TextStyle {
                        font: TextStyle::default().font,
                        font_size: config.font_size,
                        color: Color::WHITE,
                    },
                )
                .with_alignment(TextAlignment::Center),
                transform: Transform {
                    translation: Vec3::new(
                        0.0,
                        -(config.icon_size / 2.0) - TEXT_OFFSET,
                        0.1, 
                    ),
                    ..default()
                },
                visibility: Visibility::Hidden,
                ..default()
            },
            IconTitleText,
        ));
    });
}

pub(crate) fn set_favorite_pin(
    commands: &mut Commands,
    images: &mut Assets<Image>,
    parent_entity: Entity,
    config: &Res<Config>,
) {
    const PIN_ICON_SVG: &[u8] = include_bytes!("../../assets/icons/pin_stroke_rounded.svg");
    if let Some(image) = load_svg_pin_from_bytes(PIN_ICON_SVG) {
        let handle = images.add(image);
        commands.entity(parent_entity).with_children(|parent| {
            let transform = Transform {
                translation: Vec3::new(config.icon_size / 3.0, config.icon_size / 3.0, 0.1),
                scale: Vec3::splat(0.4),
                ..default()
            };

            parent
                .spawn(SpriteBundle {
                    texture: handle,
                    transform,
                    ..Default::default()
                })
                .insert(FavoritePin);
        });
    }
}

fn load_svg_pin_from_bytes(svg_bytes: &[u8]) -> Option<Image> {
    use resvg::render;
    use tiny_skia::Pixmap;
    use usvg::{Options, Tree};

    let opts = Options::default();
    let tree = match Tree::from_data(svg_bytes, &opts) {
        Ok(tree) => tree,
        Err(e) => {
            error!("Failed to parse SVG for pin: {}", e);
            return None;
        }
    };

    let pixmap_size = tree.size.to_screen_size();
    let mut pixmap = match Pixmap::new(pixmap_size.width(), pixmap_size.height()) {
        Some(p) => p,
        _ => {
            error!("Failed to create pixmap for pin");
            return None;
        }
    };

    if render(
        &tree,
        usvg::FitTo::Original,
        tiny_skia::Transform::default(),
        pixmap.as_mut(),
    )
    .is_none()
    {
        error!("Failed to render SVG to pixmap for pin");
        return None;
    }

    let rgba = pixmap.data().to_vec();
    Some(Image::new(
        Extent3d {
            width: pixmap_size.width(),
            height: pixmap_size.height(),
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        rgba,
        TextureFormat::Rgba8Unorm,
    ))
}
