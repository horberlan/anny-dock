use bevy::prelude::*;
use bevy::window::PrimaryWindow;
use crate::types::*;
use crate::config::Config;
use crate::utils::{calculate_icon_transform, dock_axis};

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
    config: Res<Config>,
) {
    if mouse_button.pressed(MouseButton::Left) && ui_state.dragging.is_none() {
        if let (Some(click_origin), Ok(window)) = (ui_state.click_origin, windows.get_single()) {
            if let Some(cursor_pos) = window.cursor_position() {
                if click_origin.distance(cursor_pos) > 10.0 {
                    if let Ok((camera, camera_transform)) = q_camera.get_single() {
                        if let Some(world_cursor) =
                            camera.viewport_to_world_2d(camera_transform, cursor_pos)
                        {
                            let mut top_dragged: Option<(Entity, f32, Vec2)> = None;

                            for (entity, _hover, transform) in q_icons.iter() {
                                let pos = transform.translation.truncate();
                                let size = Vec2::splat(config.icon_size * transform.scale.x);
                                let rect = Rect::from_center_size(pos, size * 1.1);

                                if rect.contains(world_cursor) {
                                    let z = transform.translation.z;
                                    if top_dragged.is_none() || z > top_dragged.unwrap().1 {
                                        top_dragged = Some((entity, z, pos));
                                    }
                                }
                            }

                            if let Some((entity, _z, pos)) = top_dragged {
                                let offset = world_cursor - pos;
                                commands.entity(entity).insert(Dragging { offset });
                                ui_state.dragging = Some(entity);
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
    config: Res<Config>,
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

                            let base_scale = config.base_scale * config.scale_factor.powi(hover.index as i32);
                            transform.scale = Vec3::splat(base_scale * 1.1);
                        }
                    }
                }
            }
        }
    }
}

/// Extra distance, as a fraction of the gap between two slots, the dragged icon
/// must travel past their midpoint before the preview switches slot. Keeps the
/// preview from flickering while the icon rests near a boundary.
const DROP_HYSTERESIS: f32 = 0.1;

/// Slot the dragged icon drops into, given each slot's position along the dock
/// axis and the slot currently previewed.
pub fn drop_target_index(slots: &[f32], dragged: f32, current: usize) -> usize {
    let distance = |i: usize| (slots[i] - dragged).abs();
    let Some(nearest) = (0..slots.len()).min_by(|&a, &b| distance(a).total_cmp(&distance(b))) else {
        return current;
    };
    if current >= slots.len() || nearest == current {
        return nearest;
    }
    let gap = (slots[nearest] - slots[current]).abs();
    if distance(current) - distance(nearest) > 2.0 * DROP_HYSTERESIS * gap {
        nearest
    } else {
        current
    }
}

/// Moves the dragged icon's entry in `DockOrder` to the slot under it, so the
/// other icons slide out of the way while dragging and the drop lands exactly
/// where the preview shows.
pub fn drag_preview_system(
    ui_state: Res<UiState>,
    mut preview: ResMut<DragPreview>,
    mut dock_order: ResMut<DockOrder>,
    q_icons: Query<(&Transform, &ClientAddress)>,
    windows: Query<&Window, With<PrimaryWindow>>,
    scroll_state: Res<ScrollState>,
    config: Res<Config>,
) {
    let Some(dragged) = ui_state.dragging else {
        return;
    };
    let (Ok((transform, address)), Ok(window)) = (q_icons.get(dragged), windows.get_single()) else {
        return;
    };

    if preview.original_order.is_none() {
        preview.target_index = dock_order.0.iter().position(|a| *a == address.0).unwrap_or(0);
        preview.original_order = Some(dock_order.0.clone());
    }

    let mut new_order: Vec<String> =
        dock_order.0.iter().filter(|a| **a != address.0).cloned().collect();

    let (start_pos, direction) = dock_axis(window, &config);
    let along = |position: Vec2| (position - start_pos).dot(direction);
    let slots: Vec<f32> = (0..=new_order.len())
        .map(|index| {
            let (translation, _) =
                calculate_icon_transform(index, start_pos, direction, &config, scroll_state.offset);
            along(translation.truncate())
        })
        .collect();

    let target = drop_target_index(&slots, along(transform.translation.truncate()), preview.target_index);
    preview.target_index = target;
    new_order.insert(target, address.0.clone());
    if new_order != dock_order.0 {
        dock_order.0 = new_order;
    }
}

pub fn drag_end_system(
    mut commands: Commands,
    mouse_button: Res<Input<MouseButton>>,
    mut ui_state: ResMut<UiState>,
    mut preview: ResMut<DragPreview>,
    mut reorder_trigger: ResMut<ReorderTrigger>,
) {
    if mouse_button.just_released(MouseButton::Left) {
        if let Some(dragged_entity) = ui_state.dragging.take() {
            // drag_preview_system already placed the icon in DockOrder.
            if let Some(mut entity) = commands.get_entity(dragged_entity) {
                entity.remove::<Dragging>();
            }
            preview.original_order = None;
            reorder_trigger.0 = true;
        }
    }
}

/// Esc cancels a drag: the original order comes back and reorder_icons_system
/// animates every icon, the dragged one included, to its old slot.
pub fn reset_positions_system(
    mut commands: Commands,
    keyboard_input: Res<Input<KeyCode>>,
    mut ui_state: ResMut<UiState>,
    mut preview: ResMut<DragPreview>,
    mut dock_order: ResMut<DockOrder>,
    mut reorder_trigger: ResMut<ReorderTrigger>,
) {
    if keyboard_input.just_pressed(KeyCode::Escape) {
        if let Some(dragged_entity) = ui_state.dragging.take() {
            if let Some(mut entity) = commands.get_entity(dragged_entity) {
                entity.remove::<Dragging>();
            }
            if let Some(original_order) = preview.original_order.take() {
                dock_order.0 = original_order;
            }
            reorder_trigger.0 = true;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::components::{Favorite, Favorites};
    use crate::systems::{exit_on_esc_or_q, reorder_icons_system};
    use bevy::app::AppExit;
    use bevy::input::keyboard::KeyboardInput;
    use bevy::input::ButtonState;

    fn icon_positions(app: &mut App, count: usize) -> Vec<Vec3> {
        let config = app.world.resource::<Config>().clone();
        let mut q_window = app.world.query_filtered::<&Window, With<PrimaryWindow>>();
        let window = q_window.single(&app.world);
        let start_pos = Vec2::new(
            -window.width() / 2.0 + config.margin_x,
            -window.height() / 2.0 + config.margin_y,
        );
        let center = Vec2::new(0.0, window.height() * config.tilt_y);
        let direction = (center - start_pos).normalize_or_zero();
        (0..count)
            .map(|i| calculate_icon_transform(i, start_pos, direction, &config, Vec2::ZERO).0)
            .collect()
    }

    /// Spawns one icon per class, laid out in dock order; classes listed in
    /// `favorites` get the `Favorite` component.
    fn dock_app(classes: &[&str], favorites: &[&str]) -> (App, Vec<Entity>) {
        let mut app = App::new();
        app.insert_resource(Config::default())
            .insert_resource(Input::<MouseButton>::default())
            .insert_resource(UiState::default())
            .insert_resource(ReorderTrigger::default())
            .insert_resource(ScrollState::default())
            .insert_resource(Favorites(favorites.iter().map(|f| f.to_string()).collect()))
            .insert_resource(DockOrder(classes.iter().map(|c| format!("addr-{c}")).collect()))
            .insert_resource(Input::<KeyCode>::default())
            .insert_resource(DragPreview::default())
            .add_event::<KeyboardInput>()
            .add_event::<AppExit>()
            // Same relative order as the chain in main.rs.
            .add_systems(
                Update,
                (
                    drag_preview_system,
                    drag_end_system,
                    exit_on_esc_or_q,
                    reset_positions_system,
                    reorder_icons_system,
                )
                    .chain(),
            );
        app.world.spawn((Window::default(), PrimaryWindow));

        let positions = icon_positions(&mut app, classes.len());
        let entities = classes
            .iter()
            .zip(positions)
            .enumerate()
            .map(|(index, (class, translation))| {
                let mut icon = app.world.spawn((
                    ClientAddress(format!("addr-{class}")),
                    ClientClass(class.to_string()),
                    Transform::from_translation(translation),
                    HoverTarget {
                        original_position: translation.truncate(),
                        original_z: translation.z,
                        original_scale: 1.0,
                        index,
                        is_hovered: false,
                        hover_exit_timer: None,
                    },
                ));
                if favorites.contains(class) {
                    icon.insert(Favorite);
                }
                icon.id()
            })
            .collect();
        (app, entities)
    }

    /// Slot positions for the current dock length.
    fn slots(app: &mut App) -> Vec<Vec3> {
        let count = app.world.resource::<DockOrder>().0.len();
        icon_positions(app, count)
    }

    /// Holds `dragged` (mouse still pressed) at `position` and runs one frame.
    fn drag_to(app: &mut App, dragged: Entity, position: Vec3) {
        app.world.get_mut::<Transform>(dragged).unwrap().translation = position;
        if app.world.get::<Dragging>(dragged).is_none() {
            app.world.entity_mut(dragged).insert(Dragging { offset: Vec2::ZERO });
            app.world.resource_mut::<UiState>().dragging = Some(dragged);
            app.world.resource_mut::<Input<MouseButton>>().press(MouseButton::Left);
        }
        app.update();
    }

    fn drag_to_slot(app: &mut App, dragged: Entity, slot: usize) {
        let position = slots(app)[slot];
        drag_to(app, dragged, position);
    }

    fn release(app: &mut App) {
        app.world.resource_mut::<Input<MouseButton>>().release(MouseButton::Left);
        app.update();
    }

    fn drop_at(app: &mut App, dragged: Entity, slot: usize) {
        drag_to_slot(app, dragged, slot);
        release(app);
    }

    fn dock_order(app: &App) -> Vec<String> {
        app.world.resource::<DockOrder>().0.clone()
    }

    #[test]
    fn dropped_icon_lands_where_released() {
        let (mut app, icons) = dock_app(&["a", "b", "c", "d"], &[]);

        drop_at(&mut app, icons[3], 1);

        assert_eq!(dock_order(&app), ["addr-a", "addr-d", "addr-b", "addr-c"]);
        assert!(app.world.get::<Dragging>(icons[3]).is_none());
    }

    #[test]
    fn dropped_favorite_lands_where_released() {
        let (mut app, icons) = dock_app(&["f1", "f2", "f3", "x"], &["f1", "f2", "f3"]);

        drop_at(&mut app, icons[2], 1);

        assert_eq!(dock_order(&app), ["addr-f1", "addr-f3", "addr-f2", "addr-x"]);
        assert_eq!(app.world.resource::<Favorites>().0, ["f1", "f3", "f2"]);
    }

    #[test]
    fn non_favorite_can_be_dropped_between_favorites() {
        let (mut app, icons) = dock_app(&["f1", "f2", "x"], &["f1", "f2"]);

        drop_at(&mut app, icons[2], 1);

        assert_eq!(dock_order(&app), ["addr-f1", "addr-x", "addr-f2"]);
        assert_eq!(app.world.resource::<Favorites>().0, ["f1", "f2"]);
    }

    #[test]
    fn favorite_can_be_dropped_after_non_favorites() {
        let (mut app, icons) = dock_app(&["f1", "f2", "a", "b"], &["f1", "f2"]);

        drop_at(&mut app, icons[0], 2);

        assert_eq!(dock_order(&app), ["addr-f2", "addr-a", "addr-f1", "addr-b"]);
        assert_eq!(app.world.resource::<Favorites>().0, ["f2", "f1"]);
    }

    #[test]
    fn neighbors_make_room_before_release() {
        let (mut app, icons) = dock_app(&["a", "b", "c", "d"], &[]);

        drag_to_slot(&mut app, icons[3], 1);

        assert_eq!(dock_order(&app), ["addr-a", "addr-d", "addr-b", "addr-c"]);
        assert!(app.world.get::<Dragging>(icons[3]).is_some(), "still dragging");
    }

    #[test]
    fn preview_waits_until_icon_is_clearly_past_midpoint() {
        let (mut app, icons) = dock_app(&["a", "b", "c"], &[]);
        let slots = slots(&mut app);

        drag_to(&mut app, icons[1], slots[1].lerp(slots[0], 0.52));
        assert_eq!(dock_order(&app), ["addr-a", "addr-b", "addr-c"]);

        drag_to(&mut app, icons[1], slots[1].lerp(slots[0], 0.7));
        assert_eq!(dock_order(&app), ["addr-b", "addr-a", "addr-c"]);

        // Coming back near the midpoint keeps the new slot instead of flickering.
        drag_to(&mut app, icons[1], slots[1].lerp(slots[0], 0.48));
        assert_eq!(dock_order(&app), ["addr-b", "addr-a", "addr-c"]);
    }

    #[test]
    fn dragged_icon_follows_cursor_without_lag() {
        let (mut app, icons) = dock_app(&["a", "b", "c"], &[]);
        let target = slots(&mut app)[0] + Vec3::new(3.0, 5.0, 0.0);

        drag_to(&mut app, icons[2], target);

        let translation = app.world.get::<Transform>(icons[2]).unwrap().translation;
        assert_eq!(translation.truncate(), target.truncate());
    }

    #[test]
    fn escape_cancels_drag_without_closing_dock() {
        let (mut app, icons) = dock_app(&["a", "b", "c", "d"], &[]);
        drag_to_slot(&mut app, icons[3], 1);

        app.world.resource_mut::<Input<KeyCode>>().press(KeyCode::Escape);
        app.world.send_event(KeyboardInput {
            scan_code: 1,
            key_code: Some(KeyCode::Escape),
            state: ButtonState::Pressed,
            window: Entity::PLACEHOLDER,
        });
        app.update();

        assert_eq!(dock_order(&app), ["addr-a", "addr-b", "addr-c", "addr-d"]);
        assert!(app.world.get::<Dragging>(icons[3]).is_none());
        assert!(app.world.resource::<UiState>().dragging.is_none());
        assert!(app.world.resource::<Events<AppExit>>().is_empty(), "Esc must not close the dock while dragging");
    }

    #[test]
    fn pinning_keeps_icon_in_place() {
        let (mut app, icons) = dock_app(&["a", "b", "c"], &[]);

        // What toggle_favorite does when pinning `c`.
        app.world.entity_mut(icons[2]).insert(Favorite);
        app.world.resource_mut::<Favorites>().0.push("c".to_string());
        app.world.resource_mut::<ReorderTrigger>().0 = true;
        app.update();

        assert_eq!(dock_order(&app), ["addr-a", "addr-b", "addr-c"]);
    }

    #[test]
    fn saved_favorites_follow_dock_order() {
        // `c` was pinned before `a`, but `a` comes first in the dock.
        let (mut app, _icons) = dock_app(&["a", "b", "c"], &["c", "a"]);

        app.world.resource_mut::<ReorderTrigger>().0 = true;
        app.update();

        assert_eq!(dock_order(&app), ["addr-a", "addr-b", "addr-c"]);
        assert_eq!(app.world.resource::<Favorites>().0, ["a", "c"]);
    }
}
