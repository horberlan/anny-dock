//! Wayland client state: the layer surface plus pointer/keyboard handlers.
//! Input is queued as `InputEvent`s and drained by the runner into Bevy.

use bevy::input::ButtonState;
use bevy::log::{debug, warn};
use bevy::input::keyboard::KeyCode;
use bevy::input::mouse::MouseButton;
use bevy::math::Vec2;
use smithay_client_toolkit::{
    compositor::{CompositorHandler, CompositorState},
    delegate_compositor, delegate_keyboard, delegate_layer, delegate_output, delegate_pointer,
    delegate_registry, delegate_seat, delegate_shm,
    output::{OutputHandler, OutputState},
    registry::{ProvidesRegistryState, RegistryState},
    registry_handlers,
    seat::{
        keyboard::{KeyEvent, KeyboardHandler, Keysym, Modifiers},
        pointer::{CursorIcon, PointerEvent, PointerEventKind, PointerHandler, ThemeSpec, ThemedPointer},
        Capability, SeatHandler, SeatState,
    },
    shell::{
        wlr_layer::{LayerShellHandler, LayerSurface, LayerSurfaceConfigure},
        WaylandSurface,
    },
    shm::{Shm, ShmHandler},
};
use wayland_client::{
    protocol::{wl_keyboard, wl_output, wl_pointer, wl_seat, wl_surface},
    Connection, QueueHandle,
};

use super::input::{key_code, mouse_button, wheel_lines};

pub enum InputEvent {
    CursorMoved(Vec2),
    CursorLeft,
    Button(MouseButton, ButtonState),
    Wheel { x: f32, y: f32 },
    Key { scan_code: u32, key_code: Option<KeyCode>, state: ButtonState },
}

/// Cursor shown over the dock's icons.
const DOCK_CURSOR: CursorIcon = CursorIcon::Default;

pub struct WaylandState {
    pub registry: RegistryState,
    pub compositor: CompositorState,
    pub shm: Shm,
    pub seat: SeatState,
    pub output: OutputState,
    /// Created right after the first roundtrip, once outputs are known.
    pub layer: Option<LayerSurface>,
    pub keyboard: Option<wl_keyboard::WlKeyboard>,
    /// Sets the cursor through `wp_cursor_shape_v1`, or draws it from the
    /// cursor theme when the compositor lacks that protocol.
    pub pointer: Option<ThemedPointer>,
    /// Logical size from the last configure, until the runner applies it.
    pub configured_size: Option<(u32, u32)>,
    /// Integer buffer scale of the outputs the surface is on, until applied.
    pub scale_factor: Option<i32>,
    pub closed: bool,
    pub pending: Vec<InputEvent>,
}

impl WaylandState {
    pub fn surface(&self) -> &wl_surface::WlSurface {
        self.layer
            .as_ref()
            .expect("layer surface is created in LayerShellBackend::connect")
            .wl_surface()
    }
}

impl CompositorHandler for WaylandState {
    fn scale_factor_changed(
        &mut self,
        _: &Connection,
        _: &QueueHandle<Self>,
        _: &wl_surface::WlSurface,
        new_factor: i32,
    ) {
        self.scale_factor = Some(new_factor);
    }

    fn transform_changed(
        &mut self,
        _: &Connection,
        _: &QueueHandle<Self>,
        _: &wl_surface::WlSurface,
        _: wl_output::Transform,
    ) {
    }

    fn frame(&mut self, _: &Connection, _: &QueueHandle<Self>, _: &wl_surface::WlSurface, _: u32) {}
}

impl OutputHandler for WaylandState {
    fn output_state(&mut self) -> &mut OutputState {
        &mut self.output
    }

    fn new_output(&mut self, _: &Connection, _: &QueueHandle<Self>, _: wl_output::WlOutput) {}

    fn update_output(&mut self, _: &Connection, _: &QueueHandle<Self>, _: wl_output::WlOutput) {}

    fn output_destroyed(&mut self, _: &Connection, _: &QueueHandle<Self>, _: wl_output::WlOutput) {}
}

impl LayerShellHandler for WaylandState {
    fn closed(&mut self, _: &Connection, _: &QueueHandle<Self>, _: &LayerSurface) {
        self.closed = true;
    }

    fn configure(
        &mut self,
        _: &Connection,
        _: &QueueHandle<Self>,
        _: &LayerSurface,
        configure: LayerSurfaceConfigure,
        _: u32,
    ) {
        self.configured_size = Some(configure.new_size);
    }
}

impl SeatHandler for WaylandState {
    fn seat_state(&mut self) -> &mut SeatState {
        &mut self.seat
    }

    fn new_seat(&mut self, _: &Connection, _: &QueueHandle<Self>, _: wl_seat::WlSeat) {}

