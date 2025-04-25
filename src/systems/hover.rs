use bevy::ecs::query::With;
use bevy::ecs::system::Query;
use bevy::prelude::*;
use bevy::render::camera::Camera;
use bevy::time::Time;
use bevy::transform::components::{GlobalTransform, Transform};
use bevy::window::PrimaryWindow;

use crate::{Dragging, HoverTarget, MainCamera, UiState, ICON_SIZE};

pub fn hover_system(
    windows: Query<&Window, With<PrimaryWindow>>,
    q_camera: Query<(&Camera, &GlobalTransform), With<MainCamera>>,
    mut q_icons: Query<(&mut HoverTarget, &Transform)>,
    ui_state: Res<UiState>,
    time: Res<Time>,
) {
    if ui_state.dragging.is_some() {
        return;
    }

    let window = windows.single();
    if let Ok((camera, camera_transform)) = q_camera.get_single() {
        if let Some(cursor_pos) = window.cursor_position() {
            if let Some(world_cursor) = camera.viewport_to_world_2d(camera_transform, cursor_pos) {
                for (mut hover, transform) in &mut q_icons {
                    let pos = transform.translation.truncate();
                    let size = Vec2::splat(ICON_SIZE * hover.original_scale);
                    let rect = Rect::from_center_size(pos, size * 1.1);

                    if rect.contains(world_cursor) {
                        hover.is_hovered = true;
                        hover.hover_exit_timer = None;
                    } else if hover.is_hovered {
                        if hover.hover_exit_timer.is_none() {
                            hover.hover_exit_timer =
                                Some(Timer::from_seconds(0.1, TimerMode::Once));
                        }
                        if let Some(timer) = hover.hover_exit_timer.as_mut() {
                            timer.tick(time.delta());
                            if timer.finished() {
                                hover.is_hovered = false;
                            }
                        }
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
    if ui_state.dragging.is_some() {
        return;
    }

    for (mut transform, hover) in &mut q {
        let target_y = if hover.is_hovered {
            hover.original_position.y + 20.0
        } else {
            hover.original_position.y
        };
        let current_y = transform.translation.y;
        let new_y = current_y + (target_y - current_y) * time.delta_seconds() * 4.0;

        transform.translation = Vec3::new(hover.original_position.x, new_y, hover.original_z);
        let target_scale = if hover.is_hovered {
            hover.original_scale * 1.2
        } else {
            hover.original_scale
        };
        let current_scale = transform.scale.x;
        let new_scale = current_scale + (target_scale - current_scale) * time.delta_seconds() * 3.0;
        transform.scale = Vec3::splat(new_scale);
    }
}
