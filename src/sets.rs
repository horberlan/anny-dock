use bevy::prelude::*;

/// Top-level System Sets that define the main execution phases
#[derive(SystemSet, Debug, Hash, PartialEq, Eq, Clone)]
pub enum AppSystemSet {
    Input,
    Logic,
    Events,
    Render,
}

/// Input System Sets for organizing different types of input handling
#[derive(SystemSet, Debug, Hash, PartialEq, Eq, Clone)]
pub enum InputSystemSet {
    Keyboard,
    Mouse,
    Scroll,
}

/// Logic System Sets for organizing business logic and state management
#[derive(SystemSet, Debug, Hash, PartialEq, Eq, Clone)]
pub enum LogicSystemSet {
    StateUpdate,
    Animation,
    DragLogic,
    IconManagement,
}

/// Event System Sets for organizing event processing
#[derive(SystemSet, Debug, Hash, PartialEq, Eq, Clone)]
pub enum EventSystemSet {
    HyprlandEvents,
    InternalEvents,
}

/// Render System Sets for organizing rendering and UI updates
#[derive(SystemSet, Debug, Hash, PartialEq, Eq, Clone)]
pub enum RenderSystemSet {
    TransformUpdate,
    UIUpdate,
    Camera,
}