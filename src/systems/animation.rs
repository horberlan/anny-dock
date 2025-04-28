use bevy::prelude::*;
use bevy::input::mouse::MouseWheel;
use crate::types::*;
use crate::utils::DockConfig;

#[derive(Resource, Default)]
pub struct ScrollAnimationState {
    pub is_scrolling: bool,
    pub timer: Timer,
}

pub fn icon_scale_animation_system(
    mut q_icons: Query<(&mut Transform, &HoverTarget)>,
    scroll_state: Res<ScrollState>,
    config: Res<DockConfig>,
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

    for (mut transform, hover) in q_icons.iter_mut() {
        let base_scale = config.base_scale * config.scale_factor.powi(hover.index as i32);
        
        let target_scale = if scroll_animation.is_scrolling {
            base_scale * 1.5
        } else {
            base_scale
        };

        let current_scale = transform.scale.x;
        let new_scale = lerp(current_scale, target_scale, time.delta_seconds() * 5.0);

        transform.scale = Vec3::splat(new_scale);
    }
}

fn lerp(start: f32, end: f32, t: f32) -> f32 {
    start + (end - start) * t.clamp(0.0, 1.0)
} 