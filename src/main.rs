mod components;
mod systems;
mod types;
mod utils;

use bevy::prelude::*;
use bevy::render::texture::{Image, ImageSampler};
use bevy::window::{PrimaryWindow, Window, WindowPlugin};
use bevy_svg::SvgPlugin;

use components::{
    add_client_address, add_favorite, add_icon_text, spawn_icon_entity, Favorite, Favorites,
};
use std::process::Command;
use types::*;
use utils::hover::{hover_animation_system, hover_system};
use utils::{
    calculate_icon_transform, launch_application, load_clients, load_favorites, save_favorites,
    DockConfig,
};
use utils::IconAnimationState;
use serde_json::Value;
use std::collections::HashSet;
use std::process::{Stdio};
use std::io::{BufRead, BufReader};
use std::sync::{Arc, Mutex, mpsc::{channel, Sender, Receiver}};
use std::env;
use std::os::unix::net::UnixStream;

use crate::systems::*;

static FONT_PATH: &str = "/usr/share/fonts/VictorMono/VictorMonoNerdFont-Medium.ttf";
static FALLBACK_ICON_PATH: &str = "assets/dock_icon.svg";
static ASSETS_ICON_PIN_PATH: &str = "pin_stroke_rounded.svg";

#[derive(Resource, Clone)]
struct HyprlandEventReceiver(Arc<Mutex<Receiver<HyprIpcEvent>>>);

#[derive(Debug, Clone)]
enum HyprIpcEvent {
    OpenWindow {
        address: String,
        workspace: String,
        class: String,
        title: String,
    },
    CloseWindow {
        address: String,
    },
    Other(String),
}

fn main() {
    let client_list = load_clients();
    let favorites = load_favorites();

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
        .insert_resource(DockOrder::default())
        .insert_resource(ScrollState::default())
        .insert_resource(DockConfig::default())
        .insert_resource(IconAnimationState::default())
        .add_event::<IconRemovedEvent>()
        .add_systems(Startup, setup)
        .add_systems(Startup, setup_hyprland_monitor)
        .add_systems(Update, cleanup_duplicate_cameras)
        .add_systems(Update, (
            scroll_system,
            hover_system,
            hover_animation_system,
            icon_scale_animation_system,
            collect_icon_data.before(update_text_positions),
            update_text_positions,
            icon_click_system,
            toggle_favorite_system.in_set(StateUpdate),
            toggle_titles,
            drag_register_click_system,
            drag_check_system,
            drag_update_system,
            drag_end_system.in_set(StateUpdate),
            reset_positions_system,
            reorder_icons_system.in_set(ReorderIcons),
            process_hyprland_events,
        ).chain())
        .run();
}

