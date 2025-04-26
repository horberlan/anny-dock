use bevy::prelude::*;
use bevy::input::mouse::MouseWheel;
use bevy::window::PrimaryWindow;
use crate::types::*;
use crate::utils::DockConfig;

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

    for event in scroll_events.iter() {
        let scroll_direction = Vec2::new(1.0, 0.3).normalize();
        
        let scroll_amount = event.y * config.scroll_speed;
        
        scroll_state.total_scroll_distance -= scroll_amount;
        
        let max_scroll = (total_items as f32 - config.visible_items as f32) * config.spacing;
        scroll_state.total_scroll_distance = scroll_state.total_scroll_distance.clamp(0.0, max_scroll);
        
        scroll_state.offset = scroll_direction * scroll_state.total_scroll_distance;
    }
} 