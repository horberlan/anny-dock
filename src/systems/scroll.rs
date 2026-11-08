use crate::types::*;
use crate::config::Config;
use bevy::input::mouse::{MouseScrollUnit, MouseWheel};
use bevy::prelude::*;

/// Rate (1/s) at which `ScrollState::position` closes the gap to `target`:
/// ~90% of the way in 0.19 s.
pub const SCROLL_SMOOTHING: f32 = 12.0;
/// Pause after the last wheel/autoscroll input before snapping to a whole icon.
const SNAP_DELAY: f32 = 0.15;
/// Held-arrow repeat: first repeat after `KEY_REPEAT_DELAY`, then every interval.
pub const KEY_REPEAT_DELAY: f32 = 0.28;
pub const KEY_REPEAT_INTERVAL: f32 = 0.07;

/// Frame-rate independent exponential approach of `current` to `target`.
pub fn smooth_towards(current: f32, target: f32, rate: f32, dt: f32) -> f32 {
    target + (current - target) * (-rate * dt).exp()
}

/// Largest scroll position: the last icon fills the last visible slot.
pub fn max_scroll(total_items: usize, visible_items: usize) -> f32 {
    total_items.saturating_sub(visible_items) as f32
}

/// Keyboard auto-repeat for one key, independent of the compositor's.
#[derive(Default)]
pub struct KeyRepeat {
    held: f32,
    next: f32,
}

impl KeyRepeat {
    /// Number of steps to take this frame.
    pub fn steps(&mut self, just_pressed: bool, pressed: bool, dt: f32) -> u32 {
        if just_pressed {
            self.held = 0.0;
            self.next = KEY_REPEAT_DELAY;
            return 1;
        }
        if !pressed {
            return 0;
        }
        self.held += dt;
        let mut steps = 0;
        while self.held >= self.next {
            steps += 1;
            self.next += KEY_REPEAT_INTERVAL;
        }
        steps
    }
}

/// Autoscroll speed (icons/s) for a dragged icon at `axis` (distance along the
/// dock from the first slot). Starts past the first slot (negative) or past the
/// last visible slot at `end`, easing in quadratically over `zone`.
pub fn edge_scroll_velocity(axis: f32, end: f32, zone: f32, max_speed: f32) -> f32 {
    let depth = |past: f32| (past / zone).clamp(0.0, 1.0).powi(2);
    if axis < 0.0 {
        -max_speed * depth(-axis)
    } else if axis > end {
        max_speed * depth(axis - end)
    } else {
        0.0
    }
}

pub fn scroll_system(
    mut scroll_state: ResMut<ScrollState>,
    mut scroll_events: EventReader<MouseWheel>,
    dock_order: Res<DockOrder>,
    config: Res<Config>,
) {
    let max = max_scroll(dock_order.0.len(), config.visible_items);
    for event in scroll_events.read() {
        // Lines keep the configured speed (scroll_speed px per line, where one
        // icon is `spacing` px); touchpads report pixels.
        let icons = match event.unit {
            MouseScrollUnit::Line => event.y * config.scroll_speed / config.spacing,
            MouseScrollUnit::Pixel => event.y / config.spacing,
        };
        scroll_state.target = (scroll_state.target - icons).clamp(0.0, max);
        scroll_state.idle = 0.0;
    }
}

