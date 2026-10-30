//! Runs the dock as a wlr-layer-shell surface instead of a winit window.
//!
//! winit has no layer-shell support, so this module replaces it: SCTK creates
//! the layer surface, its raw handles go into the primary `Window` as a
//! `RawHandleWrapper` (all bevy_render needs to render into it), and a custom
//! runner pumps Wayland events into the same Bevy events bevy_winit emits.

mod input;
mod region;
mod wayland;

use std::ffi::c_void;

use bevy::app::{AppExit, PluginsState};
use bevy::input::keyboard::KeyboardInput;
use bevy::input::mouse::{MouseButtonInput, MouseScrollUnit, MouseWheel};
use bevy::math::IRect;
use bevy::prelude::*;
use bevy::window::{
    CompositeAlphaMode, CursorLeft, CursorMoved, PrimaryWindow, RawHandleWrapper,
    WindowResolution,
};
use raw_window_handle::{
    RawDisplayHandle, RawWindowHandle, WaylandDisplayHandle, WaylandWindowHandle,
};
use smithay_client_toolkit::{
    compositor::{CompositorState, Region},
    output::OutputState,
    registry::RegistryState,
    seat::SeatState,
    shell::{
        wlr_layer::{Anchor, KeyboardInteractivity, Layer, LayerShell},
        WaylandSurface,
    },
};
use wayland_client::{globals::registry_queue_init, Connection, EventQueue, Proxy};

use crate::config::Config;
use crate::types::HoverTarget;
use region::icon_input_rects;
use wayland::{InputEvent, WaylandState};

const NAMESPACE: &str = "anny-dock";

/// Surface-local rectangles that receive input, written by
/// `update_input_region_system` and applied by the runner when they change.
#[derive(Resource, Default, PartialEq)]
pub struct InputRegion(pub Vec<IRect>);

pub struct LayerShellBackend {
    conn: Connection,
    queue: EventQueue<WaylandState>,
    state: WaylandState,
    compositor: CompositorState,
}

impl LayerShellBackend {
    /// Creates the layer surface and waits for its first configure. Fails when
    /// there is no Wayland compositor or it lacks `zwlr_layer_shell_v1`.
    pub fn connect() -> Result<Self, String> {
        let conn = Connection::connect_to_env().map_err(|e| format!("no Wayland display: {e}"))?;
        let (globals, mut queue) =
            registry_queue_init::<WaylandState>(&conn).map_err(|e| e.to_string())?;
        let qh = queue.handle();
        let compositor = CompositorState::bind(&globals, &qh).map_err(|e| e.to_string())?;
        let layer_shell = LayerShell::bind(&globals, &qh)
            .map_err(|e| format!("zwlr_layer_shell_v1 unavailable: {e}"))?;

        let surface = compositor.create_surface(&qh);
        let layer = layer_shell.create_layer_surface(&qh, surface, Layer::Top, Some(NAMESPACE), None);
        // Anchored to every edge with size 0: the compositor sizes it to the
        // output's usable area, so the dock keeps laying out in full-screen
        // coordinates.
        layer.set_anchor(Anchor::TOP | Anchor::BOTTOM | Anchor::LEFT | Anchor::RIGHT);
        layer.set_size(0, 0);
        layer.set_keyboard_interactivity(KeyboardInteractivity::OnDemand);
        // Click-through until the first frame computes the icon rects.
        let region = Region::new(&compositor).map_err(|e| e.to_string())?;
        layer.wl_surface().set_input_region(Some(region.wl_region()));
        layer.commit();

        let mut state = WaylandState {
            registry: RegistryState::new(&globals),
            seat: SeatState::new(&globals, &qh),
            output: OutputState::new(&globals, &qh),
            layer,
            keyboard: None,
            pointer: None,
            configured_size: None,
            scale_factor: None,
            closed: false,
            pending: Vec::new(),
        };
        // Presenting before the first configure is a protocol error.
        while state.configured_size.is_none() && !state.closed {
            queue.blocking_dispatch(&mut state).map_err(|e| e.to_string())?;
        }
        if state.closed {
            return Err("layer surface closed by the compositor".into());
        }

        Ok(Self { conn, queue, state, compositor })
    }

