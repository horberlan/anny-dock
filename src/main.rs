use bevy::prelude::*;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};
use bevy::render::texture::{Image, ImageSampler};
use bevy::window::{Window, WindowPlugin, PrimaryWindow};
use image::io::Reader as ImageReader;
use resvg::{tiny_skia, usvg};
use serde::Deserialize;
use std::path::Path;
use std::process::Command;
use std::collections::HashMap;
use xdgkit::icon_finder;

#[derive(Deserialize, Debug, Clone)]
struct Client {
    class: String,
    title: String,
    address: String,
    #[serde(default)]
    name: Option<String>,
}

#[derive(Component)]
struct ClientIcon;

#[derive(Component)]
struct ClientAddress(String);

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
    needs_restart: bool,
    click_origin: Option<Vec2>,
}

#[derive(Component)]
struct HoverTarget {
    original_x: f32,
    original_y: f32,
    original_z: f32,
    original_scale: f32,
    index: usize,
    is_hovered: bool,
}

#[derive(Component)]
struct IconText(Entity);

#[derive(Resource, Default)]
struct IconPositions(HashMap<Entity, (Vec3, Vec3)>);

#[derive(Component)]
struct MainCamera;

static FONT_PATH: &str = "/usr/share/fonts/VictorMono/VictorMonoNerdFont-Medium.ttf";
static FALLBACK_ICON_PATH: &str = "assets/dockIcon.svg";

fn main() {
    App::new()
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
                    default_sampler: ImageSampler::nearest_descriptor(),
                    ..default()
                }),
        )
        .insert_resource(ClearColor(Color::NONE))
        .insert_resource(ClientList(load_clients()))
        .insert_resource(IconPositions::default())
        .insert_resource(ShowTitles(true))
        .insert_resource(UiState::default())
        .add_startup_system(setup)
        .add_system(cleanup_duplicate_cameras)
        .add_system(hover_system)
        .add_system(hover_animation_system)
        .add_system(collect_icon_data.before(update_text_positions))
        .add_system(update_text_positions)
        .add_system(icon_click_system)
        .add_system(toggle_titles)
        // drag + threshold
        .add_system(drag_register_click_system)
        .add_system(drag_check_system)
        .add_system(drag_update_system)
        .add_system(drag_end_system)
        .add_system(reset_positions_system)
        .add_system(check_restart)
        .run();
}

fn cleanup_duplicate_cameras(
    mut commands: Commands,
    query: Query<(Entity, &MainCamera)>,
) {
    let mut found_camera = false;
    for (entity, _) in query.iter() {
        if found_camera {
            commands.entity(entity).despawn_recursive();
        } else {
            found_camera = true;
        }
    }
}

fn load_clients() -> Vec<Client> {
    let output = Command::new("hyprctl")
        .args(["clients", "-j"])
        .output()
        .expect("failed to run hyprctl");

    serde_json::from_slice(&output.stdout).unwrap_or_default()
}

fn get_icon_path(class: &str) -> String {
    let lowercase = class.to_lowercase();
    match icon_finder::find_icon(lowercase, 56, 1) {
        Some(path) => path.to_string_lossy().to_string(),
        None => {
            println!("Warning: ⚠️ No icons found for {}, using fallback", class);
            FALLBACK_ICON_PATH.to_string()
        }
    }
}

fn load_icon(path: &Path) -> Option<Image> {
    if path.extension().map_or(false, |ext| ext == "svg") {
        return load_svg_image(path);
    } else {
        if let Ok(reader) = ImageReader::open(path) {
            if let Ok(img) = reader.decode() {
                let rgba_img = img.to_rgba8();
                let (width, height) = rgba_img.dimensions();
                let data = rgba_img.into_raw();

                let image = Image::new_fill(
                    Extent3d {
                        width,
                        height,
                        depth_or_array_layers: 1,
                    },
                    TextureDimension::D2,
                    &data,
                    TextureFormat::Rgba8UnormSrgb,
                );
                return Some(image);
            }
        }
    }
    None
}

