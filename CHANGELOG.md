# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.1.0] - 2026-11-07

### Added
- The dock runs as a Wayland layer surface (wlr-layer-shell) instead of a window, so it needs no Hyprland window rules. Clicks on its empty, transparent area reach the windows below
- `[layer_shell]` section in `config.toml`: `layer`, `output` (monitor) and `keyboard` (focus behavior)
- `ANNY_DOCK_WINDOWED=1` runs the dock as a regular window; it also falls back to a window when the compositor lacks wlr-layer-shell
- Live drop preview while dragging: neighbors make room before you drop, and the icon lands exactly where the preview shows
- Dragging past the first or last visible icon scrolls the dock
- The dock order is saved to `~/.local/state/anny-dock/order.json` and restored on the next start
- Held arrow keys keep scrolling

### Changed
- Favorites are no longer forced to the front: a favorite means "always present", and any icon can be dragged anywhere
- One icon per open window on startup, as for windows opened later
- Scrolling is smooth and measured in icons: arrow keys move one icon per press, the mouse wheel settles on a whole icon when it stops, and the perspective follows the scroll
- Favorites are stored in `~/.config/anny-dock/favorites.json`; the old file in the directory the dock was started from is copied there on first start
- Missing `config.toml` fields take their default values, and an invalid file is reported and left untouched

### Fixed
- Pixelated icons and titles
- Dragged icons landing in the wrong slot
- Icons of windows open at startup staying in the dock after the window closed
- Windows of the same app on other workspaces missing from the dock on startup
- The cursor over the dock keeping the shape of the window it came from
- `Esc` during a drag also closing the dock
- `cargo install anny-dock` failing with "No asset folder found"
- An old `config.toml` missing a field crashing the dock on startup

### Removed
- `bevy_embedded_assets` dependency (no assets were loaded through it)

## [0.0.5] - 2026-03-08

### Fixed
- **Critical:** Duplicate icons being created when windows open
- **Critical:** Favorites not repositioning when toggled on/off
- **Critical:** Incorrect icon indices causing keybind issues
- Loss of saved favorites ordering on application restart
- Manual dock_order manipulation causing ordering inconsistencies

### Changed
- Simplified `toggle_favorite` logic to delegate ordering to reorder system
- Implemented automatic `dock_order` rebuild in `reorder_icons_system`
- Reorder system now uses `Favorite` component to determine correct icon ordering

### Added
- Validation in `handle_hypr_open_window` to prevent duplicate icons
- Validation in `process_new_windows` to prevent duplicate icons
- Automatic dock order reconstruction based on favorites list
- Icon strategy system with multiple resolution strategies (directory, hyprland, mapping)
- Icon caching system for improved performance
- Icon resolver with fallback chain

### Removed
- Duplicate component insertions (`HoverTarget`, `ClientClass`)
- Complex manual ordering logic in `toggle_favorite`

### Technical Details
The `reorder_icons_system` now intelligently rebuilds `dock_order` when triggered, using the `Favorite` component and favorites list to maintain correct ordering. This eliminates complex manual ordering logic and ensures consistency across all operations. Favorites always appear first in their saved order, followed by non-favorites.

## [0.0.4] - 2025-06-XX

### Fixed
- Drag action improvements
- Scroll state rollback
- Scale on hover behavior

### Changed
- Refined perspective and spacing logic of icons
- Updated drag system to get first item over cursor

### Removed
- Automatic window positioning from config

## Previous Versions

See git history for changes in versions prior to 0.0.4.

---

## Legend

- **Added** for new features
- **Changed** for changes in existing functionality
- **Deprecated** for soon-to-be removed features
- **Removed** for now removed features
- **Fixed** for any bug fixes
- **Security** in case of vulnerabilities
