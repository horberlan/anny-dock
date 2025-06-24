pub use loader::*;
pub mod hover;
pub mod loader;

pub mod launcher;
pub use launcher::*;

use crate::config::Config;
use bevy::math::{Vec2, Vec3};
use bevy::prelude::*;

#[derive(Resource)]
pub struct IconAnimationState {
    pub _is_scrolling: bool,
    pub _scroll_timer: Timer,
}

impl Default for IconAnimationState {
    fn default() -> Self {
        Self {
            _is_scrolling: false,
            _scroll_timer: Timer::from_seconds(0.3, TimerMode::Once),
        }
    }
}

pub fn calculate_icon_transform(
    index: usize,
    start_pos: Vec2,
    direction: Vec2,
    config: &Config,
    scroll_offset: Vec2,
) -> (Vec3, f32) {
    let scale_dampening = 0.4;
    let r = config.scale_factor + (1.0 - config.scale_factor) * scale_dampening;

    let spacing_boost = 1.2;
    let i = index as f32;

    let total_spacing_multiplier = if (r - 1.0).abs() < f32::EPSILON {
        i
    } else {
        (1.0 - r.powf(i)) / (1.0 - r)
    };

    let base_offset =
        direction * (total_spacing_multiplier * config.spacing * spacing_boost * config.base_scale);
    let scrolled_pos = start_pos + base_offset - scroll_offset;

    let x = scrolled_pos.x;
    let y = scrolled_pos.y;
    let z = -(index as f32 * config.z_spacing);

    let base_scale = config.base_scale * r.powi(index as i32);

    let is_scrolling = scroll_offset.length() > 0.1;
    let scale = if is_scrolling {
        config.base_scale
    } else {
        base_scale
    };

    (Vec3::new(x, y, z), scale)
}

pub fn update_sprite_alpha(sprite: &mut Sprite, is_pinned: bool, is_running: bool) {
    let alpha = if is_pinned && !is_running {
        0.5
    } else {
        1.0
    };
    sprite.color.set_a(alpha);
}