fn load_svg_image(path: &Path) -> Option<Image> {
    let svg_data = std::fs::read(path).ok()?;
    let opt = usvg::Options::default();
    let tree = usvg::Tree::from_data(&svg_data, &opt).ok()?;

    let pixmap_size = 56;
    let mut pixmap = tiny_skia::Pixmap::new(pixmap_size, pixmap_size)?;
    resvg::render(
        &tree,
        usvg::FitTo::Size(pixmap_size, pixmap_size),
        tiny_skia::Transform::default(),
        pixmap.as_mut(),
    )?;

    let data = pixmap.data().to_vec();
    let image = Image::new_fill(
        Extent3d {
            width: pixmap_size,
            height: pixmap_size,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        &data,
        TextureFormat::Rgba8UnormSrgb,
    );
    Some(image)
}

fn setup(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut images: ResMut<Assets<Image>>,
    client_list: Res<ClientList>,
    windows: Query<&Window, With<PrimaryWindow>>,
    show_titles: Res<ShowTitles>,
) {
    commands.spawn(Camera2dBundle {
        transform: Transform {
            translation: Vec3::new(0.0, 0.0, 100.0),
            ..default()
        },
        ..default()
    }).insert(MainCamera);

    let window = windows.single();
    let window_width = window.width();
    let window_height = window.height();

    let margin_x = 50.0;
    let margin_y = 50.0;
    let start_x = -window_width / 2.0 + margin_x;
    let start_y = -window_height / 2.0 + margin_y;

    let start_pos = Vec2::new(start_x, start_y);
    let center = Vec2::new(0.0, 0.0);
    let direction = (center - start_pos).normalize_or_zero();

    let spacing = 40.0;
    let z_spacing = 2.0;
    let base_scale = 1.2;
    let scale_factor: f32 = 0.9;

    let clients_count = client_list.0.len();

    for (index, client) in client_list.0.iter().enumerate() {
        let icon_path = get_icon_path(&client.class);
        let path = Path::new(&icon_path);
        
        if let Some(img) = load_icon(path) {
            let handle = images.add(img);

            let _z_index = clients_count - index - 1;
            let offset = direction * (index as f32 * spacing);
            let pos = start_pos + offset;
            let x = pos.x;
            let y = pos.y;
            let z = -(index as f32 * z_spacing);

            let scale = base_scale * scale_factor.powi(index as i32);

            let icon_entity = commands
                .spawn(SpriteBundle {
                    texture: handle.clone(),
                    transform: Transform {
                        translation: Vec3::new(x, y, z),
                        scale: Vec3::splat(scale),
                        ..default()
                    },
                    ..default()
                })
                .insert(ClientIcon)
                .insert(ClientAddress(client.address.clone()))
                .insert(HoverTarget {
                    original_x: x,
                    original_y: y,
                    original_z: z,
                    original_scale: scale,
                    index,
                    is_hovered: false,
                })
                .insert(Name::new(client.name.clone().unwrap_or(client.class.clone())))
                .id();

            if show_titles.0 {
                commands.spawn(Text2dBundle {
                    text: Text::from_section(
                        client.name.clone().unwrap_or(client.class.clone()),
                        TextStyle {
                            font: asset_server.load(FONT_PATH),
                            font_size: 12.0 * scale,
                            color: Color::WHITE,
                        },
                    )
                    .with_alignment(TextAlignment::Center),
                    transform: Transform {
                        translation: Vec3::new(x, y - 30.0 * scale, z - 0.01),
                        scale: Vec3::splat(scale),
                        ..default()
                    },
                    ..default()
                }).insert(IconText(icon_entity));
            }
        } else {
            println!("Error: ❌ Failed to load icon for {}", client.class);
        }
    }
}

fn hover_system(
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
                for (mut hover, _transform) in &mut q_icons {
                    let pos = Vec2::new(hover.original_x, hover.original_y);
                    let size = Vec2::splat(48.0 * hover.original_scale);
                    let rect = Rect::from_center_size(pos, size);
                    hover.is_hovered = rect.contains(world_cursor);
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
            hover.original_y + 20.0
        } else {
            hover.original_y
        };

        let current_y = transform.translation.y;
        let new_y = current_y + (target_y - current_y) * time.delta_seconds() * 8.0;

        transform.translation.x = hover.original_x;
        transform.translation.y = new_y;
        transform.translation.z = hover.original_z;

        let original_scale = hover.original_scale;
        let target_scale = if hover.is_hovered { original_scale * 1.2 } else { original_scale };
        let current_scale = transform.scale.x;
        let new_scale = current_scale + (target_scale - current_scale) * time.delta_seconds() * 5.0;
        transform.scale = Vec3::splat(new_scale);
    }
}

