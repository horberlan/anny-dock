mod components;
mod utils;

use bevy::prelude::*;
use bevy::render::texture::{Image, ImageSampler};
use bevy::window::{PrimaryWindow, Window, WindowPlugin};
use bevy_svg::SvgPlugin;

use components::{
    add_client_address, add_favorite, add_icon_text, spawn_icon_entity, Favorite, Favorites,
};
use serde::Deserialize;
use std::collections::HashMap;
use std::process::Command;
use utils::{calculate_icon_transform, load_clients, load_favorites, save_favorites, DockConfig};

use crate::utils::launch_application;

#[derive(Deserialize, Debug, Clone)]
struct Client {
    class: String,
    address: String,
    #[serde(default)]
    name: Option<String>,
}

#[derive(Component)]
struct ClientIcon;

#[derive(Component)]
struct ClientAddress(String);

#[derive(Component)]
struct ClientClass(String);

#[derive(Resource)]
struct ClientList(Vec<Client>);

#[derive(Resource)]
struct ShowTitles(bool);

#[derive(Component)]
struct Dragging {
    offset: Vec2,
}

#[derive(Resource, Default)]
struct UiState {
    dragging: Option<Entity>,
    click_origin: Option<Vec2>,
}

#[derive(Component)]
struct HoverTarget {
    original_position: Vec2,
    original_z: f32,
    original_scale: f32,
    index: usize,
    is_hovered: bool,
    hover_exit_timer: Option<Timer>,
}

#[derive(Component)]
struct IconText(Entity);

#[derive(Resource, Default)]
struct IconPositions(HashMap<Entity, (Vec3, Vec3)>);

#[derive(Resource)]
struct ReorderTrigger(bool);

impl Default for ReorderTrigger {
    fn default() -> Self {
        ReorderTrigger(false)
    }
}

#[derive(Component)]
struct MainCamera;

#[derive(SystemSet, Debug, Hash, PartialEq, Eq, Clone)]
struct StateUpdate;

#[derive(SystemSet, Debug, Hash, PartialEq, Eq, Clone)]
struct ReorderIcons;

#[derive(Resource, Default)]
struct DockOrder(Vec<String>);

static FONT_PATH: &str = "/usr/share/fonts/VictorMono/VictorMonoNerdFont-Medium.ttf";
static FALLBACK_ICON_PATH: &str = "assets/dock_icon.svg";
static ASSETS_ICON_PIN_PATH: &str = "pin_stroke_rounded.svg";
const ICON_SIZE: f32 = 56.0;

fn main() {
    let client_list = load_clients();
    let favorites = load_favorites();

    let mut initial_order = Vec::new();

    initial_order.extend(favorites.0.iter().cloned());

    for client in &client_list {
        if !favorites.0.contains(&client.class) {
            initial_order.push(client.class.clone());
        }
    }

    App::new()
        .insert_resource(Msaa::Sample4)
        .add_plugins(
            DefaultPlugins
                .set(WindowPlugin {
                    primary_window: Some(Window {
                        transparent: true,
                        decorations: false,
                        ..default()
                    }),
                    ..default()
                })
                .set(ImagePlugin {
                    default_sampler: ImageSampler::linear_descriptor(),
                    ..default()
                }),
        )
        .add_plugins(SvgPlugin)
        .insert_resource(ClearColor(Color::NONE))
        .insert_resource(ClientList(client_list))
        .insert_resource(IconPositions::default())
        .insert_resource(ShowTitles(false))
        .insert_resource(UiState::default())
        .insert_resource(favorites)
        .insert_resource(ReorderTrigger::default())
        .insert_resource(DockOrder(initial_order))
        .add_systems(Startup, setup)
        .add_systems(Update, cleanup_duplicate_cameras)
        .add_systems(Update, hover_system)
        .add_systems(Update, hover_animation_system)
        .add_systems(Update, collect_icon_data.before(update_text_positions))
        .add_systems(Update, update_text_positions)
        .add_systems(Update, icon_click_system)
        .add_systems(Update, toggle_favorite_system.in_set(StateUpdate))
        .add_systems(Update, toggle_titles)
        .add_systems(Update, drag_register_click_system)
        .add_systems(Update, drag_check_system)
        .add_systems(Update, drag_update_system)
        .add_systems(Update, drag_end_system.in_set(StateUpdate))
        .add_systems(Update, reset_positions_system)
        .add_systems(PostUpdate, reorder_icons_system.in_set(ReorderIcons))
        .run();
}
fn cleanup_duplicate_cameras(mut commands: Commands, query: Query<(Entity, &MainCamera)>) {
    let mut found_camera = false;
    for (entity, _) in query.iter() {
        if found_camera {
            commands.entity(entity).despawn_recursive();
        } else {
            found_camera = true;
        }
    }
}

