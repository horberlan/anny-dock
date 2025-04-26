<div align="center">
<img src="assets/dock_icon.svg" alt="Anny-Dock Logo" width="120" height="120"/>

# Anny-Dock

[![Rust](https://img.shields.io/badge/rust-%23000000.svg?style=for-the-badge&logo=rust&logoColor=white)](https://www.rust-lang.org/)
[![Bevy](https://img.shields.io/badge/bevy-%23232323.svg?style=for-the-badge&logo=bevy&logoColor=white)](https://bevyengine.org/)
[![Hyprland](https://img.shields.io/badge/Hyprland-222222?style=for-the-badge&logo=Hyprland&logoColor=58E1FF)](https://hyprland.org/)
[![License](https://img.shields.io/badge/license-MIT-blue.svg?style=for-the-badge)](LICENSE)
[![Maintenance](https://img.shields.io/badge/Maintained%3F-yes-green.svg?style=for-the-badge)](https://github.com/username/anny-dock/graphs/commit-activity)

A modern, animated dock for Hyprland built with Rust and Bevy Engine.
Currently exclusive to Hyprland, with plans to support other window managers in the future.

> **Note**: This is my first Rust project! While I'm committed to writing clean and efficient code, 
> I'm also learning along the way. Feedback and suggestions are greatly appreciated as i explore 
> Rust's capabilities and best practices.

[Features](#features) •
[Installation](#installation) •
[Usage](#usage) •
[Configuration](#configuration) •
[Contributing](#contributing)

</div>

## Features

### Smooth Animations
- 🎯 Intelligent hover effects with smooth transitions
- 🔄 Natural diagonal scrolling behavior
- ✨ Frame-rate independent animations
- 🎨 Subtle scale transformations

### Smart Icon Management
- 📱 Dynamic icon loading and scaling
- 🔍 Automatic icon discovery from running applications
- 📌 Pinnable favorite applications
- 🎯 Precise icon positioning with smooth reordering

### Hyprland Integration
- 🖥️ Seamless Hyprland window management
- 🚀 Native Hyprland client detection
- 🎨 Transparent background support
- 🔗 Direct window focusing and management

### Modern Interface
- 🖼️ Transparent background support
- 🎨 High-quality SVG icon rendering
- 📐 Configurable dock size and position
- 🔲 Clean, minimal design

### Performance
- ⚡ Hardware-accelerated rendering
- 🎮 Optimized animation system
- 🔄 Efficient state management
- 📊 Low resource usage

## Requirements

- Hyprland
- Rust 1.75+
- Linux

## Installation

```bash
# Clone the repository
git clone https://github.com/username/anny-dock.git

# Enter the directory
cd anny-dock

# Build the project
cargo build --release

# Run Anny-Dock
cargo run --release
```

## Usage

### Basic Controls
- **Scroll**: Navigate through icons
- **Left Click**: Launch/Focus application
- **Right Click**: Pin/Unpin application
- **T**: Toggle application titles
- **Drag & Drop**: Reorder icons

### Configuration

Anny-Dock can be configured through the `DockConfig` resource:

```rust
pub struct DockConfig {
    pub margin_x: f32,        // Horizontal margin from screen edge
    pub margin_y: f32,        // Vertical margin from screen edge
    pub spacing: f32,         // Space between icons
    pub z_spacing: f32,       // Depth spacing for 3D effect
    pub base_scale: f32,      // Base icon scale
    pub scale_factor: f32,    // Scale factor for animations
    pub scroll_speed: f32,    // Scroll sensitivity
    pub visible_items: usize, // Number of visible icons
}
```

## Architecture

Anny-Dock is built using a modern ECS architecture with Bevy:

### Core Systems
- Animation System
- Hover System
- Scroll System
- Icon Management
- Hyprland Integration
- Event Handling

### Components
- `HoverTarget`: Manages hover states and animations
- `ClientIcon`: Handles icon rendering and scaling
- `ScrollState`: Controls scroll behavior
- `DockConfig`: Manages dock configuration

## Roadmap

### Current
- [x] Hyprland support
- [x] Basic animation system
- [x] Icon management
- [x] Favorite applications

### Planned
- [ ] Support for other window managers
- [ ] Custom themes
- [ ] Plugin system
- [ ] Configuration file
- [ ] Multi-monitor support

## Development

### Prerequisites
- Rust 1.75+
- Bevy dependencies
- Hyprland
- Linux

### Building from Source
```bash
# Debug build
cargo build

# Release build
cargo build --release

# Run tests
cargo test
```

## Contributing

We welcome contributions! Please see our [Contributing Guidelines](CONTRIBUTING.md) for details.

As this is my first Rust project, I'm particularly open to:
- Code reviews and best practices suggestions
- Rust idioms and patterns recommendations
- Performance optimization tips
- Architecture improvement ideas

### Development Process
1. Fork the repository
2. Create your feature branch
3. Commit your changes
4. Push to the branch
5. Create a Pull Request

## License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.

## Acknowledgments

- [Bevy Engine](https://bevyengine.org/) for the game engine
- [Hyprland](https://hyprland.org/) for the window manager integration
- [bevy_svg](https://github.com/Weasy666/bevy_svg) for SVG rendering
- [xdg-utils](https://www.freedesktop.org/wiki/Software/xdg-utils/) for desktop integration

---

<div align="center">

Made with ❤️ using [Rust](https://www.rust-lang.org/) and [Bevy](https://bevyengine.org/)

**Currently Hyprland Only** - More window managers coming soon

</div>