fn setup(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut images: ResMut<Assets<Image>>,
    client_list: Res<ClientList>,
    windows: Query<&Window, With<PrimaryWindow>>,
    show_titles: Res<ShowTitles>,
    favorites: Res<Favorites>,
    config: Res<DockConfig>,
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

    let start_x = -window_width / 2.0 + config.margin_x;
    let start_y = -window_height / 2.0 + config.margin_y;
    let start_pos = Vec2::new(start_x, start_y);
    let center = Vec2::new(0.0, 0.0);
    let direction = (center - start_pos).normalize_or_zero();

    let mut all_apps: Vec<(String, Option<Client>, bool)> = Vec::new();
    let mut initial_order = Vec::new();

    for fav_class in &favorites.0 {
        let client = client_list
            .0
            .iter()
            .find(|c| &c.class == fav_class)
            .cloned();
        all_apps.push((fav_class.clone(), client.clone(), true));
        if let Some(client) = client {
            initial_order.push(client.address.clone());
        } else {
            initial_order.push(format!("pinned:{}", fav_class));
        }
    }

    for client in &client_list.0 {
        if !favorites.0.contains(&client.class) {
            all_apps.push((client.class.clone(), Some(client.clone()), false));
            initial_order.push(client.address.clone());
        }
    }

    commands.insert_resource(DockOrder(initial_order));

    for (index, (class, client_opt, is_favorite)) in all_apps.iter().enumerate() {
        let (translation, scale) = calculate_icon_transform(
            index,
            start_pos,
            direction,
            &config,
            Vec2::ZERO
        );
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
            commands
                .entity(icon_entity)
                .insert(ClientAddress(client.address.clone()));
            commands
                .entity(icon_entity)
                .insert(ClientClass(client.class.clone()));
        } else if *is_favorite {
            let placeholder_address = format!("pinned:{}", class);
            add_client_address(&mut commands, icon_entity, placeholder_address.clone());
            commands
                .entity(icon_entity)
                .insert(ClientAddress(placeholder_address));
            commands
                .entity(icon_entity)
                .insert(ClientClass(class.clone()));
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
        &ClientAddress,
        Option<&Favorite>,
        &mut Sprite,
        Option<&ClientAddress>,
    )>,
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut favorites: ResMut<Favorites>,
    client_list: Res<ClientList>,
    mut icon_removed_writer: EventWriter<IconRemovedEvent>,
    dock_order: Res<DockOrder>,
) {
    if buttons.just_released(MouseButton::Right) {
        let window = windows.single();
        if let Some(cursor_pos) = window.cursor_position() {
            if let Ok((camera, camera_transform)) = q_camera.get_single() {
                if let Some(world_pos) = camera.viewport_to_world_2d(camera_transform, cursor_pos) {
                    for (
                        entity,
                        transform,
                        client_class,
                        _client_address,
                        favorite_opt,
                        mut sprite,
                        client_address_opt,
                    ) in &mut q_icons
                    {
                        let pos = transform.translation.truncate();
                        let size = Vec2::splat(ICON_SIZE);
                        let rect = Rect::from_center_size(pos, size * 1.1);
                        if rect.contains(world_pos) {
                            toggle_favorite(
                                &mut commands,
                                &asset_server,
                                &mut favorites,
                                entity,
                                &client_class.0,
                                favorite_opt.is_some(),
                                &mut sprite,
                                client_address_opt,
                                client_list,
                                &mut icon_removed_writer,
                                &dock_order,
                            );
                            break;
                        }
                    }
                }
            }
        }
    }
}

fn toggle_favorite(
    commands: &mut Commands,
    asset_server: &Res<AssetServer>,
    favorites: &mut ResMut<Favorites>,
    entity: Entity,
    app_class: &str,
    is_favorite: bool,
    sprite: &mut Sprite,
    q_address: Option<&ClientAddress>,
    client_list: Res<ClientList>,
    icon_removed_writer: &mut EventWriter<IconRemovedEvent>,
    dock_order: &Res<DockOrder>,
) {
    if is_favorite {
        info!("Removing favorite: {}", app_class);
        favorites.0.retain(|c| c != app_class);
        commands.entity(entity).remove::<Favorite>();
        commands.entity(entity).despawn_descendants();

        let address = q_address.map(|a| a.0.clone()).unwrap_or_else(|| format!("pinned:{}", app_class));
        if address.starts_with("pinned:") {
            commands.entity(entity).despawn();
            icon_removed_writer.send(IconRemovedEvent(address.clone()));
            
            let new_order: Vec<String> = dock_order.0
                .iter()
                .filter(|addr| addr != &&address)
                .cloned()
                .collect();

            commands.insert_resource(DockOrder(new_order));
            commands.insert_resource(ReorderTrigger(true));
        }
    } else {
        info!("Adding favorite: {}", app_class);
        if !favorites.0.contains(&app_class.to_string()) {
            favorites.0.push(app_class.to_string());
        }
        commands.entity(entity).insert(Favorite);
        crate::components::add_favorite(commands, entity, asset_server);
    }

    if commands.get_entity(entity).is_some() {
        update_sprite_alpha(
            sprite,
            !is_favorite,
            q_address.is_some() && q_address.map(|a| !a.0.starts_with("pinned:")).unwrap_or(false),
        );
    }

    save_favorites(favorites);
}

