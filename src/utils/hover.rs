use bevy::ecs::query::With;
use bevy::ecs::system::Query;
use bevy::prelude::*;
use bevy::render::camera::Camera;
use bevy::time::Time;
use bevy::transform::components::{GlobalTransform, Transform};
use bevy::window::PrimaryWindow;

use crate::{Dragging, HoverTarget, MainCamera, UiState, ICON_SIZE, ScrollState, DockConfig};

const HOVER_TOLERANCE: f32 = 5.0;
const HOVER_LIFT: f32 = 35.0;
const HOVER_SCALE: f32 = 1.15;
const ANIMATION_SMOOTHNESS: f32 = 0.85;

pub fn hover_system(
    windows: Query<&Window, With<PrimaryWindow>>,
    q_camera: Query<(&Camera, &GlobalTransform), With<MainCamera>>,
    mut q_icons: Query<(&mut HoverTarget, &Transform)>,
    ui_state: Res<UiState>,
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
                    let rect = Rect::from_center_size(pos, size);

                    let is_in_hover_area = rect.contains(world_cursor) || 
                        rect.min.distance(world_cursor) <= HOVER_TOLERANCE;

                    hover.is_hovered = is_in_hover_area;
                }
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
) {
    if ui_state.dragging.is_some() {
        return;
    }

    let dt = time.delta_seconds();

    for (mut transform, hover, mut state) in &mut q {
        state.target_lift = if hover.is_hovered { HOVER_LIFT } else { 0.0 };
        state.target_scale = if hover.is_hovered {
            hover.original_scale * HOVER_SCALE
        } else {
            hover.original_scale
        };

        state.current_lift += (state.target_lift - state.current_lift) * 
            (1.0 - ANIMATION_SMOOTHNESS.powf(dt * 60.0));
        
        state.current_scale += (state.target_scale - state.current_scale) * 
            (1.0 - ANIMATION_SMOOTHNESS.powf(dt * 60.0));

        transform.translation = Vec3::new(
            hover.original_position.x,
            hover.original_position.y + state.current_lift,
            hover.original_z
        );

        transform.scale = Vec3::splat(state.current_scale);
    }
}