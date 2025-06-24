use crate::types::*;
use crate::config::Config;
use crate::utils::calculate_icon_transform;

use bevy::prelude::*;
use bevy::window::PrimaryWindow;



pub fn reorder_icons_system(
    mut q_icons: Query<(Entity, &ClientAddress, &mut Transform, &mut HoverTarget)>,
    dock_order: Res<DockOrder>,
    windows: Query<&Window, With<PrimaryWindow>>,
    scroll_state: Res<ScrollState>,
    config: Res<Config>,
    mut reorder_trigger: ResMut<ReorderTrigger>,
) {
    let window = windows.single();
    let start_x = -window.width() / 2.0 + config.margin_x;
    let start_y = -window.height() / 2.0 + config.margin_y;
    let start_pos = Vec2::new(start_x, start_y);
    let center = Vec2::new(0.0, window.height() * config.tilt_y);
    let direction = (center - start_pos).normalize_or_zero();

    for (index, address) in dock_order.0.iter().enumerate() {
        let (translation, scale) =
            calculate_icon_transform(index, start_pos, direction, &config, scroll_state.offset);

        for (_entity, icon_address, mut transform, mut hover) in q_icons.iter_mut() {
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
