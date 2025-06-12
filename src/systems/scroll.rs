use crate::types::*;
use crate::utils::DockConfig;
use bevy::input::mouse::MouseWheel;
use bevy::prelude::*;
use bevy::window::PrimaryWindow;

pub fn scroll_system(
    mut scroll_state: ResMut<ScrollState>,
    mut scroll_events: EventReader<MouseWheel>,
    q_icons: Query<&HoverTarget>,
    config: Res<DockConfig>,
    windows: Query<&Window, With<PrimaryWindow>>,
) {
    let total_items = q_icons.iter().count();
    if total_items <= config.visible_items {
        scroll_state.offset = Vec2::ZERO;
        scroll_state.total_scroll_distance = 0.0;
        return;
    }

    let window = windows.single();
    let window_width = window.width();
    let window_height = window.height();

    let start_x = -window_width / 2.0 + config.margin_x;
    let start_y = -window_height / 2.0 + config.margin_y;
    let start_pos = Vec2::new(start_x, start_y);
    let center = Vec2::new(0.0, window_height * config.tilt_y);
    let direction = (center - start_pos).normalize_or_zero();

    for event in scroll_events.read() {
        let scroll_direction = direction;
        let scroll_amount = event.y * config.scroll_speed;
        scroll_state.total_scroll_distance -= scroll_amount;

        let max_scroll =
            ((total_items as f32 - config.visible_items as f32).max(0.0)) * config.spacing;
        scroll_state.total_scroll_distance =
            scroll_state.total_scroll_distance.clamp(0.0, max_scroll);

        scroll_state.offset = scroll_direction * scroll_state.total_scroll_distance;
    }
}