fn setup(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut images: ResMut<Assets<Image>>,
    client_list: Res<ClientList>,
    windows: Query<&Window, With<PrimaryWindow>>,
    show_titles: Res<ShowTitles>,
    favorites: Res<Favorites>,
) {
    commands
        .spawn(Camera2dBundle {
            transform: Transform {
                translation: Vec3::new(0.0, 0.0, 100.0),
                ..default()
            },
            ..default()
        })
        .insert(MainCamera);

    let window = windows.single();
    let window_width = window.width();
    let window_height = window.height();

    let config = DockConfig::default();
    let start_x = -window_width / 2.0 + config.margin_x;
    let start_y = -window_height / 2.0 + config.margin_y;
    let start_pos = Vec2::new(start_x, start_y);
    let center = Vec2::new(0.0, 0.0);
    let direction = (center - start_pos).normalize_or_zero();

    let mut all_apps: Vec<(String, Option<Client>, bool)> = Vec::new();
    for fav_class in &favorites.0 {
        let client = client_list
            .0
            .iter()
            .find(|c| &c.class == fav_class)
            .cloned();
        all_apps.push((fav_class.clone(), client, true));
    }
    for client in &client_list.0 {
        if !favorites.0.contains(&client.class) {
            all_apps.push((client.class.clone(), Some(client.clone()), false));
        }
    }

    for (index, (class, client_opt, is_favorite)) in all_apps.iter().enumerate() {
        let (translation, scale) = calculate_icon_transform(index, start_pos, direction, &config);
        let transform = Transform {
            translation,
            scale: Vec3::splat(scale),
            ..default()
        };
        let alpha = if *is_favorite && client_opt.is_none() {
            0.2
        } else {
            1.0
        };

        let icon_entity = spawn_icon_entity(
            &mut commands,
            &mut images,
            &asset_server,
            class,
            transform,
            scale,
            alpha,
            index,
        );

        commands.entity(icon_entity).insert(HoverTarget {
            original_position: translation.truncate(),
            original_z: translation.z,
            original_scale: scale,
            index,
            is_hovered: false,
            hover_exit_timer: None,
        });

        if let Some(client) = client_opt {
            add_client_address(&mut commands, icon_entity, client.address.clone());
        }
        if *is_favorite {
            add_favorite(&mut commands, icon_entity, &asset_server);
        }
        if show_titles.0 {
            add_icon_text(
                &mut commands,
                icon_entity,
                class,
                transform,
                scale,
                &asset_server,
            );
        }
    }
}

