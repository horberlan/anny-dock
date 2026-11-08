use bevy::ecs::query::With;
use bevy::ecs::system::Query;
use bevy::prelude::*;
use bevy::render::camera::Camera;
use bevy::time::Time;
use bevy::transform::components::{GlobalTransform, Transform};
use bevy::window::PrimaryWindow;
use bevy::time::Timer;
use std::time::Duration;

use crate::{config::Config, types::*, utils::front_boost};

const HOVER_LIFT: f32 = 15.0;
const HOVER_SCALE: f32 = 1.15;

pub fn hover_system(
    windows: Query<&Window, With<PrimaryWindow>>,
    q_camera: Query<(&Camera, &GlobalTransform), With<MainCamera>>,
    mut q_icons: Query<(&mut HoverTarget, &Transform)>,
    time: Res<Time>,
    ui_state: Res<UiState>,
    config: Res<Config>,
) {
    if ui_state.dragging.is_some() {
        return;
    }

    let window = windows.single();
    if let Ok((camera, camera_transform)) = q_camera.get_single() {
        if let Some(cursor_pos) = window.cursor_position() {
            if let Some(world_pos) = camera.viewport_to_world_2d(camera_transform, cursor_pos) {
                let mut top_hovered: Option<(usize, f32)> = None;
                for (i, (_hover_target, transform)) in q_icons.iter().enumerate() {
                    let icon_position = transform.translation.truncate();
                    let size = Vec2::splat(config.icon_size * transform.scale.x);
                    let rect = Rect::from_center_size(icon_position, size * 1.1);
                    if rect.contains(world_pos) {
                        let z = transform.translation.z;
                        if top_hovered.is_none() || z > top_hovered.unwrap().1 {
                            top_hovered = Some((i, z));
                        }
                    }
                }

                for (i, (mut hover_target, _)) in q_icons.iter_mut().enumerate() {
                    if Some(i) == top_hovered.map(|(idx, _)| idx) {
                        if !hover_target.is_hovered {
                            hover_target.is_hovered = true;
                            hover_target.hover_exit_timer = None;
                        }
                    } else {
                        if hover_target.is_hovered && hover_target.hover_exit_timer.is_none() {
                            hover_target.hover_exit_timer = Some(Timer::new(Duration::from_secs_f32(0.15), TimerMode::Once));
                        }
                        hover_target.is_hovered = false;
                    }
                }
            } else {
                for (mut hover, _) in &mut q_icons {
                    if hover.is_hovered && hover.hover_exit_timer.is_none() {
                        hover.hover_exit_timer = Some(Timer::new(Duration::from_secs_f32(0.15), TimerMode::Once));
                    }
                    hover.is_hovered = false;
                }
            }
        } else {
            for (mut hover, _) in &mut q_icons {
                if hover.is_hovered && hover.hover_exit_timer.is_none() {
                    hover.hover_exit_timer = Some(Timer::new(Duration::from_secs_f32(0.15), TimerMode::Once));
                }
                hover.is_hovered = false;
            }
        }
    }

    for (mut hover, _) in &mut q_icons {
        if let Some(timer) = hover.hover_exit_timer.as_mut() {
            timer.tick(time.delta());
            if timer.finished() {
                hover.is_hovered = false;
                hover.hover_exit_timer = None;
            }
        }
    }
}

#[derive(Component)]
pub struct HoverState {
    pub current_lift: f32,
    pub current_scale: f32,
    pub target_lift: f32,
    pub target_scale: f32,
}

impl Default for HoverState {
    fn default() -> Self {
        Self {
            current_lift: 0.0,
            current_scale: 1.0,
            target_lift: 0.0,
            target_scale: 1.0,
        }
    }
}

/// Rate (1/s) at which icons follow their slot, scale and hover lift.
const ICON_SMOOTHING: f32 = 18.0;

/// The only system that animates icon transforms: eases every icon except the
/// dragged one toward the slot `reorder_icons_system` assigned, with the hover
/// lift and scale on top, so reorders, scrolling and hover blend smoothly.
pub fn hover_animation_system(
    time: Res<Time>,
    mut q: Query<(&mut Transform, &HoverTarget, &mut HoverState), Without<Dragging>>,
    ui_state: Res<UiState>,
    scroll_state: Res<ScrollState>,
) {
    let follow = 1.0 - (-ICON_SMOOTHING * time.delta_seconds()).exp();
    // While an icon is dragged the others only make room, without hover effects.
    let dragging = ui_state.dragging.is_some();

    for (mut transform, hover, mut state) in &mut q {
        let hovered = hover.is_hovered && !dragging && hover.original_scale > 0.0;
        let slot = hover.index as f32 - scroll_state.position;
        state.target_scale = if hovered {
            hover.original_scale / front_boost(slot) * HOVER_SCALE
        } else {
            hover.original_scale
        };
        state.target_lift = if hovered { HOVER_LIFT } else { 0.0 };
        state.current_lift += (state.target_lift - state.current_lift) * follow;
        state.current_scale += (state.target_scale - state.current_scale) * follow;

        let target = Vec2::new(
            hover.original_position.x,
            hover.original_position.y + state.current_lift,
        );
        let current = transform.translation.truncate();
        transform.translation = (current + (target - current) * follow).extend(hover.original_z);
        transform.scale = Vec3::splat(state.current_scale);
    }
}