fn collect_icon_data(
    query: Query<(Entity, &Transform, &HoverTarget)>,
    mut icon_positions: ResMut<IconPositions>,
) {
    icon_positions.0.clear();
    for (entity, transform, _hover) in query.iter() {
        icon_positions.0.insert(entity, (transform.translation, transform.scale));
    }
}

fn update_text_positions(
    mut text_query: Query<(&mut Transform, &IconText)>,
    icon_positions: Res<IconPositions>,
) {
    for (mut text_transform, icon_text) in text_query.iter_mut() {
        if let Some((position, scale)) = icon_positions.0.get(&icon_text.0) {
            text_transform.translation.x = position.x;
            text_transform.translation.y = position.y - 30.0;
            text_transform.translation.z = position.z - 0.01;
            text_transform.scale = *scale;
        }
    }
}

fn icon_click_system(
    buttons: Res<Input<MouseButton>>,
    windows: Query<&Window, With<PrimaryWindow>>,
    q_camera: Query<(&Camera, &GlobalTransform), With<MainCamera>>,
    q_icons: Query<(&Transform, &ClientAddress), With<ClientIcon>>,
    ui_state: Res<UiState>,
) {
    // if not dragging, work w/ click
    if buttons.just_released(MouseButton::Left) && ui_state.dragging.is_none() {
        let window = windows.single();
        if let Some(cursor_pos) = window.cursor_position() {
            if let Ok((camera, camera_transform)) = q_camera.get_single() {
                if let Some(world_pos) = camera.viewport_to_world_2d(camera_transform, cursor_pos) {
                    for (transform, address) in &q_icons {
                        let pos = transform.translation.truncate();
                        let size = Vec2::splat(48.0);
                        let rect = Rect::from_center_size(pos, size);
                        if rect.contains(world_pos) {
                            focus_client(&address.0);
                        }
                    }
                }
            }
        }
    }
}

fn focus_client(address: &str) {
    let full_address = format!("address:{}", address.trim_start_matches("address:"));

    let output = Command::new("hyprctl")
        .args(["dispatch", "focuswindow", &full_address])
        .output()
        .expect("failed to execute hyprctl");

    if output.status.success() {
        println!("✅ Focused window: {}", full_address);
    } else {
        eprintln!(
            "❌ Failed to focus window: {}. Error: {}",
            full_address,
            String::from_utf8_lossy(&output.stderr)
        );
    }
}

fn toggle_titles(
    mut show_titles: ResMut<ShowTitles>,
    keyboard_input: Res<Input<KeyCode>>,
    mut ui_state: ResMut<UiState>,
) {
    if keyboard_input.just_pressed(KeyCode::T) {
        show_titles.0 = !show_titles.0;
        ui_state.needs_restart = true;
        println!("Title visibility toggled: {}", show_titles.0);
    }
}

fn drag_register_click_system(
    windows: Query<&Window, With<PrimaryWindow>>,
    mouse_button: Res<Input<MouseButton>>,
    mut ui_state: ResMut<UiState>,
) {
    if mouse_button.just_pressed(MouseButton::Left) {
        if let Some(window) = windows.iter().next() {
            if let Some(cursor_pos) = window.cursor_position() {
                ui_state.click_origin = Some(cursor_pos);
            }
        }
    }
}