fn icon_click_system(
    buttons: Res<Input<MouseButton>>,
    windows: Query<&Window, With<PrimaryWindow>>,
    q_camera: Query<(&Camera, &GlobalTransform), With<MainCamera>>,
    q_icons: Query<(
        &Transform,
        Option<&ClientAddress>,
        Option<&Favorite>,
        &ClientClass,
    )>,
    ui_state: Res<UiState>,
) {
    if buttons.just_released(MouseButton::Left) && ui_state.dragging.is_none() {
        let window = windows.single();
        if let Some(cursor_pos) = window.cursor_position() {
            if let Ok((camera, camera_transform)) = q_camera.get_single() {
                if let Some(world_pos) = camera.viewport_to_world_2d(camera_transform, cursor_pos) {
                    for (transform, address_opt, favorite_opt, client_class) in &q_icons {
                        let icon_position = transform.translation.truncate();
                        let size = Vec2::splat(ICON_SIZE);
                        let rect = Rect::from_center_size(icon_position, size);
                        if rect.contains(world_pos) {
                            if let Some(address) = address_opt {
                                if address.0.starts_with("pinned:") {
                                    info!("launch_application, {}", client_class.0);
                                    launch_application(&client_class.0);
                                } else {
                                    focus_client(&address.0);
                                }
                            } else if favorite_opt.is_some() {
                                info!("launch_application, {}", client_class.0);
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
    info!("Executing hyprctl dispatch focuswindow {}", full_address);
    let output = Command::new("hyprctl")
        .args(["dispatch", "focuswindow", &full_address])
        .output();
    match output {
        Ok(result) => {
            if !result.status.success() {
                warn!(
                    "Failed to focus window: {:?}",
                    String::from_utf8_lossy(&result.stderr)
                );
            }
        }
        Err(e) => error!("Error executing hyprctl: {:?}", e),
    }
}

fn update_client_list_system(
    mut client_list: ResMut<ClientList>,
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    windows: Query<&Window, With<PrimaryWindow>>,
    config: Res<DockConfig>,
    favorites: Res<Favorites>,
    show_titles: Res<ShowTitles>,
    q_entities: Query<(Entity, Option<&ClientAddress>)>,
    mut images: ResMut<Assets<Image>>,
) {
    match crate::utils::loader::get_current_clients() {
        Ok(current_windows) => {
            let current_addresses: HashSet<String> = current_windows
                .iter()
                .map(|c| c.address.clone())
                .collect();
            
            let old_addresses: HashSet<String> = client_list.0
                .iter()
                .map(|c| c.address.clone())
                .collect();

            let new_windows: Vec<Client> = current_windows
                .iter()
                .filter(|c| !old_addresses.contains(&c.address))
                .cloned()
                .collect();

            let closed_windows: Vec<String> = old_addresses
                .difference(&current_addresses)
                .cloned()
                .collect();

            if !new_windows.is_empty() || !closed_windows.is_empty() {
                client_list.0 = current_windows.clone();

                for address in closed_windows {
                    if let Some((entity, _)) = q_entities.iter().find(|(_, addr)| addr.map(|a| a.0 == address).unwrap_or(false)) {
                        commands.entity(entity).despawn();
                    }
                }

                let window = windows.single();
                let start_x = -window.width() / 2.0 + config.margin_x;
                let start_y = -window.height() / 2.0 + config.margin_y;
                let start_pos = Vec2::new(start_x, start_y);
                let center = Vec2::new(0.0, 0.0);
                let direction = (center - start_pos).normalize_or_zero();

                for (index, client) in new_windows.iter().enumerate() {
                    let (translation, scale) = calculate_icon_transform(
                        index,
                        start_pos,
                        direction,
                        &config,
                        Vec2::ZERO
                    );
                    let transform = Transform {
                        translation,
                        scale: Vec3::splat(scale),
                        ..default()
                    };

                    let icon_entity = spawn_icon_entity(
                        &mut commands,
                        &mut images,
                        &asset_server,
                        &client.class,
                        transform,
                        scale,
                        1.0,
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

                    add_client_address(&mut commands, icon_entity, client.address.clone());
                    commands.entity(icon_entity).insert(ClientAddress(client.address.clone()));
                    commands.entity(icon_entity).insert(ClientClass(client.class.clone()));

                    if show_titles.0 {
                        add_icon_text(
                            &mut commands,
                            icon_entity,
                            &client.class,
                            transform,
                            scale,
                            &asset_server,
                        );
                    }
                }
            }
        }
        Err(e) => {
            error!("Failed to get current windows: {:?}", e);
        }
    }
}

fn setup_hyprland_monitor(mut commands: Commands) {
    let (event_sender, event_receiver) = channel();
    let event_receiver = Arc::new(Mutex::new(event_receiver));

    std::thread::spawn(move || {
        let xdg_runtime_dir = match env::var("XDG_RUNTIME_DIR") {
            Ok(val) => val,
            Err(_) => return,
        };
        let hyprland_instance_signature = match env::var("HYPRLAND_INSTANCE_SIGNATURE") {
            Ok(val) => val,
            Err(_) => return,
        };
        let socket_path = format!("{}/hypr/{}/.socket2.sock", xdg_runtime_dir, hyprland_instance_signature);
        let stream = match UnixStream::connect(socket_path) {
            Ok(s) => s,
            Err(_) => return,
        };
        let reader = BufReader::new(stream);

        for line in reader.lines().flatten() {
            if let Some(rest) = line.strip_prefix("openwindow>>") {
                let mut parts = rest.splitn(4, ',');
                if let (Some(address), Some(workspace), Some(class), Some(title)) =
                    (parts.next(), parts.next(), parts.next(), parts.next())
                {
                    let _ = event_sender.send(HyprIpcEvent::OpenWindow {
                        address: address.to_string(),
                        workspace: workspace.to_string(),
                        class: class.to_string(),
                        title: title.to_string(),
                    });
                }
            } else if let Some(rest) = line.strip_prefix("closewindow>>") {
                let address = rest.trim().to_string();
                let _ = event_sender.send(HyprIpcEvent::CloseWindow { address });
            } else {
                let _ = event_sender.send(HyprIpcEvent::Other(line));
            }
        }
    });

    commands.insert_resource(HyprlandEventReceiver(event_receiver));
}

fn process_hyprland_events(
    mut commands: Commands,
    mut client_list: ResMut<ClientList>,
    mut dock_order: ResMut<DockOrder>,
    mut reorder_trigger: ResMut<ReorderTrigger>,
    event_receiver: Res<HyprlandEventReceiver>,
    asset_server: Res<AssetServer>,
    windows: Query<&Window, With<PrimaryWindow>>,
    config: Res<DockConfig>,
    favorites: Res<Favorites>,
    show_titles: Res<ShowTitles>,
    q_entities: Query<(Entity, Option<&ClientAddress>)>,
    mut images: ResMut<Assets<Image>>,
) {
    let event_receiver = event_receiver.0.lock().unwrap();
    while let Ok(event) = event_receiver.try_recv() {
        match event {
            HyprIpcEvent::OpenWindow { address, workspace, class, title } => {
                let client = Client {
                    address: address.clone(),
                    class: class.clone(),
                    name: Some(title.clone()),
                };
                if !client_list.0.iter().any(|c| c.address == client.address) {
                    client_list.0.push(client.clone());
                    dock_order.0.push(address.clone());
                    reorder_trigger.0 = true;
                    let window = windows.single();
                    let start_x = -window.width() / 2.0 + config.margin_x;
                    let start_y = -window.height() / 2.0 + config.margin_y;
                    let start_pos = Vec2::new(start_x, start_y);
                    let center = Vec2::new(0.0, 0.0);
                    let direction = (center - start_pos).normalize_or_zero();

                    let (translation, scale) = calculate_icon_transform(
                        0,
                        start_pos,
                        direction,
                        &config,
                        Vec2::ZERO
                    );
                    let transform = Transform {
                        translation,
                        scale: Vec3::splat(scale),
                        ..default()
                    };

                    let icon_entity = spawn_icon_entity(
                        &mut commands,
                        &mut images,
                        &asset_server,
                        &client.class,
                        transform,
                        scale,
                        1.0,
                        0,
                    );

                    commands.entity(icon_entity).insert(HoverTarget {
                        original_position: translation.truncate(),
                        original_z: translation.z,
                        original_scale: scale,
                        index: 0,
                        is_hovered: false,
                        hover_exit_timer: None,
                    });

                    add_client_address(&mut commands, icon_entity, client.address.clone());
                    commands.entity(icon_entity).insert(ClientAddress(client.address.clone()));
                    commands.entity(icon_entity).insert(ClientClass(client.class.clone()));

                    if show_titles.0 {
                        add_icon_text(
                            &mut commands,
                            icon_entity,
                            &client.class,
                            transform,
                            scale,
                            &asset_server,
                        );
                    }
                }
            }
            HyprIpcEvent::CloseWindow { address } => {
                if let Some((entity, _)) = q_entities.iter().find(|(_, addr)| addr.map(|a| a.0 == address).unwrap_or(false)) {
                    commands.entity(entity).despawn();
                }
                client_list.0.retain(|c| c.address != address);
                dock_order.0.retain(|a| a != &address);
                reorder_trigger.0 = true;
            }
            HyprIpcEvent::Other(_) => {}
        }
    }
}
