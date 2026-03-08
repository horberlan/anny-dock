pub mod directory;
pub mod mapping;
pub mod hyprland;

#[cfg(test)]
mod examples;

pub use directory::DirectoryStrategy;
pub use mapping::{MappingStrategy, ApplicationMapper};
pub use hyprland::HyprlandStrategy;