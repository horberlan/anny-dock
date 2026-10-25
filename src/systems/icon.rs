use crate::types::*;
use crate::config::Config;
use crate::utils::{calculate_icon_transform, save_favorites, title_transform_scale};
use crate::{IconText, Favorite, Favorites};
use bevy::prelude::*;
use bevy::window::PrimaryWindow;

pub fn collect_icon_data(
    query: Query<(Entity, &Transform, &HoverTarget)>,
    mut icon_positions: ResMut<IconPositions>,
) {
    icon_positions.0.clear();
    for (entity, transform, _hover) in query.iter() {
        icon_positions
            .0
            .insert(entity, (transform.translation, transform.scale));
    }
}

pub fn update_text_positions(
    icon_query: Query<(&Transform, &ClientClass), Without<IconText>>,
    mut text_query: Query<(&mut Transform, &IconText)>,
    config: Res<Config>,
) {
    for (mut text_transform, icon_text) in text_query.iter_mut() {
        if let Ok((icon_transform, _)) = icon_query.get(icon_text.0) {
            let scale = icon_transform.scale.y;
            text_transform.translation.x = icon_transform.translation.x;
            text_transform.translation.y =
                icon_transform.translation.y - (config.icon_size * scale / 2.0) - 2.0;
            text_transform.translation.z = icon_transform.translation.z - 0.01;
            text_transform.scale = Vec3::splat(title_transform_scale(scale, &config));
        }
    }
}

pub fn reorder_icons_system(
    mut q_icons: Query<(Entity, &ClientAddress, &ClientClass, &mut Transform, &mut HoverTarget, Option<&Favorite>)>,
    mut dock_order: ResMut<DockOrder>,
    mut favorites: ResMut<Favorites>,
    windows: Query<&Window, With<PrimaryWindow>>,
    scroll_state: Res<ScrollState>,
    config: Res<Config>,
    ui_state: Res<UiState>,
    mut reorder_trigger: ResMut<ReorderTrigger>,
) {
    if reorder_trigger.0 {
        // dock_order is the source of truth: drags, pins and window events edit
        // it in place. Only drop addresses without an icon and append icons that
        // are missing; query iteration order is arbitrary (it changes whenever an
        // entity moves between archetypes), so it is just a fallback.
        let mut new_order: Vec<String> = Vec::new();
        for address in &dock_order.0 {
            if !new_order.contains(address)
                && q_icons.iter().any(|(_, addr, _, _, _, _)| addr.0 == *address)
            {
                new_order.push(address.clone());
            }
        }
        for (_, addr, _, _, _, _) in q_icons.iter() {
            if !new_order.contains(&addr.0) {
                new_order.push(addr.0.clone());
            }
        }

        // Favorites are saved in dock order so pinned apps reopen where they were.
        let mut saved_favorites: Vec<String> = Vec::new();
        for address in &new_order {
            if let Some((_, _, class, _, _, _)) =
                q_icons.iter().find(|(_, addr, _, _, _, _)| addr.0 == *address)
            {
                if favorites.0.contains(&class.0) && !saved_favorites.contains(&class.0) {
                    saved_favorites.push(class.0.clone());
                }
            }
        }
        for class in &favorites.0 {
            if !saved_favorites.contains(class) {
                saved_favorites.push(class.clone());
            }
        }
        if saved_favorites != favorites.0 {
            favorites.0 = saved_favorites;
        }

        dock_order.0 = new_order;
    }
    
    let window = windows.single();
    let start_x = -window.width() / 2.0 + config.margin_x;
    let start_y = -window.height() / 2.0 + config.margin_y;
    let start_pos = Vec2::new(start_x, start_y);
    let center = Vec2::new(0.0, window.height() * config.tilt_y);
    let direction = (center - start_pos).normalize_or_zero();

    for (index, address) in dock_order.0.iter().enumerate() {
        let (translation, scale) =
            calculate_icon_transform(index, start_pos, direction, &config, scroll_state.offset);

        for (entity, icon_address, _, mut transform, mut hover, _) in q_icons.iter_mut() {
            if icon_address.0 == *address {
                // The dragged icon follows the cursor (drag_update_system); its
                // slot still updates so it animates there on drop or cancel.
                if ui_state.dragging != Some(entity) {
                    let current_pos = transform.translation;
                    let target_pos = translation;

                    transform.translation = Vec3::new(
                        lerp(current_pos.x, target_pos.x, 0.2),
                        lerp(current_pos.y, target_pos.y, 0.2),
                        target_pos.z,
                    );
                }

                hover.original_position = translation.truncate();
                hover.original_z = translation.z;
                hover.original_scale = scale;
                hover.index = index;
                break;
            }
        }
    }

    reorder_trigger.0 = false;
}
/// Persists favorites whose order changed without going through
/// `toggle_favorite` (which saves on its own), e.g. after a drag.
pub fn persist_favorites_system(favorites: Res<Favorites>) {
    if favorites.is_changed() && !favorites.is_added() {
        save_favorites(&favorites);
    }
}

fn lerp(start: f32, end: f32, t: f32) -> f32 {
    start + (end - start) * t
}
