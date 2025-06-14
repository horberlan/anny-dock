mod components;
mod systems;
mod types;
mod utils;
mod config;

use bevy::prelude::*;
use bevy::render::texture::Image;
use bevy::window::{PrimaryWindow, Window, WindowPlugin};
use bevy_embedded_assets::EmbeddedAssetPlugin;
use bevy_svg::SvgPlugin;

use components::{
    add_client_address, add_favorite, add_icon_text, spawn_icon_entity, Favorite, Favorites, FavoritePin,
};
use std::collections::HashSet;
use std::process::Command;
use types::*;
use utils::hover::{hover_animation_system, hover_system};
use utils::{
    calculate_icon_transform, launch_application, load_clients, load_favorites, save_favorites,
    update_sprite_alpha, IconAnimationState,
};
use config::{load_config, Config};

use std::env;
use std::io::{BufRead, BufReader};
use std::os::unix::net::UnixStream;
use std::sync::{mpsc::channel, Arc, Mutex};

use crate::systems::*;
use systems::animation::ScrollAnimationState;

fn main() {
    let config = load_config();
    let client_list = load_clients();
    let favorites = load_favorites();

    App::new()
        
        .insert_resource(Msaa::Sample4)
        .add_plugins(EmbeddedAssetPlugin::default())
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
                    default_sampler: bevy::render::texture::ImageSamplerDescriptor::linear(),
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
        .insert_resource(config)
        .insert_resource(IconAnimationState::default())
        .insert_resource(ScrollAnimationState::default())
        .add_event::<IconRemovedEvent>()
        .add_systems(Startup, setup)
        .add_systems(Startup, setup_hyprland_monitor)
        
        .add_systems(Update, cleanup_duplicate_cameras)
        .add_systems(
            Update,
            (
                scroll_system,
                scroll_with_arrows,
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
                exit_on_esc_or_q,
                keybind_launch_visible_icons,
            )
                .chain(),
        )
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
    config: Res<Config>,
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
    let center = Vec2::new(0.0, window_height * config.tilt_y);
    let direction = (center - start_pos).normalize_or_zero();

    let mut all_apps: Vec<(String, Option<Client>, bool)> = Vec::new();
    let mut initial_order = Vec::new();
    let mut processed_favorites = HashSet::new();

    for fav_class in &favorites.0 {
        if processed_favorites.contains(fav_class) {
            continue;
        }

        let client = client_list.0.iter().find(|c| &c.class == fav_class).cloned();
        all_apps.push((fav_class.clone(), client.clone(), true));

        if let Some(client) = client {
            initial_order.push(client.address.clone());
        } else {
            initial_order.push(format!("pinned:{}", fav_class));
        }
        processed_favorites.insert(fav_class.clone());
    }

    for client in &client_list.0 {
        if !favorites.0.contains(&client.class) {
            all_apps.push((client.class.clone(), Some(client.clone()), false));
            initial_order.push(client.address.clone());
        }
    }

    commands.insert_resource(DockOrder(initial_order));

    for (index, (class, client_opt, is_favorite)) in all_apps.iter().enumerate() {
        let (translation, scale) =
            calculate_icon_transform(index, start_pos, direction, &config, Vec2::ZERO);
        let transform = Transform {
            translation,
            scale: Vec3::splat(scale),
            ..default()
        };
        let alpha = if *is_favorite && client_opt.is_none() {
            0.5
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
            add_favorite(&mut commands, icon_entity, &mut images, &config);
        }
        if show_titles.0 {
            add_icon_text(
                &mut commands,
                icon_entity,
                class,
                transform,
                scale,
                &asset_server,
                &config,
            );
        }
    }
}