fn hover_system(
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

fn hover_animation_system(
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

fn collect_icon_data(
    query: Query<(Entity, &Transform, &HoverTarget)>,
    mut icon_positions: ResMut<IconPositions>,
) {
    icon_positions.0.clear();
    for (entity, transform, _hover) in query.iter() {
        icon_positions
            .0
            .insert(entity, (transform.translation, transform.scale));
    }
}

fn update_text_positions(
    mut text_query: Query<(&mut Transform, &IconText)>,
    icon_positions: Res<IconPositions>,
) {
    for (mut text_transform, icon_text) in text_query.iter_mut() {
        if let Some((position, scale)) = icon_positions.0.get(&icon_text.0) {
            text_transform.translation =
                Vec3::new(position.x, position.y - 30.0, position.z - 0.01);
            text_transform.scale = *scale;
        }
    }
}

fn update_sprite_alpha(sprite: &mut Sprite, has_favorite: bool, has_address: bool) {
    sprite.color = if has_favorite && !has_address {
        Color::rgba(1.0, 1.0, 1.0, 0.2)
    } else {
        Color::rgba(1.0, 1.0, 1.0, 1.0)
    };
}

fn toggle_favorite_system(
    buttons: Res<Input<MouseButton>>,
    windows: Query<&Window, With<PrimaryWindow>>,
    q_camera: Query<(&Camera, &GlobalTransform), With<MainCamera>>,
    mut q_icons: Query<(
        Entity,
        &Transform,
        &ClientClass,
        Option<&Favorite>,
        &mut Sprite,
    )>,
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut favorites: ResMut<Favorites>,
) {
    if buttons.just_released(MouseButton::Right) {
        let window = windows.single();
        if let Some(cursor_pos) = window.cursor_position() {
            if let Ok((camera, camera_transform)) = q_camera.get_single() {
                if let Some(world_pos) = camera.viewport_to_world_2d(camera_transform, cursor_pos) {
                    for (entity, transform, client_class, favorite_opt, mut sprite) in &mut q_icons
                    {
                        let pos = transform.translation.truncate();
                        let size = Vec2::splat(ICON_SIZE);
                        let rect = Rect::from_center_size(pos, size * 1.1);
                        if rect.contains(world_pos) {
                            let app_class = client_class.0.clone();
                            if favorite_opt.is_some() {
                                favorites.0.retain(|c| c != &app_class);
                                commands.entity(entity).remove::<Favorite>();
                                commands.entity(entity).despawn_descendants();
                            } else {
                                if !favorites.0.contains(&app_class) {
                                    favorites.0.push(app_class.clone());
                                }
                                commands.entity(entity).insert(Favorite);
                                add_favorite(&mut commands, entity, &asset_server);
                            }
                            update_sprite_alpha(&mut sprite, favorite_opt.is_none(), true);
                            save_favorites(&favorites);
                            break;
                        }
                    }
                }
            }
        }
    }
}
fn icon_click_system(
    buttons: Res<Input<MouseButton>>,
    windows: Query<&Window, With<PrimaryWindow>>,
    q_camera: Query<(&Camera, &GlobalTransform), With<MainCamera>>,
    q_icons: Query<
        (
            &Transform,
            Option<&ClientAddress>,
            Option<&Favorite>,
            &ClientClass,
        ),
        With<ClientIcon>,
    >,
    ui_state: Res<UiState>,
) {
    if buttons.just_released(MouseButton::Left) && ui_state.dragging.is_none() {
        let window = windows.single();
        if let Some(cursor_pos) = window.cursor_position() {
            if let Ok((camera, camera_transform)) = q_camera.get_single() {
                if let Some(world_pos) = camera.viewport_to_world_2d(camera_transform, cursor_pos) {
                    for (transform, address_opt, favorite_opt, client_class) in &q_icons {
                        let pos = transform.translation.truncate();
                        let size = Vec2::splat(ICON_SIZE);
                        let rect = Rect::from_center_size(pos, size);
                        if rect.contains(world_pos) {
                            if let Some(address) = address_opt {
                                focus_client(&address.0);
                            } else if favorite_opt.is_some() {
                                launch_application(&client_class.0);
                            }
                            break;
                        }
                    }
                }
            }
        }
    }
}

fn focus_client(address: &str) {
    let full_address = format!("address:{}", address.trim_start_matches("address:"));
    let _ = Command::new("hyprctl")
        .args(["dispatch", "focuswindow", &full_address])
        .output();
}

fn toggle_titles(
    mut commands: Commands,
    mut show_titles: ResMut<ShowTitles>,
    keyboard_input: Res<Input<KeyCode>>,
    q_icons: Query<(Entity, &Transform, &ClientClass, &HoverTarget)>,
    asset_server: Res<AssetServer>,
    q_texts: Query<Entity, With<IconText>>,
) {
    if keyboard_input.just_pressed(KeyCode::T) {
        show_titles.0 = !show_titles.0;

        if show_titles.0 {
            for (entity, transform, class, hover) in q_icons.iter() {
                add_icon_text(
                    &mut commands,
                    entity,
                    &class.0,
                    *transform,
                    hover.original_scale,
                    &asset_server,
                );
            }
        } else {
            for entity in q_texts.iter() {
                commands.entity(entity).despawn_recursive();
            }
        }
    }
}

fn drag_register_click_system(
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

fn drag_check_system(
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

fn drag_update_system(
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

fn drag_end_system(
    mut commands: Commands,
    mouse_button: Res<Input<MouseButton>>,
    mut ui_state: ResMut<UiState>,
    mut dock_order: ResMut<DockOrder>,
    q_icons: Query<(Entity, &Transform, &ClientClass)>,
    mut reorder_trigger: ResMut<ReorderTrigger>,
) {
    if mouse_button.just_released(MouseButton::Left) && ui_state.dragging.is_some() {
        if let Some(dragged_entity) = ui_state.dragging {
            commands.entity(dragged_entity).remove::<Dragging>();
            ui_state.dragging = None;

            let dragged_x = q_icons
                .get(dragged_entity)
                .map(|(_, transform, _)| transform.translation.x)
                .unwrap();
            let dragged_class = q_icons
                .get(dragged_entity)
                .map(|(_, _, class)| class.0.clone())
                .unwrap();

            let mut other_icons: Vec<(String, f32)> = q_icons
                .iter()
                .filter(|(e, _, _)| *e != dragged_entity)
                .map(|(_, transform, class)| (class.0.clone(), transform.translation.x))
                .collect();

            other_icons.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap());

            let new_index = other_icons
                .iter()
                .position(|(_, x)| *x > dragged_x)
                .unwrap_or(other_icons.len());

            let mut new_order = Vec::new();
            for (i, (class, _)) in other_icons.iter().enumerate() {
                if i == new_index {
                    new_order.push(dragged_class.clone());
                }
                new_order.push(class.clone());
            }
            if new_index == other_icons.len() {
                new_order.push(dragged_class.clone());
            }

            dock_order.0 = new_order;
            reorder_trigger.0 = true;
        }
    }
}
fn reorder_icons_system(
    mut q_icons: Query<(Entity, &ClientClass, &mut Transform, &mut HoverTarget)>,
    dock_order: Res<DockOrder>,
    windows: Query<&Window, With<PrimaryWindow>>,
    mut reorder_trigger: ResMut<ReorderTrigger>,
) {
    if !reorder_trigger.0 {
        return;
    }

    let window = windows.single();
    let config = DockConfig::default();
    let start_x = -window.width() / 2.0 + config.margin_x;
    let start_y = -window.height() / 2.0 + config.margin_y;
    let start_pos = Vec2::new(start_x, start_y);
    let center = Vec2::ZERO;
    let direction = (center - start_pos).normalize_or_zero();

    for (index, class) in dock_order.0.iter().enumerate() {
        let (translation, scale) = calculate_icon_transform(index, start_pos, direction, &config);
        for (entity, icon_class, mut transform, mut hover) in q_icons.iter_mut() {
            if icon_class.0 == *class {
                transform.translation = translation;
                transform.scale = Vec3::splat(scale);
                hover.original_position = translation.truncate();
                hover.original_z = translation.z;
                hover.original_scale = scale;
                hover.index = index;
                break;
            }
        }
    }

    reorder_trigger.0 = false;
}
fn reset_positions_system(
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
