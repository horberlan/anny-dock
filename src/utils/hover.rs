use bevy::ecs::query::With;
use bevy::ecs::system::Query;
use bevy::prelude::*;
use bevy::render::camera::Camera;
use bevy::time::Time;
use bevy::transform::components::{GlobalTransform, Transform};
use bevy::window::PrimaryWindow;
use bevy::time::Timer;
use std::time::Duration;

use crate::{Dragging, HoverTarget, MainCamera, UiState, ICON_SIZE, ScrollState, DockConfig};

const HOVER_LIFT: f32 = 35.0;
const HOVER_SCALE: f32 = 1.15;
const ANIMATION_SMOOTHNESS: f32 = 0.85;

pub fn hover_system(
    windows: Query<&Window, With<PrimaryWindow>>,
    q_camera: Query<(&Camera, &GlobalTransform), With<MainCamera>>,
    mut q_icons: Query<(&mut HoverTarget, &Transform)>,
    time: Res<Time>,
    ui_state: Res<UiState>,
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
                    let size = Vec2::splat(ICON_SIZE);
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

pub fn hover_animation_system(
    time: Res<Time>,
    mut q: Query<(
        &mut Transform,
        &HoverTarget,
        &mut HoverState,
    ), Without<Dragging>>,
    ui_state: Res<UiState>,
    scroll_state: Res<ScrollState>,
    config: Res<crate::utils::DockConfig>,
) {
    if ui_state.dragging.is_some() {
        return;
    }

    let delta_time = time.delta_seconds();

    let scroll = scroll_state.total_scroll_distance / config.spacing;
    let first_visible_index = scroll.floor() as usize;
    let interp = scroll - scroll.floor();

    let mut scales = vec![1.0; config.visible_items];
    for i in 0..config.visible_items {
        if i == 0 {
            scales[i] = 1.2 - 0.2 * interp as f32;
        } else if i == 1 {
            scales[i] = 1.0 + 0.2 * interp as f32;
        } else {
            scales[i] = 1.0;
        }
    }

    let sum: f32 = scales.iter().sum();
    let norm = config.visible_items as f32 / sum;
    for s in &mut scales {
        *s *= norm;
    }

    for (mut transform, hover, mut state) in &mut q {
        let base_scale = config.base_scale * config.scale_factor.powi(hover.index as i32);

        let in_window = hover.index >= first_visible_index
            && hover.index < first_visible_index + config.visible_items;

        let mut target_scale = 0.0;
        if in_window {
            let rel_idx = hover.index - first_visible_index;
            target_scale = base_scale * scales[rel_idx];
        }

        if hover.is_hovered && in_window {
            target_scale = base_scale * HOVER_SCALE;
        }

        state.target_scale = target_scale;
        state.target_lift = if hover.is_hovered && in_window { HOVER_LIFT } else { 0.0 };

        state.current_lift += (state.target_lift - state.current_lift)
            * (1.0 - ANIMATION_SMOOTHNESS.powf(delta_time * 60.0));

        state.current_scale += (state.target_scale - state.current_scale)
            * (1.0 - ANIMATION_SMOOTHNESS.powf(delta_time * 60.0));

        transform.translation = Vec3::new(
            hover.original_position.x,
            hover.original_position.y + state.current_lift,
            hover.original_z,
        );

        transform.scale = Vec3::splat(state.current_scale);
    }
}