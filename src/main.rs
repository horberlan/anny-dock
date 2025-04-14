use bevy::prelude::*;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};
use bevy::render::texture::ImageSampler;
use bevy::window::{WindowPlugin, PrimaryWindow};
use image::io::Reader as ImageReader;
use resvg::{tiny_skia, usvg};
use serde::Deserialize;
use std::path::Path;
use std::process::Command;
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
        .add_startup_system(setup)
        .add_system(icon_click_system)
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

fn load_icon(path: &Path) -> Option<bevy::prelude::Image> {
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

fn load_svg_image(path: &Path) -> Option<bevy::prelude::Image> {
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
) {
    commands.spawn(Camera2dBundle::default());

    let mut x_offset = -200.0;

    for client in &client_list.0 {
        let icon_path = get_icon_path(&client.class);
        if let Some(path_str) = icon_path {
            let path = Path::new(&path_str);
            if let Some(img) = load_icon(path) {
                let handle = images.add(img);

                commands
                    .spawn(SpriteBundle {
                        texture: handle.clone(),
                        transform: Transform::from_xyz(x_offset, 0.0, 0.0),
                        ..default()
                    })
                    .insert(ClientIcon)
                    .insert(ClientAddress(client.address.clone()))
                    .insert(Name::new(client.name.clone().unwrap_or(client.class.clone())));
            }
        }

        // Nome do app
        commands.spawn(Text2dBundle {
            text: Text::from_section(
                client.name.clone().unwrap_or(client.class.clone()),
                TextStyle {
                    font: asset_server.load("fonts/FiraSans-Bold.ttf"),
                    font_size: 14.0,
                    color: Color::WHITE,
                },
            )
            .with_alignment(TextAlignment::Center),
            transform: Transform::from_xyz(x_offset, 32.0, 1.0),
            ..default()
        });

        x_offset += 64.0;
    }
}

fn icon_click_system(
    buttons: Res<Input<MouseButton>>,
    windows: Query<&Window, With<PrimaryWindow>>, // Corrigido para usar PrimaryWindow
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
                        println!("🔘 Clicked on {}", address.0);
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
