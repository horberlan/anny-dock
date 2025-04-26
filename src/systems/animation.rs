use bevy::prelude::*;
use bevy::input::mouse::MouseWheel;
use crate::types::*;
use crate::utils::DockConfig;

pub fn icon_scale_animation_system(
    mut q_icons: Query<(&mut Transform, &HoverTarget)>,
    scroll_state: Res<ScrollState>,
    config: Res<DockConfig>,
    time: Res<Time>,
    mut scroll_events: EventReader<MouseWheel>,
    mut scroll_timer: Local<Option<Timer>>,
) {
    let is_scroll_event = !scroll_events.is_empty();
    
    let is_actively_scrolling = if is_scroll_event {
        *scroll_timer = Some(Timer::from_seconds(0.3, TimerMode::Once));
        true
    } else if let Some(timer) = scroll_timer.as_mut() {
        timer.tick(time.delta());
        !timer.finished()
    } else {
        false
    };

    for (mut transform, hover) in q_icons.iter_mut() {
        let base_scale = config.base_scale * config.scale_factor.powi(hover.index as i32);
        
        let target_scale = if is_actively_scrolling {
            base_scale * 1.5 
        } else {
            base_scale 
        };

        let speed_factor = if is_actively_scrolling { 3.0 } else { 5.0 };
        let delta = time.delta_seconds() * speed_factor;

        let current_scale = transform.scale.x;
        let new_scale = lerp(current_scale, target_scale, delta);

        transform.scale = Vec3::splat(new_scale);
    }
}

fn lerp(start: f32, end: f32, t: f32) -> f32 {
    start + (end - start) * t.clamp(0.0, 1.0)
} 