fn drag_check_system(
    mut commands: Commands,
    windows: Query<&Window, With<PrimaryWindow>>,
    q_camera: Query<(&Camera, &GlobalTransform), With<MainCamera>>,
    q_icons: Query<(Entity, &HoverTarget)>,
    mouse_button: Res<Input<MouseButton>>,
    mut ui_state: ResMut<UiState>,
) {
    // if the button is pressed and we have registered origin
    if mouse_button.pressed(MouseButton::Left) {
        let window = windows.single();
        if let (Some(click_origin), Some(cursor_pos)) = (ui_state.click_origin, window.cursor_position()) {
           
            let threshold = 10.0;
            if click_origin.distance(cursor_pos) > threshold && ui_state.dragging.is_none() {

                if let Ok((camera, camera_transform)) = q_camera.get_single() {
                    if let Some(world_cursor) = camera.viewport_to_world_2d(camera_transform, cursor_pos) {
                        // check each icon to see if the cursor is inside it

                        for (entity, hover) in q_icons.iter() {
                            let pos = Vec2::new(hover.original_x, hover.original_y);
                            let size = Vec2::splat(48.0 * hover.original_scale);
                            let rect = Rect::from_center_size(pos, size);
                            
                            // calculate the offset between the icon and the click position
                            if rect.contains(world_cursor) {
                                let offset = pos - world_cursor;
                                commands.entity(entity).insert(Dragging { offset });
                                ui_state.dragging = Some(entity);
                                break;
                            }
                        }
                    }
                }
            }
        }
    } else if mouse_button.just_released(MouseButton::Left) {
        // ---if the button is released without having exceeded the threshold, reset the origin
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
            let window = windows.single();
            if let Some(cursor_pos) = window.cursor_position() {
                if let Ok((camera, camera_transform)) = q_camera.get_single() {
                    if let Some(world_cursor) = camera.viewport_to_world_2d(camera_transform, cursor_pos) {
                        transform.translation.x = world_cursor.x + dragging.offset.x;
                        transform.translation.y = world_cursor.y + dragging.offset.y;
                        transform.translation.z = hover.original_z;
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
    mut client_list: ResMut<ClientList>,
    q_icons: Query<(&Transform, &HoverTarget, &ClientAddress)>,
) {
    if mouse_button.just_released(MouseButton::Left) && ui_state.dragging.is_some() {
        if let Some(dragged_entity) = ui_state.dragging {
            commands.entity(dragged_entity).remove::<Dragging>();
            
            // order clients by horizontal flow
            let mut positions: Vec<(usize, f32, String)> = q_icons
                .iter()
                .map(|(transform, hover, address)| (hover.index, transform.translation.x, address.0.clone()))
                .collect();
                
            positions.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap());
            
            let mut new_list = Vec::new();
            for (_, _, address) in positions.iter() {
                for client in &client_list.0 {
                    if client.address == *address {
                        new_list.push(client.clone());
                        break;
                    }
                }
            }
            
            client_list.0 = new_list;
            
            ui_state.dragging = None;
            ui_state.click_origin = None;
            ui_state.needs_restart = true;
        }
    }
}

fn reset_positions_system(
    mut commands: Commands,
    mut q_dragging: Query<(Entity, &mut Transform, &HoverTarget), With<Dragging>>,
    keyboard_input: Res<Input<KeyCode>>,
    mut ui_state: ResMut<UiState>,
) {
    if keyboard_input.just_pressed(KeyCode::Escape) {
        for (entity, mut transform, hover) in &mut q_dragging {
            transform.translation.x = hover.original_x;
            transform.translation.y = hover.original_y;
            commands.entity(entity).remove::<Dragging>();
            ui_state.dragging = None;
        }
    }
}

fn check_restart(
    mut ui_state: ResMut<UiState>,
    mut commands: Commands,
    query: Query<Entity, Or<(With<ClientIcon>, With<IconText>)>>,
    cameras: Query<Entity, With<MainCamera>>,
    client_list: Res<ClientList>,
    asset_server: Res<AssetServer>,
    mut images: ResMut<Assets<Image>>,
    windows: Query<&Window, With<PrimaryWindow>>,
    show_titles: Res<ShowTitles>,
) {
    if ui_state.needs_restart {
        for entity in cameras.iter() {
            commands.entity(entity).despawn_recursive();
        }
        
        for entity in &query {
            commands.entity(entity).despawn();
        }
        
        ui_state.needs_restart = false;
        
        setup(commands, asset_server, images, client_list, windows, show_titles);
    }
}