use bevy::prelude::*;
use serde::Deserialize;
use std::collections::HashMap;
use std::sync::{Arc, Mutex, mpsc::Receiver};

#[derive(Deserialize, Debug, Clone)]
pub struct Client {
    pub class: String,
    pub address: String,
    #[serde(default)]
    pub name: Option<String>,
}

#[derive(Component)]
pub struct ClientIcon;

#[derive(Component, Debug)]
pub struct ClientAddress(pub String);

#[derive(Component, Debug)]
pub struct ClientClass(pub String);

#[derive(Resource)]
pub struct ClientList(pub Vec<Client>);

#[derive(Resource)]
pub struct ShowTitles(pub bool);

#[derive(Component)]
pub struct Dragging {
    pub offset: Vec2,
}

#[derive(Resource, Default)]
pub struct UiState {
    pub dragging: Option<Entity>,
    pub click_origin: Option<Vec2>,
}

#[derive(Component)]
pub struct HoverTarget {
    pub original_position: Vec2,
    pub original_z: f32,
    pub original_scale: f32,
    pub index: usize,
    pub is_hovered: bool,
    pub hover_exit_timer: Option<Timer>,
}

#[derive(Component)]
pub struct IconText(pub Entity);

#[derive(Resource, Default)]
pub struct IconPositions(pub HashMap<Entity, (Vec3, Vec3)>);

#[derive(Resource)]
pub struct ReorderTrigger(pub bool);

impl Default for ReorderTrigger {
    fn default() -> Self {
        ReorderTrigger(false)
    }
}

#[derive(Component)]
pub struct MainCamera;

#[derive(SystemSet, Debug, Hash, PartialEq, Eq, Clone)]
pub struct StateUpdate;

#[derive(SystemSet, Debug, Hash, PartialEq, Eq, Clone)]
pub struct ReorderIcons;

#[derive(Resource, Default)]
pub struct DockOrder(pub Vec<String>);

pub const ICON_SIZE: f32 = 56.0;

#[derive(Event)]
pub struct IconRemovedEvent(pub String);

#[derive(Resource)]
pub struct ScrollState {
    pub offset: Vec2,
    pub total_scroll_distance: f32,
    pub max_visible_items: usize,
}

impl Default for ScrollState {
    fn default() -> Self {
        Self {
            offset: Vec2::ZERO,
            total_scroll_distance: 0.0,
            max_visible_items: 8,
        }
    }
}

#[derive(Resource, Clone)]
pub struct HyprlandEventReceiver(pub Arc<Mutex<Receiver<HyprIpcEvent>>>);

#[derive(Debug, Clone)]
pub enum HyprIpcEvent {
    OpenWindow {
        address: String,
        workspace: String,
        class: String,
        title: String,
    },
    CloseWindow {
        address: String,
    },
    Other(String),
}