/// Eases the scroll position toward its target and, once wheel/autoscroll input
/// stops, snaps the target to a whole icon so the dock always rests aligned.
pub fn scroll_smoothing_system(
    mut scroll_state: ResMut<ScrollState>,
    dock_order: Res<DockOrder>,
    config: Res<Config>,
    ui_state: Res<UiState>,
    time: Res<Time>,
) {
    let dt = time.delta_seconds();
    let max = max_scroll(dock_order.0.len(), config.visible_items);

    scroll_state.idle += dt;
    if scroll_state.idle >= SNAP_DELAY && ui_state.dragging.is_none() {
        let snapped = scroll_state.target.round().clamp(0.0, max);
        if snapped != scroll_state.target {
            scroll_state.target = snapped;
        }
    }
    // Items may have closed while scrolled to the end.
    if scroll_state.target > max {
        scroll_state.target = max;
    }

    let position = smooth_towards(scroll_state.position, scroll_state.target, SCROLL_SMOOTHING, dt);
    let position = if (position - scroll_state.target).abs() < 1e-3 {
        scroll_state.target
    } else {
        position
    };
    if position != scroll_state.position {
        scroll_state.position = position;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::systems::scroll_with_arrows;
    use bevy::input::mouse::MouseScrollUnit;

    #[test]
    fn smoothing_does_not_depend_on_frame_rate() {
        let one_step = smooth_towards(0.0, 10.0, SCROLL_SMOOTHING, 1.0 / 30.0);
        let half = smooth_towards(0.0, 10.0, SCROLL_SMOOTHING, 1.0 / 60.0);
        let two_steps = smooth_towards(half, 10.0, SCROLL_SMOOTHING, 1.0 / 60.0);

        assert!((one_step - two_steps).abs() < 1e-4);
        assert!(one_step > 0.0 && one_step < 10.0);
    }

    #[test]
    fn max_scroll_keeps_the_last_page_full() {
        assert_eq!(max_scroll(12, 8), 4.0);
        assert_eq!(max_scroll(5, 8), 0.0);
    }

    #[test]
    fn held_key_repeats_after_a_delay() {
        let mut repeat = KeyRepeat::default();

        assert_eq!(repeat.steps(true, true, 0.0), 1);
        assert_eq!(repeat.steps(false, true, 0.2), 0, "still within the initial delay");
        assert_eq!(repeat.steps(false, true, 0.1), 1, "delay elapsed");
        assert_eq!(repeat.steps(false, true, 2.0 * KEY_REPEAT_INTERVAL), 2);
        assert_eq!(repeat.steps(false, false, 1.0), 0, "released");
    }

    #[test]
    fn autoscroll_only_past_the_first_and_last_visible_icons() {
        let (end, zone, max) = (500.0, 50.0, 8.0);

        assert_eq!(edge_scroll_velocity(0.0, end, zone, max), 0.0);
        assert_eq!(edge_scroll_velocity(250.0, end, zone, max), 0.0);
        assert_eq!(edge_scroll_velocity(end, end, zone, max), 0.0);
        assert_eq!(edge_scroll_velocity(end + zone, end, zone, max), max);
        assert_eq!(edge_scroll_velocity(end + 3.0 * zone, end, zone, max), max, "capped");
        assert_eq!(edge_scroll_velocity(-zone, end, zone, max), -max);
        let ramp = edge_scroll_velocity(end + zone / 2.0, end, zone, max);
        assert!(ramp > 0.0 && ramp < max / 2.0, "eases in: {ramp}");
    }

    fn scroll_app(items: usize) -> App {
        let mut app = App::new();
        app.insert_resource(Config::default())
            .insert_resource(ScrollState::default())
            .insert_resource(Input::<KeyCode>::default())
            .insert_resource(UiState::default())
            .insert_resource(DockOrder((0..items).map(|i| format!("addr-{i}")).collect()))
            .init_resource::<Time>()
            .add_event::<MouseWheel>()
            .add_systems(Update, (scroll_system, scroll_with_arrows, scroll_smoothing_system).chain());
        app
    }

    fn advance(app: &mut App, seconds: f32) {
        app.world
            .resource_mut::<Time>()
            .advance_by(std::time::Duration::from_secs_f32(seconds));
        app.update();
    }

    #[test]
    fn arrow_scrolls_exactly_one_icon_and_eases_there() {
        let mut app = scroll_app(12);
        app.world.resource_mut::<Input<KeyCode>>().press(KeyCode::Right);
        advance(&mut app, 1.0 / 60.0);

        let scroll = app.world.resource::<ScrollState>();
        assert_eq!(scroll.target, 1.0);
        assert!(scroll.position > 0.0 && scroll.position < 1.0, "animates: {}", scroll.position);
    }

    #[test]
    fn arrows_stop_at_the_ends() {
        let mut app = scroll_app(12);
        app.world.resource_mut::<Input<KeyCode>>().press(KeyCode::Left);
        advance(&mut app, 1.0 / 60.0);

        assert_eq!(app.world.resource::<ScrollState>().target, 0.0);
    }

    #[test]
    fn wheel_snaps_to_a_whole_icon_once_it_stops() {
        let mut app = scroll_app(12);
        app.world.send_event(MouseWheel {
            unit: MouseScrollUnit::Line,
            x: 0.0,
            y: -1.0,
            window: Entity::PLACEHOLDER,
        });
        advance(&mut app, 1.0 / 60.0);
        let target = app.world.resource::<ScrollState>().target;
        assert!(target > 0.0 && target.fract() != 0.0, "follows the wheel: {target}");

        for _ in 0..60 {
            advance(&mut app, 1.0 / 60.0);
        }
        let scroll = app.world.resource::<ScrollState>();
        assert_eq!(scroll.target, scroll.target.round());
        assert!((scroll.position - scroll.target).abs() < 0.01);
    }

    #[test]
    fn short_list_never_scrolls() {
        let mut app = scroll_app(5);
        app.world.resource_mut::<Input<KeyCode>>().press(KeyCode::Right);
        advance(&mut app, 1.0 / 60.0);

        assert_eq!(app.world.resource::<ScrollState>().target, 0.0);
    }
}
