use bevy::prelude::*;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};
use bevy::render::texture::ImageSampler;
use bevy::window::{WindowPlugin, PrimaryWindow};
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

static FONT_PATH: &str = "/usr/share/fonts/VictorMono/VictorMonoNerdFont-Medium.ttf";

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                transparent: true,
                decorations: false,
                ..default()
            }),
            ..default()
        }).set(ImagePlugin {
            default_sampler: ImageSampler::nearest_descriptor(),
        }))
        .insert_resource(ClearColor(Color::NONE))
        .insert_resource(ClientList(load_clients()))
        .insert_resource(IconPositions::default())
        .insert_resource(ShowTitles(true))
        .add_startup_system(setup)
        .add_system(hover_system)
        .add_system(hover_animation_system)
        .add_system(collect_icon_data.before(update_text_positions))
        .add_system(update_text_positions)
        .add_system(icon_click_system)
        .add_system(toggle_titles)
        .run();
}

fn load_clients() -> Vec<Client> {
    let output = Command::new("hyprctl")
        .args(["clients", "-j"])
        .output()
        .expect("failed to run hyprctl");

    serde_json::from_slice(&output.stdout).unwrap_or_default()
}

fn get_icon_path(class: &str) -> Option<String> {
    let lowercase = class.to_lowercase();
    let icon_path = icon_finder::find_icon(lowercase, 48, 1)?;
    Some(icon_path.to_string_lossy().to_string())
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

    let pixmap_size = 48;
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
    });

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
        if let Some(path_str) = icon_path {
            let path = Path::new(&path_str);
            if let Some(img) = load_icon(path) {
                let handle = images.add(img);

                let z_index = clients_count - index - 1;
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
        } else {
            println!("Warning: ⚠️ No icons found for {}", client.class);
        }
    }
}

fn hover_system(
    windows: Query<&Window, With<PrimaryWindow>>,
    q_camera: Query<(&Camera, &GlobalTransform)>,
    mut q_icons: Query<(&GlobalTransform, &mut HoverTarget)>,
) {
    let window = windows.single();
    let (camera, camera_transform) = q_camera.single();

    if let Some(cursor_pos) = window.cursor_position() {
        if let Some(world_cursor) = camera.viewport_to_world_2d(camera_transform, cursor_pos) {
            for (global_transform, mut hover) in &mut q_icons {
                let pos = global_transform.translation().truncate();
                let size = Vec2::splat(48.0);
                let rect = Rect::from_center_size(pos, size);

                hover.is_hovered = rect.contains(world_cursor);
            }
        }
    }
}

fn hover_animation_system(
    time: Res<Time>,
    mut q: Query<(&mut Transform, &HoverTarget)>,
) {
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
    for (entity, transform, _) in query.iter() {
        icon_positions.0.insert(
            entity,
            (transform.translation, transform.scale)
        );
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
    q_camera: Query<(&Camera, &GlobalTransform)>,
    q_icons: Query<(&Transform, &ClientAddress), With<ClientIcon>>,
) {
    if buttons.just_pressed(MouseButton::Left) {
        let window = windows.single();
        if let Some(cursor_pos) = window.cursor_position() {
            let (camera, camera_transform) = q_camera.single();

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

fn focus_client(address: &str) {
    let full_address = format!("address:{}", address.trim_start_matches("address:"));

    let output = Command::new("hyprctl")
        .args(["dispatch", "focuswindow", &full_address])
        .output()
        .expect("failed to execute hyprctl");

    if output.status.success() {
        println!("✅ Focused window: {}", full_address);
    } else {
        eprintln!("❌ Failed to focus window: {}. Error: {}", full_address, String::from_utf8_lossy(&output.stderr));
    }
}
fn toggle_titles(
    mut show_titles: ResMut<ShowTitles>,
    keyboard_input: Res<Input<KeyCode>>,
) {
    if keyboard_input.just_pressed(KeyCode::T) {
        show_titles.0 = !show_titles.0;
        println!("has titles: {}", show_titles.0);
    }
}