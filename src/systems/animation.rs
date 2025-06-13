use bevy::prelude::*;
use bevy::input::mouse::MouseWheel;
use crate::types::*;
use crate::config::Config;

#[derive(Resource, Default)]
pub struct ScrollAnimationState {
    pub is_scrolling: bool,
    pub timer: Timer,
}

pub fn icon_scale_animation_system(
    _q_icons: Query<(&Transform, &HoverTarget)>,
    _scroll_state: Res<ScrollState>,
    _config: Res<Config>,
    time: Res<Time>,
    mut scroll_events: EventReader<MouseWheel>,
    mut scroll_animation: ResMut<ScrollAnimationState>,
) {
    if !scroll_events.is_empty() {
        scroll_animation.is_scrolling = true;
        scroll_animation.timer = Timer::from_seconds(0.3, TimerMode::Once);
    } else if scroll_animation.is_scrolling {
        scroll_animation.timer.tick(time.delta());
        if scroll_animation.timer.finished() {
            scroll_animation.is_scrolling = false;
        }
    }

    scroll_events.clear();
}