    fn new_capability(
        &mut self,
        _: &Connection,
        qh: &QueueHandle<Self>,
        seat: wl_seat::WlSeat,
        capability: Capability,
    ) {
        if capability == Capability::Keyboard && self.keyboard.is_none() {
            self.keyboard = self.seat.get_keyboard(qh, &seat, None).ok();
        }
        if capability == Capability::Pointer && self.pointer.is_none() {
            let cursor_surface = self.compositor.create_surface(qh);
            self.pointer = self
                .seat
                .get_pointer_with_theme(qh, &seat, self.shm.wl_shm(), cursor_surface, ThemeSpec::default())
                .map_err(|e| warn!("No pointer for the dock: {e}"))
                .ok();
        }
    }

    fn remove_capability(
        &mut self,
        _: &Connection,
        _: &QueueHandle<Self>,
        _: wl_seat::WlSeat,
        capability: Capability,
    ) {
        if capability == Capability::Keyboard {
            if let Some(keyboard) = self.keyboard.take() {
                keyboard.release();
            }
        }
        if capability == Capability::Pointer {
            if let Some(pointer) = self.pointer.take() {
                pointer.pointer().release();
            }
        }
    }

    fn remove_seat(&mut self, _: &Connection, _: &QueueHandle<Self>, _: wl_seat::WlSeat) {}
}

impl PointerHandler for WaylandState {
    fn pointer_frame(
        &mut self,
        conn: &Connection,
        _: &QueueHandle<Self>,
        _: &wl_pointer::WlPointer,
        events: &[PointerEvent],
    ) {
        for event in events {
            let position = Vec2::new(event.position.0 as f32, event.position.1 as f32);
            let input = match event.kind {
                PointerEventKind::Enter { .. } => {
                    debug!("Pointer entered at {position}");
                    // Without this the compositor keeps the cursor of the
                    // surface the pointer came from (e.g. a text I-beam).
                    if let Some(pointer) = &self.pointer {
                        if let Err(e) = pointer.set_cursor(conn, DOCK_CURSOR) {
                            warn!("Failed to set the cursor: {e}");
                        }
                    }
                    InputEvent::CursorMoved(position)
                }
                PointerEventKind::Motion { .. } => InputEvent::CursorMoved(position),
                PointerEventKind::Leave { .. } => {
                    debug!("Pointer left");
                    InputEvent::CursorLeft
                }
                PointerEventKind::Press { button, .. } => {
                    InputEvent::Button(mouse_button(button), ButtonState::Pressed)
                }
                PointerEventKind::Release { button, .. } => {
                    InputEvent::Button(mouse_button(button), ButtonState::Released)
                }
                PointerEventKind::Axis { horizontal, vertical, .. } => InputEvent::Wheel {
                    x: wheel_lines(horizontal),
                    y: wheel_lines(vertical),
                },
            };
            self.pending.push(input);
        }
    }
}

impl KeyboardHandler for WaylandState {
    fn enter(
        &mut self,
        _: &Connection,
        _: &QueueHandle<Self>,
        _: &wl_keyboard::WlKeyboard,
        _: &wl_surface::WlSurface,
        _: u32,
        _: &[u32],
        _: &[Keysym],
    ) {
        debug!("Keyboard focus gained");
    }

    fn leave(
        &mut self,
        _: &Connection,
        _: &QueueHandle<Self>,
        _: &wl_keyboard::WlKeyboard,
        _: &wl_surface::WlSurface,
        _: u32,
    ) {
        debug!("Keyboard focus lost");
    }

    fn press_key(
        &mut self,
        _: &Connection,
        _: &QueueHandle<Self>,
        _: &wl_keyboard::WlKeyboard,
        _: u32,
        event: KeyEvent,
    ) {
        self.pending.push(InputEvent::Key {
            scan_code: event.raw_code,
            key_code: key_code(event.keysym),
            state: ButtonState::Pressed,
        });
    }

    fn release_key(
        &mut self,
        _: &Connection,
        _: &QueueHandle<Self>,
        _: &wl_keyboard::WlKeyboard,
        _: u32,
        event: KeyEvent,
    ) {
        self.pending.push(InputEvent::Key {
            scan_code: event.raw_code,
            key_code: key_code(event.keysym),
            state: ButtonState::Released,
        });
    }

    fn update_modifiers(
        &mut self,
        _: &Connection,
        _: &QueueHandle<Self>,
        _: &wl_keyboard::WlKeyboard,
        _: u32,
        _: Modifiers,
    ) {
    }
}

impl ShmHandler for WaylandState {
    fn shm_state(&mut self) -> &mut Shm {
        &mut self.shm
    }
}

impl ProvidesRegistryState for WaylandState {
    fn registry(&mut self) -> &mut RegistryState {
        &mut self.registry
    }
    registry_handlers![OutputState, SeatState];
}

delegate_compositor!(WaylandState);
delegate_output!(WaylandState);
delegate_layer!(WaylandState);
delegate_seat!(WaylandState);
delegate_pointer!(WaylandState);
delegate_keyboard!(WaylandState);
delegate_registry!(WaylandState);
delegate_shm!(WaylandState);
