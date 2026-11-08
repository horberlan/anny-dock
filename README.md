<div align="center">
<img src="assets/icons/dock_icon.svg" alt="anny-dock Logo" width="120" height="120"/>

# anny-dock

[![Rust](https://img.shields.io/badge/rust-%23000000.svg?style=for-the-badge&logo=rust&logoColor=white)](https://www.rust-lang.org/)
[![Bevy](https://img.shields.io/badge/bevy-%23232323.svg?style=for-the-badge&logo=bevy&logoColor=white)](https://bevyengine.org/)
[![Hyprland](https://img.shields.io/badge/Hyprland-222222?style=for-the-badge&logo=Hyprland&logoColor=58E1FF)](https://hyprland.org/)
[![License](https://img.shields.io/badge/license-MIT-blue.svg?style=for-the-badge)](LICENSE)
[![Maintenance](https://img.shields.io/badge/Maintained%3F-yes-green.svg?style=for-the-badge)](https://github.com/horberlan/anny-dock/graphs/commit-activity)

A modern, animated dock for Hyprland built with Rust and Bevy Engine.

> **Note**: This is my first Rust project! While I'm committed to writing clean and efficient code, 
> I'm also learning along the way. Feedback and suggestions are greatly appreciated as I explore 
> Rust's capabilities and best practices.

[Appearance](#appearance) •
[Installation](#installation) •
[Usage](#usage) •
[Configuration](#configuration) •
[Contributing](#contributing)

</div>

## Appearance

[![anny-dock running on Hyprland (click to watch the video)](https://img.youtube.com/vi/yGkF93IRhU8/maxresdefault.jpg)](https://www.youtube.com/watch?v=yGkF93IRhU8)

## Features

- Runs as a layer surface (wlr-layer-shell): no Hyprland window rules needed, and clicks on the empty, transparent area go through to the windows below
- One icon per open window, kept in sync live through Hyprland's IPC socket
- Pinned (favorite) apps always show up, even with no window open
- Drag & drop reordering with a live preview of the drop slot; the order is remembered between sessions
- Smooth scrolling with the mouse wheel, touchpad and arrow keys, with diagonal perspective
- Crisp icons and titles on any icon size and display scale

## Requirements

- Hyprland
- Rust 1.75+ (to build)

## Installation
You can install the **anny-dock** manually by cloning this repo and building from source:

```bash
# Clone the repository
git clone https://github.com/horberlan/anny-dock.git

# Enter the directory
cd anny-dock

# Build the project
cargo build --release

# Run anny-dock
cargo run --release
```
or downloading it and installing it directly (from [Crates](https://crates.io))

```bash
cargo install anny-dock
```

Besides, you may opt for moving the target path to your local binaries for turning the **anny-dock** an executable in your system $PATH:
```bash
cp target/release/anny-dock /usr/local/bin
```

### Starting with Hyprland

Add it to `~/.config/hypr/hyprland.conf` to start the dock with your session:

```ini
exec-once = anny-dock
```

To stop it, press **Q** or **Esc** while the dock has keyboard focus, or run `pkill anny-dock`.

## Usage

### Basic Controls

- **Left Click**: Launch/Focus application
- **Right Click**: Pin/Unpin application
- **Mouse Wheel / Touchpad**: Scroll through icons; the dock settles on a whole icon when you stop
- **Arrow Keys (←/→)**: Scroll one icon per press; hold to keep scrolling
- **Drag & Drop**: Reorder icons; neighbors make room before you drop, and dragging past the first or last visible icon scrolls the dock
- **Esc** (while dragging): Cancel the drag
- **1..9, 0**: Launch/Focus the visible applications, in order
- **T**: Toggle application titles
- **Q/Esc**: Close the dock

Keyboard shortcuts work while the dock has keyboard focus (see `keyboard` under [Configuration](#configuration)).

### Pinned apps and order

- Pinned apps are always in the dock: with no window open they show as a faded icon, and clicking it launches the app.
- The dock has one icon per window. When a pinned app has several windows, the first one uses the pinned icon and the others get their own icons.
- Pinned apps are not forced to the front: drag any icon anywhere.
- The dock order is saved whenever it changes and restored on the next start. Windows are matched by app, since window addresses change between sessions: with three terminals open, their three positions are restored, but not which window was in which position. Apps opened for the first time go to the end.

## Configuration

anny-dock's configuration can be customized through a TOML file located at `~/.config/anny-dock/config.toml`. These are the options and their defaults:

```toml
visible_items = 8        # Number of icons visible at once (also how many number keys are active)
margin_x = 85.0          # Horizontal margin from screen edge
margin_y = 50.0          # Vertical margin from screen edge
spacing = 40.0           # Space between icons
z_spacing = 2.0          # Depth spacing for 3D effect
base_scale = 1.2         # Base icon scale
scale_factor = 0.9       # How much each icon shrinks along the dock, for the perspective
scroll_speed = 15.0      # Mouse wheel speed, in pixels per wheel step
tilt_y = 0.25            # Inclination factor (0.0 = horizontal, 0.5 = strong diagonal)
icon_size = 56.0         # Base size of icons

[layer_shell]            # How the dock is placed on screen (wlr-layer-shell)
layer = "top"            # "background", "bottom", "top" or "overlay" (also above fullscreen)
output = ""              # Monitor name from `hyprctl monitors` (e.g. "eDP-1"); empty = focused monitor
keyboard = "on_demand"   # "on_demand" (when you interact with the dock), "exclusive" or "none" (disables shortcuts)
```

The dock runs as a layer surface, so it needs no Hyprland window rules. Set `ANNY_DOCK_WINDOWED=1` to run it as a regular window instead (it also falls back to a window when the compositor lacks wlr-layer-shell). The `[layer_shell]` options are ignored in window mode.

#### Configuration File Location

- Default location: `~/.config/anny-dock/config.toml`
- The file will be created automatically on first run if it doesn't exist
- All values are optional - missing values will use defaults
- An invalid file is reported in the log and the defaults are used, without overwriting it
- Changes take effect on the next start

#### Dock Inclination (`tilt_y`)

- The `tilt_y` field controls the inclination of the dock towards the center of the screen.
- Example: `tilt_y = 0.25` makes the dock point to a spot 25% of the screen height above the center.
- Adjust this value to customize the dock's angle to your visual preference.

### Files

| File | Contents |
|------|----------|
| `~/.config/anny-dock/config.toml` | Configuration (edited by you) |
| `~/.config/anny-dock/favorites.json` | Pinned apps (written by the dock). Versions up to 0.0.5 kept it in the directory the dock was started from; it is copied to the new location on first start |
| `~/.local/state/anny-dock/order.json` | Dock order from the last session (written by the dock) |

### Troubleshooting

Set `RUST_LOG` to see what the dock is doing, for example where icons come from and the layer surface's input events:

```bash
RUST_LOG=info anny-dock
RUST_LOG=warn,anny_dock::layer_shell=debug anny-dock
```

## Architecture

anny-dock is built using an ECS architecture with Bevy:

- `src/layer_shell/`: Wayland layer-shell backend. It replaces winit, which has no layer-shell support: it creates the layer surface, feeds pointer and keyboard input into Bevy and keeps the input region on the icons
- `src/systems/`: scroll, drag & drop, reordering, keybinds, titles
- `src/components/`: icon spawning, pins, titles
- `src/utils/`: layout, hover animation, icon loading, app launching, persistence
- `src/main.rs`: app setup and the Hyprland IPC integration (`hyprctl clients -j` at startup, `.socket2.sock` events afterwards)

### Main Types
- `DockOrder`: Order of the icons in the dock
- `HoverTarget`: Slot, scale and depth each icon animates to
- `ScrollState`: Scroll position, measured in icons
- `Config`: Dock configuration

## Roadmap

### Current
- [x] Hyprland support
- [x] Basic animation system
- [x] Icon management
- [x] Favorite applications
- [x] Scroll using keyboard arrows
- [x] Configuration file
- [x] Migrate to hyprland layer
- [x] Drag & drop reordering, remembered between sessions

### Planned
- [ ] Add it to the Arch User Repository
- [ ] Include custom themes
- [ ] Multi-monitor support (today the dock runs on one monitor, chosen with `output`)

## Development

### Prerequisites
- Rust 1.75+
- Bevy dependencies
- Hyprland

### Building from Source
```bash
# Debug build
cargo build

# Release build
cargo build --release

# Tests
cargo test
```

## Contributing

We welcome contributions!

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

- [Bevy Engine](https://bevyengine.org/) for the UI
- [Hyprland](https://hyprland.org/) for the window manager integration
- [Smithay Client Toolkit](https://github.com/Smithay/client-toolkit) for the Wayland layer-shell integration
- [bevy_svg](https://github.com/Weasy666/bevy_svg) for SVG rendering
- [xdg-utils](https://www.freedesktop.org/wiki/Software/xdg-utils/) for desktop integration

---

<div align="center">

Made with ❤️ using [Rust](https://www.rust-lang.org/) and [Bevy](https://bevyengine.org/)

</div>
