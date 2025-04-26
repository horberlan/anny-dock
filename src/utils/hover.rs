use bevy::ecs::query::With;
use bevy::ecs::system::Query;
use bevy::prelude::*;
use bevy::render::camera::Camera;
use bevy::time::Time;
use bevy::transform::components::{GlobalTransform, Transform};
use bevy::window::PrimaryWindow;

use crate::{Dragging, HoverTarget, MainCamera, UiState, ICON_SIZE};

const HOVER_TOLERANCE: f32 = 5.0;
const HOVER_ENTER_TIME: f32 = 0.05;
const HOVER_EXIT_TIME: f32 = 0.1;
const HOVER_AREA_SCALE: f32 = 1.2;

pub fn hover_system(
    windows: Query<&Window, With<PrimaryWindow>>,
    q_camera: Query<(&Camera, &GlobalTransform), With<MainCamera>>,
    mut q_icons: Query<(&mut HoverTarget, &Transform)>,
    ui_state: Res<UiState>,
    time: Res<Time>,
) {
    //! uses tolerance areas and timers to prevent flickering when the cursor is near icon edges :)
    if ui_state.dragging.is_some() {
        return;
    }

    let window = windows.single();
    if let Ok((camera, camera_transform)) = q_camera.get_single() {
        if let Some(cursor_pos) = window.cursor_position() {
            if let Some(world_cursor) = camera.viewport_to_world_2d(camera_transform, cursor_pos) {
                for (mut hover, transform) in &mut q_icons {
                    let pos = transform.translation.truncate();
                    let size = Vec2::splat(ICON_SIZE * hover.original_scale * HOVER_AREA_SCALE);
                    let rect = Rect::from_center_size(pos, size);

                    let is_in_hover_area = rect.contains(world_cursor) || 
                        rect.min.distance(world_cursor) <= HOVER_TOLERANCE ||
                        rect.max.distance(world_cursor) <= HOVER_TOLERANCE;

                    if is_in_hover_area {
                        if !hover.is_hovered {
                            if hover.hover_exit_timer.is_none() {
                                hover.hover_exit_timer = Some(Timer::from_seconds(
                                    HOVER_ENTER_TIME,
                                    TimerMode::Once,
                                ));
                            }
                            
                            if let Some(timer) = hover.hover_exit_timer.as_mut() {
                                timer.tick(time.delta());
                                if timer.finished() {
                                    hover.is_hovered = true;
                                    hover.hover_exit_timer = None;
                                }
                            }
                        }
                    } else if hover.is_hovered {
                        if hover.hover_exit_timer.is_none() {
                            hover.hover_exit_timer = Some(Timer::from_seconds(
                                HOVER_EXIT_TIME,
                                TimerMode::Once,
                            ));
                        }
                        if let Some(timer) = hover.hover_exit_timer.as_mut() {
                            timer.tick(time.delta());
                            if timer.finished() {
                                hover.is_hovered = false;
                                hover.hover_exit_timer = None;
                            }
                        }
                    } else {
                        hover.hover_exit_timer = None;
                    }
                }
            }
        }
    }
}

pub fn hover_animation_system(
    time: Res<Time>,
    mut q: Query<(&mut Transform, &HoverTarget), Without<Dragging>>,
    ui_state: Res<UiState>,
) {
    //! uses linear interpolation for position and scale transitions
    if ui_state.dragging.is_some() {
        return;
    }

    for (mut transform, hover) in &mut q {
        let target_y = if hover.is_hovered {
            hover.original_position.y + 20.0
        } else {
            hover.original_position.y
        };
        
        let target_scale = if hover.is_hovered {
            hover.original_scale * 1.2
        } else {
            hover.original_scale
        };

        let speed_factor = if hover.is_hovered { 3.0 } else { 2.0 };
        let delta = time.delta_seconds() * speed_factor;

        let current_y = transform.translation.y;
        let new_y = lerp(current_y, target_y, delta);
        
        let current_scale = transform.scale.x;
        let new_scale = lerp(current_scale, target_scale, delta);

        transform.translation = Vec3::new(
            hover.original_position.x,
            new_y,
            hover.original_z
        );
        transform.scale = Vec3::splat(new_scale);
    }
}

/// linear interpolation f32!!!
fn lerp(start: f32, end: f32, t: f32) -> f32 {
    start + (end - start) * t.clamp(0.0, 1.0)
}
