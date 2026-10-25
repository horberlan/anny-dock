use bevy::prelude::*;
use bevy::window::PrimaryWindow;
use crate::types::*;
use crate::config::Config;

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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::components::{Favorite, Favorites};
    use crate::systems::reorder_icons_system;
    use crate::utils::calculate_icon_transform;

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
            .add_systems(Update, (drag_end_system, reorder_icons_system).chain());
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

    /// Drops `dragged` halfway between the icons currently at `before` and `after`.
    fn drop_between(app: &mut App, dragged: Entity, before: Entity, after: Entity) {
        let x_before = app.world.get::<Transform>(before).unwrap().translation.x;
        let x_after = app.world.get::<Transform>(after).unwrap().translation.x;
        app.world.get_mut::<Transform>(dragged).unwrap().translation.x = (x_before + x_after) / 2.0;
        app.world.entity_mut(dragged).insert(Dragging { offset: Vec2::ZERO });
        app.world.resource_mut::<UiState>().dragging = Some(dragged);

        let mut mouse = app.world.resource_mut::<Input<MouseButton>>();
        mouse.press(MouseButton::Left);
        mouse.release(MouseButton::Left);
        app.update();
    }

    fn dock_order(app: &App) -> Vec<String> {
        app.world.resource::<DockOrder>().0.clone()
    }

    #[test]
    fn dropped_icon_lands_where_released() {
        let (mut app, icons) = dock_app(&["a", "b", "c", "d"], &[]);

        drop_between(&mut app, icons[3], icons[0], icons[1]);

        assert_eq!(dock_order(&app), ["addr-a", "addr-d", "addr-b", "addr-c"]);
    }

    #[test]
    fn dropped_favorite_lands_where_released() {
        let (mut app, icons) = dock_app(&["f1", "f2", "f3", "x"], &["f1", "f2", "f3"]);

        drop_between(&mut app, icons[2], icons[0], icons[1]);

        assert_eq!(dock_order(&app), ["addr-f1", "addr-f3", "addr-f2", "addr-x"]);
        assert_eq!(app.world.resource::<Favorites>().0, ["f1", "f3", "f2"]);
    }

    #[test]
    fn non_favorite_can_be_dropped_between_favorites() {
        let (mut app, icons) = dock_app(&["f1", "f2", "x"], &["f1", "f2"]);

        drop_between(&mut app, icons[2], icons[0], icons[1]);

        assert_eq!(dock_order(&app), ["addr-f1", "addr-x", "addr-f2"]);
        assert_eq!(app.world.resource::<Favorites>().0, ["f1", "f2"]);
    }

    #[test]
    fn favorite_can_be_dropped_after_non_favorites() {
        let (mut app, icons) = dock_app(&["f1", "f2", "a", "b"], &["f1", "f2"]);

        drop_between(&mut app, icons[0], icons[2], icons[3]);

        assert_eq!(dock_order(&app), ["addr-f2", "addr-a", "addr-f1", "addr-b"]);
        assert_eq!(app.world.resource::<Favorites>().0, ["f2", "f1"]);
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
