use bevy::prelude::*;
use bevy::window::PrimaryWindow;
use crate::types::*;
use crate::components::*;
use crate::utils::DockConfig;

pub fn drag_register_click_system(
    windows: Query<&Window, With<PrimaryWindow>>,
    mouse_button: Res<Input<MouseButton>>,
    mut ui_state: ResMut<UiState>,
) {
    if mouse_button.just_pressed(MouseButton::Left) {
        if let Ok(window) = windows.get_single() {
            ui_state.click_origin = window.cursor_position();
        }
    }
}

pub fn drag_check_system(
    mut commands: Commands,
    windows: Query<&Window, With<PrimaryWindow>>,
    q_camera: Query<(&Camera, &GlobalTransform), With<MainCamera>>,
    q_icons: Query<(Entity, &HoverTarget, &Transform)>,
    mouse_button: Res<Input<MouseButton>>,
    mut ui_state: ResMut<UiState>,
) {
    if mouse_button.pressed(MouseButton::Left) && ui_state.dragging.is_none() {
        if let (Some(click_origin), Ok(window)) = (ui_state.click_origin, windows.get_single()) {
            if let Some(cursor_pos) = window.cursor_position() {
                if click_origin.distance(cursor_pos) > 10.0 {
                    if let Ok((camera, camera_transform)) = q_camera.get_single() {
                        if let Some(world_cursor) =
                            camera.viewport_to_world_2d(camera_transform, cursor_pos)
                        {
                            for (entity, hover, transform) in q_icons.iter() {
                                let pos = transform.translation.truncate();
                                let size = Vec2::splat(ICON_SIZE * hover.original_scale);
                                let rect = Rect::from_center_size(pos, size * 1.1);
                                if rect.contains(world_cursor) {
                                    let offset = world_cursor - pos;
                                    commands.entity(entity).insert(Dragging { offset });
                                    ui_state.dragging = Some(entity);
                                    break;
                                }
                            }
                        }
                    }
                }
            }
        }
    }
    if mouse_button.just_released(MouseButton::Left) {
        ui_state.click_origin = None;
    }
}

pub fn drag_update_system(
    windows: Query<&Window, With<PrimaryWindow>>,
    q_camera: Query<(&Camera, &GlobalTransform), With<MainCamera>>,
    mut q_dragging: Query<(&mut Transform, &Dragging, &HoverTarget)>,
    ui_state: Res<UiState>,
) {
    if let Some(entity) = ui_state.dragging {
        if let Ok((mut transform, dragging, hover)) = q_dragging.get_mut(entity) {
            if let Ok(window) = windows.get_single() {
                if let Some(cursor_pos) = window.cursor_position() {
                    if let Ok((camera, camera_transform)) = q_camera.get_single() {
                        if let Some(world_cursor) =
                            camera.viewport_to_world_2d(camera_transform, cursor_pos)
                        {
                            let new_pos = world_cursor - dragging.offset;
                            transform.translation =
                                Vec3::new(new_pos.x, new_pos.y, hover.original_z + 10.0);
                            transform.scale = Vec3::splat(hover.original_scale * 1.2);
                        }
                    }
                }
            }
        }
    }
}

pub fn drag_end_system(
    mut commands: Commands,
    mouse_button: Res<Input<MouseButton>>,
    mut ui_state: ResMut<UiState>,
    mut dock_order: ResMut<DockOrder>,
    q_icons: Query<(Entity, &Transform, &ClientAddress)>,
    mut reorder_trigger: ResMut<ReorderTrigger>,
) {
    if mouse_button.just_released(MouseButton::Left) && ui_state.dragging.is_some() {
        if let Some(dragged_entity) = ui_state.dragging {
            commands.entity(dragged_entity).remove::<Dragging>();
            ui_state.dragging = None;

            let (dragged_x, dragged_address) = q_icons
                .get(dragged_entity)
                .map(|(_, transform, address)| (transform.translation.x, address.0.clone()))
                .unwrap_or((0.0, String::new()));

            let mut other_icons: Vec<(String, f32)> = q_icons
                .iter()
                .filter(|(e, _, _)| *e != dragged_entity)
                .map(|(_, transform, address)| (address.0.clone(), transform.translation.x))
                .collect();

            other_icons.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap());

            let new_index = other_icons
                .iter()
                .position(|(_, x)| *x > dragged_x)
                .unwrap_or(other_icons.len());

            let mut new_order = Vec::new();
            let mut added_dragged = false;
            for (i, (address, _)) in other_icons.iter().enumerate() {
                if i == new_index && !added_dragged {
                    new_order.push(dragged_address.clone());
                    added_dragged = true;
                }
                new_order.push(address.clone());
            }
            if !added_dragged {
                new_order.push(dragged_address);
            }

            dock_order.0 = new_order;
            reorder_trigger.0 = true;
        }
    }
}

pub fn reset_positions_system(
    mut commands: Commands,
    mut q_dragging: Query<(Entity, &mut Transform, &HoverTarget), With<Dragging>>,
    keyboard_input: Res<Input<KeyCode>>,
    mut ui_state: ResMut<UiState>,
) {
    if keyboard_input.just_pressed(KeyCode::Escape) {
        for (entity, mut transform, hover) in &mut q_dragging {
            transform.translation = Vec3::new(
                hover.original_position.x,
                hover.original_position.y,
                hover.original_z,
            );
            commands.entity(entity).remove::<Dragging>();
            ui_state.dragging = None;
        }
    }
} 