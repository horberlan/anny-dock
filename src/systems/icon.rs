use crate::types::*;
use crate::config::Config;
use crate::utils::calculate_icon_transform;
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
            text_transform.scale = Vec3::splat(scale);
        }
    }
}

pub fn reorder_icons_system(
    mut q_icons: Query<(Entity, &ClientAddress, &ClientClass, &mut Transform, &mut HoverTarget, Option<&Favorite>)>,
    mut dock_order: ResMut<DockOrder>,
    favorites: Res<Favorites>,
    windows: Query<&Window, With<PrimaryWindow>>,
    scroll_state: Res<ScrollState>,
    config: Res<Config>,
    mut reorder_trigger: ResMut<ReorderTrigger>,
) {
    if reorder_trigger.0 {
        // Rebuild dock_order to ensure favorites are first
        let mut new_order = Vec::new();
        let mut non_favorite_addresses = Vec::new();
        
        // Collect all current addresses with their classes
        let mut address_to_class: std::collections::HashMap<String, String> = std::collections::HashMap::new();
        for (_, addr, class, _, _, _) in q_icons.iter() {
            address_to_class.insert(addr.0.clone(), class.0.clone());
        }
        
        // First, add favorites in order
        for fav_class in &favorites.0 {
            // Find the address for this favorite class
            if let Some((_, addr, _, _, _, _)) = q_icons.iter().find(|(_, _, class, _, _, _)| &class.0 == fav_class) {
                new_order.push(addr.0.clone());
            }
        }
        
        // Then, add non-favorites
        for (_, addr, _, _, _, favorite_opt) in q_icons.iter() {
            if favorite_opt.is_none() && !new_order.contains(&addr.0) {
                non_favorite_addresses.push(addr.0.clone());
            }
        }
        
        new_order.extend(non_favorite_addresses);
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

        for (_entity, icon_address, _, mut transform, mut hover, _) in q_icons.iter_mut() {
            if icon_address.0 == *address {
                let current_pos = transform.translation;
                let target_pos = translation;

                transform.translation = Vec3::new(
                    lerp(current_pos.x, target_pos.x, 0.2),
                    lerp(current_pos.y, target_pos.y, 0.2),
                    target_pos.z,
                );

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
fn lerp(start: f32, end: f32, t: f32) -> f32 {
    start + (end - start) * t
}
