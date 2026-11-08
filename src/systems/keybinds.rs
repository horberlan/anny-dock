use bevy::app::AppExit;
use bevy::input::keyboard::KeyboardInput;
use bevy::input::ButtonState;
use bevy::prelude::*;
use crate::{focus_client, types::*};
use crate::config::Config;
use crate::utils::launch_application;
use super::{max_scroll, KeyRepeat};

/// Left/Right move one icon per press and repeat while held; repeated presses
/// accumulate on the target, and `scroll_smoothing_system` animates the move.
pub fn scroll_with_arrows(
    keyboard: Res<Input<KeyCode>>,
    mut scroll_state: ResMut<ScrollState>,
    dock_order: Res<DockOrder>,
    config: Res<Config>,
    time: Res<Time>,
    mut repeat: Local<(KeyRepeat, KeyRepeat)>,
) {
    let dt = time.delta_seconds();
    let back = repeat.0.steps(keyboard.just_pressed(KeyCode::Left), keyboard.pressed(KeyCode::Left), dt);
    let forward =
        repeat.1.steps(keyboard.just_pressed(KeyCode::Right), keyboard.pressed(KeyCode::Right), dt);
    if back == forward {
        return;
    }

    let max = max_scroll(dock_order.0.len(), config.visible_items);
    let target = scroll_state.target.round() + forward as f32 - back as f32;
    scroll_state.target = target.clamp(0.0, max);
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

    let first_visible_index = scroll_state.target.round().max(0.0) as usize;

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

pub fn exit_on_esc_or_q(
    mut keys: EventReader<KeyboardInput>,
    mut exit: EventWriter<AppExit>,
    ui_state: Res<UiState>,
) {
    for key_event in keys.read() {
        if let Some(key_code) = key_event.key_code {
            // While dragging, Esc cancels the drag (reset_positions_system).
            let cancels_drag = key_code == KeyCode::Escape && ui_state.dragging.is_some();
            if key_event.state == ButtonState::Pressed
                && (key_code == KeyCode::Escape || key_code == KeyCode::Q)
                && !cancels_drag
            {
                exit.send(AppExit);
            }
        }
    }
}