    /// Spawns the primary window backed by the layer surface. Must run before
    /// `DefaultPlugins`: RenderPlugin picks the GPU adapter from this surface
    /// while it builds.
    pub fn spawn_window(&self, world: &mut World) -> Entity {
        let (width, height) = self.state.configured_size.unwrap_or_default();

        let mut window_handle = WaylandWindowHandle::empty();
        window_handle.surface = self.state.layer.wl_surface().id().as_ptr() as *mut c_void;
        let mut display_handle = WaylandDisplayHandle::empty();
        display_handle.display = self.conn.backend().display_ptr() as *mut c_void;

        world
            .spawn((
                Window {
                    title: NAMESPACE.into(),
                    resolution: WindowResolution::new(width as f32, height as f32),
                    transparent: true,
                    decorations: false,
                    composite_alpha_mode: CompositeAlphaMode::PreMultiplied,
                    ..default()
                },
                PrimaryWindow,
                RawHandleWrapper {
                    window_handle: RawWindowHandle::Wayland(window_handle),
                    display_handle: RawDisplayHandle::Wayland(display_handle),
                },
            ))
            .id()
    }

    /// Replaces the winit runner with one driven by the Wayland connection.
    pub fn install(self, app: &mut App, window: Entity) {
        app.init_resource::<InputRegion>()
            .set_runner(move |app| self.run(app, window));
    }

    fn run(mut self, mut app: App, window: Entity) {
        // Same plugin finalization App::run's default runner does.
        while app.plugins_state() != PluginsState::Ready {
            bevy::tasks::tick_global_task_pools_on_main_thread();
        }
        app.finish();
        app.cleanup();

        let mut logical_size = self.state.configured_size.take().unwrap_or_default();
        let mut scale = 1;
        let mut applied_region: Option<Vec<IRect>> = None;

        loop {
            if let Err(e) = self.conn.flush() {
                error!("Wayland connection lost: {e}");
                break;
            }
            if let Some(guard) = self.queue.prepare_read() {
                // The socket is non-blocking: WouldBlock only means no new events.
                let _ = guard.read();
            }
            if let Err(e) = self.queue.dispatch_pending(&mut self.state) {
                error!("Wayland dispatch failed: {e}");
                break;
            }

            let resized = self.state.configured_size.take();
            let rescaled = self.state.scale_factor.take();
            if resized.is_some() || rescaled.is_some() {
                logical_size = resized.unwrap_or(logical_size);
                scale = rescaled.unwrap_or(scale).max(1);
                self.state.layer.wl_surface().set_buffer_scale(scale);
                if let Some(mut win) = app.world.get_mut::<Window>(window) {
                    win.resolution.set_scale_factor(scale as f64);
                    win.resolution.set_physical_resolution(
                        logical_size.0 * scale as u32,
                        logical_size.1 * scale as u32,
                    );
                }
            }
            for event in std::mem::take(&mut self.state.pending) {
                send_input(&mut app.world, window, event);
            }

            app.update();

            // Double-buffered: applied by the commit of the next presented frame.
            let region = &app.world.resource::<InputRegion>().0;
            if applied_region.as_ref() != Some(region) {
                if let Ok(wl_region) = Region::new(&self.compositor) {
                    for rect in region {
                        wl_region.add(rect.min.x, rect.min.y, rect.width(), rect.height());
                    }
                    self.state.layer.wl_surface().set_input_region(Some(wl_region.wl_region()));
                    debug!("Input region: {region:?}");
                    applied_region = Some(region.clone());
                }
            }

            if self.state.closed || !app.world.resource::<Events<AppExit>>().is_empty() {
                break;
            }
        }
    }
}

fn send_input(world: &mut World, window: Entity, event: InputEvent) {
    match event {
        InputEvent::CursorMoved(position) => {
            if let Some(mut win) = world.get_mut::<Window>(window) {
                win.set_cursor_position(Some(position));
            }
            world.send_event(CursorMoved { window, position });
        }
        InputEvent::CursorLeft => {
            if let Some(mut win) = world.get_mut::<Window>(window) {
                win.set_cursor_position(None);
            }
            world.send_event(CursorLeft { window });
        }
        InputEvent::Button(button, state) => {
            world.send_event(MouseButtonInput { button, state, window });
        }
        InputEvent::Wheel { x, y } => {
            world.send_event(MouseWheel { unit: MouseScrollUnit::Line, x, y, window });
        }
        InputEvent::Key { scan_code, key_code, state } => {
            world.send_event(KeyboardInput { scan_code, key_code, state, window });
        }
    }
}

/// Keeps `InputRegion` on the icons' current bounds. No-op without the
/// layer-shell backend (the resource only exists there).
pub fn update_input_region_system(
    region: Option<ResMut<InputRegion>>,
    windows: Query<&Window, With<PrimaryWindow>>,
    icons: Query<&Transform, With<HoverTarget>>,
    config: Res<Config>,
) {
    let (Some(mut region), Ok(window)) = (region, windows.get_single()) else {
        return;
    };
    let rects = icon_input_rects(
        icons.iter().map(|t| (t.translation, t.scale.x)),
        config.icon_size,
        Vec2::new(window.width(), window.height()),
    );
    if region.0 != rects {
        region.0 = rects;
    }
}
