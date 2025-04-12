use gtk4::prelude::*;
use gtk4::{Application, ApplicationWindow, Box as GtkBox, Button, CssProvider, Orientation, StyleContext, Image, gdk};
use std::process::Command;
use serde::Deserialize;

#[derive(Deserialize)]
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
        .expect("Failed to run hyprctl");

    serde_json::from_slice(&output.stdout).unwrap_or_default()
}

fn focus_client(address: &str) {
    let _ = Command::new("hyprctl")
        .arg("dispatch")
        .arg("focuswindow")
        .arg(address)
        .output();
}

fn get_icon_for_class(class: &str) -> Option<Image> {
    let lowercase = class.to_lowercase();
    let icon_path = format!("/usr/share/icons/hicolor/48x48/apps/{}.png", lowercase);

    if std::path::Path::new(&icon_path).exists() {
        Some(Image::from_file(icon_path))
    } else {
        None
    }
}

fn main() {
    let app = Application::builder()
        .application_id("dev.pdock")
        .build();

    app.connect_activate(|app| {
        let clients = load_clients();
        let provider = CssProvider::new();
        provider.load_from_data(include_str!("./style.css"));

        // Ainda é válido no Rust:
        StyleContext::add_provider_for_display(
            &gdk::Display::default().unwrap(),
            &provider,
            gtk4::STYLE_PROVIDER_PRIORITY_APPLICATION,
        );

        let button_box = GtkBox::new(Orientation::Horizontal, 6);
        button_box.set_margin_top(6);
        button_box.set_margin_bottom(6);
        button_box.set_margin_start(6);
        button_box.set_margin_end(6);

        for client in clients {
            let btn = Button::new();
            btn.set_tooltip_text(Some(&client.title));

            if let Some(icon) = get_icon_for_class(&client.class) {
                btn.set_child(Some(&icon));
            } else {
                btn.set_label(&client.class);
            }

            let address = client.address.clone();
            btn.connect_clicked(move |_| {
                focus_client(&address);
            });

            button_box.append(&btn);
        }

        let window = ApplicationWindow::builder()
            .application(app)
            .title("pdock")
            .child(&button_box)
            .build();

        // 🧼 Janela limpa
        window.set_decorated(false);
        window.fullscreen();

        // 💡 Transparência no Wayland: use o CSS mesmo
        let widget: gtk4::Widget = window.clone().upcast();
        widget.set_visible(true);

        window.present();
    });

    app.run();
}
