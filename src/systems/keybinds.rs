use bevy::app::AppExit;
use bevy::input::keyboard::KeyboardInput;
use bevy::input::ButtonState;
use bevy::prelude::*;
use bevy::window::PrimaryWindow;
use crate::{focus_client, types::*};
use crate::config::Config;
use crate::utils::launch_application;

pub fn scroll_with_arrows(
    keyboard: Res<Input<KeyCode>>,
    mut scroll_state: ResMut<ScrollState>,
    q_icons: Query<&HoverTarget>,
    config: Res<Config>,
    windows: Query<&Window, With<PrimaryWindow>>,
) {
    let total_items = q_icons.iter().count();
    if total_items <= config.visible_items {
        return;
    }

    let scroll_amount = config.scroll_speed;

    if keyboard.just_pressed(KeyCode::Left) {
        scroll_state.total_scroll_distance -= scroll_amount;
    }
    if keyboard.just_pressed(KeyCode::Right) {
        scroll_state.total_scroll_distance += scroll_amount;
    }

    let max_scroll = ((total_items as f32 - config.visible_items as f32).max(0.0)) * config.spacing;
    scroll_state.total_scroll_distance = scroll_state.total_scroll_distance.clamp(0.0, max_scroll);

    let window = windows.single();
    let window_width = window.width();
    let window_height = window.height();
    let start_x = -window_width / 2.0 + config.margin_x;
    let start_y = -window_height / 2.0 + config.margin_y;
    let start_pos = Vec2::new(start_x, start_y);
    let center = Vec2::new(0.0, window_height * config.tilt_y);
    let direction = (center - start_pos).normalize_or_zero();

    scroll_state.offset = direction * scroll_state.total_scroll_distance;
}

pub fn keybind_launch_visible_icons(
    keyboard: Res<Input<KeyCode>>,
    icons: Query<(&ClientClass, &HoverTarget, Option<&ClientAddress>)>,
    scroll_state: Res<ScrollState>,
    config: Res<Config>,
) {
    let keycodes = [
        KeyCode::Key1,
        KeyCode::Key2,
        KeyCode::Key3,
        KeyCode::Key4,
        KeyCode::Key5,
        KeyCode::Key6,
        KeyCode::Key7,
        KeyCode::Key8,
        KeyCode::Key9,
        KeyCode::Key0,
    ];

    let first_visible_index = (scroll_state.total_scroll_distance / config.spacing).floor() as usize;

    for (i, &key) in keycodes.iter().enumerate().take(config.visible_items) {
        if keyboard.just_pressed(key) {
            let target_index = first_visible_index + i;
            if let Some((class, _, address)) = icons.iter().find(|(_, hover, _)| hover.index == target_index) {
                if let Some(addr) = address {
                    if addr.0.starts_with("pinned:") {
                        launch_application(&class.0);
                    } else {
                        focus_client(&addr.0);
                    }
                } else {
                    launch_application(&class.0);
                }
            }
        }
    }
}

pub fn exit_on_esc_or_q(mut keys: EventReader<KeyboardInput>, mut exit: EventWriter<AppExit>) {
    for key_event in keys.read() {
        if let Some(key_code) = key_event.key_code {
            if key_event.state == ButtonState::Pressed
                && (key_code == KeyCode::Escape || key_code == KeyCode::Q)
            {
                exit.send(AppExit);
            }
        }
    }
}