fn toggle_favorite_system(
    mut commands: Commands,
    mut images: ResMut<Assets<Image>>,
    mut favorites: ResMut<Favorites>,
    mut reorder_trigger: ResMut<ReorderTrigger>,
    mut dock_order: ResMut<DockOrder>,
    mut q_icons: Query<(
        Entity,
        &ClientClass,
        Option<&mut Sprite>,
        Option<&ClientAddress>,
        Option<&Favorite>,
        &HoverTarget,
        &Transform,
        Option<&Children>,
    )>,
    config: Res<Config>,
    q_pins: Query<Entity, With<FavoritePin>>,
    mouse_button: Res<Input<MouseButton>>,
) {
    if mouse_button.just_released(MouseButton::Right) {
        if let Some((entity, class, mut sprite_opt, address_opt, favorite_opt, _hover, _transform, children)) =
            q_icons.iter_mut().find(|(_, _, _, _, _, hover, _, _)| hover.is_hovered)
        {
            toggle_favorite(
                &mut commands,
                &mut images,
                &mut favorites,
                &mut reorder_trigger,
                &mut dock_order,
                entity,
                &class.0,
                favorite_opt.is_some(),
                sprite_opt.as_deref_mut().unwrap(),
                address_opt,
                &config,
                children,
                &q_pins,
            );
        }
    }
}

fn toggle_favorite(
    commands: &mut Commands,
    images: &mut Assets<Image>,
    favorites: &mut ResMut<Favorites>,
    reorder_trigger: &mut ResMut<ReorderTrigger>,
    dock_order: &mut ResMut<DockOrder>,
    entity: Entity,
    app_class: &str,
    is_favorite: bool,
    sprite: &mut Sprite,
    q_address: Option<&ClientAddress>,
    config: &Res<Config>,
    children: Option<&Children>,
    q_pins: &Query<Entity, With<FavoritePin>>,
) {
    if is_favorite {
        info!("Removing favorite: {}", app_class);
        favorites.0.retain(|f| f != app_class);

        let is_running = q_address.map_or(false, |addr| !addr.0.starts_with("pinned:"));

        if is_running {
            // App is running, just remove favorite status and pin
            commands.entity(entity).remove::<Favorite>();
            if let Some(children) = children {
                for &child in children.iter() {
                    if q_pins.get(child).is_ok() {
                        commands.entity(child).despawn();
                    }
                }
            }
        } else {
            // App is not running, it only exists because it was a favorite.
            // Despawn the whole thing.
            let pinned_addr = format!("pinned:{}", app_class);
            dock_order.0.retain(|a| a != &pinned_addr);
            commands.entity(entity).despawn_recursive();
        }

        reorder_trigger.0 = true;
    } else {
        info!("Adding favorite: {}", app_class);
        if !favorites.0.contains(&app_class.to_string()) {
            favorites.0.push(app_class.to_string());
        }
        add_favorite(commands, entity, images, config);
        reorder_trigger.0 = true;
    }
    save_favorites(favorites);

    if commands.get_entity(entity).is_some() {
        let is_running =
            q_address.is_some() && q_address.map(|a| !a.0.starts_with("pinned:")).unwrap_or(false);
        update_sprite_alpha(sprite, !is_favorite, is_running);
    }
}

