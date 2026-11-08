//! Translation of Wayland input into the values bevy_winit would report.

use bevy::input::keyboard::KeyCode;
use bevy::input::mouse::MouseButton;
use smithay_client_toolkit::seat::{keyboard::Keysym, pointer::AxisScroll};

/// Keys the dock reacts to; anything else is forwarded with `key_code: None`.
pub fn key_code(keysym: Keysym) -> Option<KeyCode> {
    Some(match keysym {
        Keysym::Escape => KeyCode::Escape,
        Keysym::q | Keysym::Q => KeyCode::Q,
        Keysym::t | Keysym::T => KeyCode::T,
        Keysym::Left => KeyCode::Left,
        Keysym::Right => KeyCode::Right,
        Keysym::_0 => KeyCode::Key0,
        Keysym::_1 => KeyCode::Key1,
        Keysym::_2 => KeyCode::Key2,
        Keysym::_3 => KeyCode::Key3,
        Keysym::_4 => KeyCode::Key4,
        Keysym::_5 => KeyCode::Key5,
        Keysym::_6 => KeyCode::Key6,
        Keysym::_7 => KeyCode::Key7,
        Keysym::_8 => KeyCode::Key8,
        Keysym::_9 => KeyCode::Key9,
        _ => return None,
    })
}

/// Linux input event codes (`BTN_LEFT` = 0x110, ...).
pub fn mouse_button(code: u32) -> MouseButton {
    match code {
        0x110 => MouseButton::Left,
        0x111 => MouseButton::Right,
        0x112 => MouseButton::Middle,
        other => MouseButton::Other(other as u16),
    }
}

/// Approximate pixels per wheel line, for touchpads that send no discrete steps.
const PIXELS_PER_LINE: f64 = 10.0;

/// Scroll amount in lines with Bevy's sign convention (positive = up/left).
pub fn wheel_lines(axis: AxisScroll) -> f32 {
    if axis.discrete != 0 {
        -axis.discrete as f32
    } else {
        (-axis.absolute / PIXELS_PER_LINE) as f32
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maps_every_key_the_dock_uses() {
        let cases = [
            (Keysym::Escape, KeyCode::Escape),
            (Keysym::q, KeyCode::Q),
            (Keysym::Q, KeyCode::Q),
            (Keysym::t, KeyCode::T),
            (Keysym::T, KeyCode::T),
            (Keysym::Left, KeyCode::Left),
            (Keysym::Right, KeyCode::Right),
            (Keysym::_0, KeyCode::Key0),
            (Keysym::_1, KeyCode::Key1),
            (Keysym::_5, KeyCode::Key5),
            (Keysym::_9, KeyCode::Key9),
        ];
        for (keysym, expected) in cases {
            assert_eq!(key_code(keysym), Some(expected), "{keysym:?}");
        }
        assert_eq!(key_code(Keysym::F13), None);
    }

    #[test]
    fn maps_linux_button_codes() {
        assert_eq!(mouse_button(0x110), MouseButton::Left);
        assert_eq!(mouse_button(0x111), MouseButton::Right);
        assert_eq!(mouse_button(0x112), MouseButton::Middle);
        assert_eq!(mouse_button(0x113), MouseButton::Other(0x113));
    }

    #[test]
    fn wheel_uses_discrete_steps_and_bevy_direction() {
        let step = |discrete, absolute| AxisScroll { absolute, discrete, stop: false };
        // Wayland: positive = down; Bevy (as winit reports it): positive = up.
        assert_eq!(wheel_lines(step(1, 15.0)), -1.0);
        assert_eq!(wheel_lines(step(-2, -30.0)), 2.0);
        // Touchpads send no discrete steps, only pixels (~10 per line).
        assert_eq!(wheel_lines(step(0, 5.0)), -0.5);
    }
}
