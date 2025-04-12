use image::io::Reader as ImageReader;
use serde::Deserialize;
use slint::{Image, SharedPixelBuffer, SharedString, VecModel};
use std::path::Path;
use std::process::Command;
use std::rc::Rc;

slint::include_modules!();

#[derive(Deserialize, Debug, Clone)]
struct Client {
    class: String,
    title: String,
    pid: i32,
    address: String,
}

fn load_clients() -> Vec<Client> {
    let output = Command::new("hyprctl")
        .args(["clients", "-j"])
        .output()
        .expect("failed to run hyprctl");

    serde_json::from_slice(&output.stdout).unwrap_or_default()
}

fn focus_client(address: &str) {
    let full_address = format!("address:{}", address.trim_start_matches("address:"));
    println!("🔘 focusing window: {}", full_address);

    let output = Command::new("hyprctl")
        .arg("dispatch")
        .arg("focuswindow")
        .arg(&full_address)
        .output()
        .expect("failed to execute hyprctl");

    println!(
        "ℹ️ command executed: hyprctl dispatch focuswindow {}",
        full_address
    );

    if !output.status.success() {
        eprintln!(
            "❌ error focusing window: {}\nFull output:\n{}",
            String::from_utf8_lossy(&output.stderr),
            String::from_utf8_lossy(&output.stdout)
        );
    } else {
        println!("✅ window successfully focused!");
    }
}

fn get_icon_for_class(class: &str) -> Option<Image> {
    let lowercase = class.to_lowercase();
    // todo: change to a real icon way...
    let icon_path = format!("/usr/share/icons/hicolor/48x48/apps/{}.png", lowercase);

    if Path::new(&icon_path).exists() {
        if let Ok(reader) = ImageReader::open(&icon_path) {
            if let Ok(img) = reader.decode() {
                let rgba_img = img.to_rgba8();
                let (width, height) = rgba_img.dimensions();

                let mut buffer = SharedPixelBuffer::<slint::Rgba8Pixel>::new(width, height);
                let buffer_slice = buffer.make_mut_slice();
                let raw_data = rgba_img.into_raw();

                buffer_slice.copy_from_slice(bytemuck::cast_slice(&raw_data));

                return Some(Image::from_rgba8(buffer));
            }
        }
    }
    None
}

fn main() -> Result<(), slint::PlatformError> {
    let ui = AppWindow::new()?;

    let clients_data = load_clients();
    let clients_model = Rc::new(VecModel::default());

    for client in clients_data {
        let icon = get_icon_for_class(&client.class);
        let address = client.address.clone();

        let client_data = ClientData {
            title: SharedString::from(client.title),
            class: SharedString::from(client.class),
            address: SharedString::from(address),
            has_icon: icon.is_some(),
            icon: icon.unwrap_or_default(),
        };

        clients_model.push(client_data);
    }

    ui.set_clients(clients_model.into());

    let ui_handle = ui.as_weak();
    ui.on_focus_window(move |address| {
        let address_str = address.to_string();
        println!("🖱️ click detected on window: {}", address_str);

        focus_client(&address_str);

        if let Some(ui) = ui_handle.upgrade() {
            let clients_data = load_clients();
            let clients_model = Rc::new(VecModel::default());

            for client in clients_data {
                let icon = get_icon_for_class(&client.class);
                let address = client.address.clone();

                let client_data = ClientData {
                    title: SharedString::from(client.title),
                    class: SharedString::from(client.class),
                    address: SharedString::from(address),
                    has_icon: icon.is_some(),
                    icon: icon.unwrap_or_default(),
                };

                clients_model.push(client_data);
            }

            ui.set_clients(clients_model.into());
        }
    });

    ui.run()
}