fn icon_click_system(
    q_camera: Query<(&Camera, &GlobalTransform), With<MainCamera>>,
    q_icons: Query<(
        Entity,
        &ClientAddress,
        &ClientClass,
        &HoverTarget,
        &Transform,
    )>,
    mouse_button: Res<Input<MouseButton>>,
    windows: Query<&Window, With<PrimaryWindow>>,
    ui_state: Res<UiState>,
    config: Res<Config>,
) {
    if mouse_button.just_released(MouseButton::Left) && ui_state.dragging.is_none() {
        let window = windows.single();
        if let Some(cursor_pos) = window.cursor_position() {
            if let Ok((camera, camera_transform)) = q_camera.get_single() {
                if let Some(world_pos) = camera.viewport_to_world_2d(camera_transform, cursor_pos) {
                    for (_entity, address, client_class, hover, transform) in q_icons.iter() {
                        let icon_position = transform.translation.truncate();
                        let size = Vec2::splat(config.icon_size);
                        let rect = Rect::from_center_size(icon_position, size);
                        if rect.contains(world_pos) && hover.is_hovered {
                            if address.0.starts_with("pinned:") {
                                launch_application(&client_class.0);
                            } else {
                                focus_client(&address.0);
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
    let full_address = if address.starts_with("address:") {
        address.to_string()
    } else {
        format!("address:{}", address)
    };
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

#[allow(dead_code)]
fn update_client_list_system(
    mut client_list: ResMut<ClientList>,
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    windows: Query<&Window, With<PrimaryWindow>>,
    config: Res<Config>,
    favorites: Res<Favorites>,
    show_titles: Res<ShowTitles>,
    mut q_entities: Query<(
        Entity,
        Option<&ClientAddress>,
        Option<&ClientClass>,
        Option<&mut Sprite>,
    )>,
    mut images: ResMut<Assets<Image>>,
    mut dock_order: ResMut<DockOrder>,
    mut reorder_trigger: ResMut<ReorderTrigger>,
) {
    match crate::utils::loader::get_current_clients() {
        Ok(current_windows) => {
            let current_addresses: HashSet<String> =
                current_windows.iter().map(|c| c.address.clone()).collect();
            let old_addresses: HashSet<String> =
                client_list.0.iter().map(|c| c.address.clone()).collect();

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

                process_closed_windows(
                    &closed_windows,
                    &favorites,
                    &mut q_entities,
                    &mut dock_order,
                    &mut commands,
                );

                process_new_windows(
                    &new_windows,
                    &mut q_entities,
                    &mut commands,
                    &mut images,
                    &asset_server,
                    &windows,
                    &config,
                    &show_titles,
                    &mut dock_order,
                    &mut client_list,
                    &mut reorder_trigger,
                );
            }
        }
        Err(e) => {
            error!("Failed to get current windows: {:?}", e);
        }
    }
}

#[allow(dead_code)]
fn process_closed_windows(
    closed_windows: &[String],
    favorites: &Favorites,
    q_entities: &mut Query<(
        Entity,
        Option<&ClientAddress>,
        Option<&ClientClass>,
        Option<&mut Sprite>,
    )>,
    dock_order: &mut DockOrder,
    commands: &mut Commands,
) {
    for address in closed_windows {
        if let Some((entity, addr_opt, class_opt, Some(mut sprite))) =
            q_entities.iter_mut().find(|(_, addr, _, sprite)| {
                addr.as_ref().map(|a| a.0 == *address).unwrap_or(false) && sprite.is_some()
            })
        {
            if let (Some(_addr), Some(class)) = (addr_opt, class_opt) {
                if favorites.0.contains(&class.0) {
                    handle_close_pinned_window(
                        entity,
                        address,
                        &class.0,
                        &mut sprite,
                        dock_order,
                        commands,
                    );
                } else {
                    commands.entity(entity).despawn();
                    dock_order.0.retain(|a| a != address);
                }
            }
        }
    }
}

#[allow(dead_code)]
fn process_new_windows(
    new_windows: &[Client],
    q_entities: &mut Query<(
        Entity,
        Option<&ClientAddress>,
        Option<&ClientClass>,
        Option<&mut Sprite>,
    )>,
    commands: &mut Commands,
    images: &mut ResMut<Assets<Image>>,
    asset_server: &Res<AssetServer>,
    windows: &Query<&Window, With<PrimaryWindow>>,
    config: &Res<Config>,
    show_titles: &Res<ShowTitles>,
    dock_order: &mut DockOrder,
    client_list: &mut ResMut<ClientList>,
    reorder_trigger: &mut ResMut<ReorderTrigger>,
) {

    for (_index, client) in new_windows.iter().enumerate() {
        if let Some((entity, _, _, Some(mut sprite))) = q_entities.iter_mut().find(|(_, addr_opt, class_opt, _)| {
            addr_opt.map_or(false, |a| a.0.starts_with("pinned:"))
                && class_opt.map_or(false, |c| c.0 == client.class)
        }) {
            commands
                .entity(entity)
                .insert(ClientAddress(client.address.clone()));
            handle_open_pinned_window(entity, client, &mut sprite, dock_order);
            client_list.0.push(client.clone());
            reorder_trigger.0 = true;
            continue;
        }

        client_list.0.push(client.clone());
        dock_order.0.push(client.address.clone());
        reorder_trigger.0 = true;
        let window = windows.single();
        let start_x = -window.width() / 2.0 + config.margin_x;
        let start_y = -window.height() / 2.0 + config.margin_y;
        let start_pos = Vec2::new(start_x, start_y);
        let center = Vec2::new(0.0, window.height() * config.tilt_y);
        let direction = (center - start_pos).normalize_or_zero();

        let (translation, scale) =
            calculate_icon_transform(0, start_pos, direction, config, Vec2::ZERO);
        let transform = Transform {
            translation,
            scale: Vec3::splat(scale),
            ..default()
        };

        let icon_entity = spawn_icon_entity(
            commands,
            images,
            asset_server,
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

        add_client_address(commands, icon_entity, client.address.clone());
        commands
            .entity(icon_entity)
            .insert(ClientAddress(client.address.clone()));
        commands
            .entity(icon_entity)
            .insert(ClientClass(client.class.clone()));

        if show_titles.0 {
            add_icon_text(
                commands,
                icon_entity,
                &client.class,
                transform,
                scale,
                asset_server,
                config,
            );
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
        let socket_path = format!(
            "{}/hypr/{}/.socket2.sock",
            xdg_runtime_dir, hyprland_instance_signature
        );
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
                let _ = event_sender.send(HyprIpcEvent::Other);
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
    windows: Query<&Window, With<PrimaryWindow>>,
    config: Res<Config>,
    favorites: Res<Favorites>,
    show_titles: Res<ShowTitles>,
    asset_server: Res<AssetServer>,
    mut q_entities: Query<(
        Entity,
        Option<&ClientAddress>,
        Option<&ClientClass>,
        Option<&mut Sprite>,
    )>,
    mut images: ResMut<Assets<Image>>,
) {
    let event_receiver = event_receiver.0.lock().unwrap();
    while let Ok(event) = event_receiver.try_recv() {
        match event {
            HyprIpcEvent::OpenWindow {
                address,
                workspace: _,
                class,
                title,
            } => {
                handle_hypr_open_window(
                    &mut commands,
                    &mut client_list,
                    &mut dock_order,
                    &mut reorder_trigger,
                    &asset_server,
                    &windows,
                    &config,
                    &favorites,
                    &show_titles,
                    &mut q_entities,
                    &mut images,
                    address,
                    class,
                    title,
                );
            }
            HyprIpcEvent::CloseWindow { address } => {
                handle_hypr_close_window(
                    &mut commands,
                    &mut client_list,
                    &mut dock_order,
                    &mut reorder_trigger,
                    &favorites,
                    &mut q_entities,
                    address,
                );
            }
            HyprIpcEvent::Other => {}
        }
    }
}

fn handle_hypr_open_window(
    commands: &mut Commands,
    client_list: &mut ResMut<ClientList>,
    dock_order: &mut ResMut<DockOrder>,
    reorder_trigger: &mut ResMut<ReorderTrigger>,
    asset_server: &Res<AssetServer>,
    windows: &Query<&Window, With<PrimaryWindow>>,
    config: &Res<Config>,
    _favorites: &Res<Favorites>,
    show_titles: &Res<ShowTitles>,
    q_entities: &mut Query<(
        Entity,
        Option<&ClientAddress>,
        Option<&ClientClass>,
        Option<&mut Sprite>,
    )>,
    images: &mut ResMut<Assets<Image>>,
    address: String,
    class: String,
    title: String,
) {
    let client = Client {
        address: address.clone(),
        class: class.clone(),
        _name: Some(title.clone()),
    };
    let pinned_addr = format!("pinned:{}", client.class);
    if let Some((entity, _, _, Some(mut sprite))) = q_entities.iter_mut().find(|(_, addr_opt, class_opt, _)| {
        addr_opt.map_or(false, |a| a.0 == pinned_addr)
            && class_opt.map_or(false, |c| c.0 == client.class)
    }) {
        commands
            .entity(entity)
            .insert(ClientAddress(client.address.clone()));
        handle_open_pinned_window(entity, &client, &mut sprite, dock_order);
        client_list.0.push(client.clone());
        reorder_trigger.0 = true;
        return;
    }

    client_list.0.push(client.clone());
    dock_order.0.push(address.clone());
    reorder_trigger.0 = true;
    let window = windows.single();
    let start_x = -window.width() / 2.0 + config.margin_x;
    let start_y = -window.height() / 2.0 + config.margin_y;
    let start_pos = Vec2::new(start_x, start_y);
    let center = Vec2::new(0.0, window.height() * config.tilt_y);
    let direction = (center - start_pos).normalize_or_zero();

    let (translation, scale) =
        calculate_icon_transform(0, start_pos, direction, &config, Vec2::ZERO);
    let transform = Transform {
        translation,
        scale: Vec3::splat(scale),
        ..default()
    };

    let icon_entity = spawn_icon_entity(
        commands,
        images,
        asset_server,
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

    add_client_address(commands, icon_entity, client.address.clone());
    commands
        .entity(icon_entity)
        .insert(ClientAddress(client.address.clone()));
    commands
        .entity(icon_entity)
        .insert(ClientClass(client.class.clone()));

    if show_titles.0 {
        add_icon_text(
            commands,
            icon_entity,
            &client.class,
            transform,
            scale,
            asset_server,
            config,
        );
    }
}

fn handle_hypr_close_window(
    commands: &mut Commands,
    client_list: &mut ResMut<ClientList>,
    dock_order: &mut ResMut<DockOrder>,
    reorder_trigger: &mut ResMut<ReorderTrigger>,
    favorites: &Res<Favorites>,
    q_entities: &mut Query<(
        Entity,
        Option<&ClientAddress>,
        Option<&ClientClass>,
        Option<&mut Sprite>,
    )>,
    address: String,
) {
    if let Some((entity, addr_opt, class_opt, Some(mut sprite))) =
        q_entities.iter_mut().find(|(_, addr, _, sprite)| {
            addr.as_ref().map(|a| a.0 == address).unwrap_or(false) && sprite.is_some()
        })
    {
        if let (Some(_addr), Some(class)) = (addr_opt, class_opt) {
            if favorites.0.contains(&class.0) {
                handle_close_pinned_window(
                    entity,
                    &address,
                    &class.0,
                    &mut sprite,
                    dock_order,
                    commands,
                );
            } else {
                commands.entity(entity).despawn();
                dock_order.0.retain(|a| a != &address);
            }
        }
    }
    client_list.0.retain(|c| c.address != address);
    reorder_trigger.0 = true;
}

fn handle_open_pinned_window(
    _entity: Entity,
    client: &Client,
    sprite: &mut Sprite,
    dock_order: &mut DockOrder,
) {
    sprite.color.set_a(1.0);

    let pinned_addr = format!("pinned:{}", client.class.clone());
    if let Some(index) = dock_order.0.iter().position(|a| a == &pinned_addr) {
        dock_order.0[index] = client.address.clone();
    }
}

fn handle_close_pinned_window(
    entity: Entity,
    address: &str,
    class: &str,
    sprite: &mut Sprite,
    dock_order: &mut DockOrder,
    commands: &mut Commands,
) {
    let pinned_addr = format!("pinned:{}", class);
    commands
        .entity(entity)
        .insert(ClientAddress(pinned_addr.clone()));
    update_sprite_alpha(sprite, true, false);

    if let Some(index) = dock_order.0.iter().position(|a| a == address) {
        dock_order.0[index] = pinned_addr;
    }
}



