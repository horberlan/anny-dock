//! Input region of the layer surface: only the icons take input, so clicks on
//! the transparent rest of the dock reach the windows below.

use bevy::math::{IRect, Vec2, Vec3};

/// Surface-local rectangles (origin top-left, y down) covering each icon, given
/// its world translation (origin at the surface center, y up) and scale.
pub fn icon_input_rects(
    icons: impl IntoIterator<Item = (Vec3, f32)>,
    icon_size: f32,
    surface: Vec2,
) -> Vec<IRect> {
    let bounds = IRect::new(0, 0, surface.x as i32, surface.y as i32);
    icons
        .into_iter()
        .filter_map(|(translation, scale)| {
            let center = Vec2::new(
                translation.x + surface.x / 2.0,
                surface.y / 2.0 - translation.y,
            );
            let half = icon_size * scale / 2.0;
            let rect = IRect::new(
                (center.x - half).floor() as i32,
                (center.y - half).floor() as i32,
                (center.x + half).ceil() as i32,
                (center.y + half).ceil() as i32,
            )
            .intersect(bounds);
            (!rect.is_empty()).then_some(rect)
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    const SURFACE: Vec2 = Vec2::new(1000.0, 800.0);

    #[test]
    fn converts_world_centered_y_up_to_surface_top_left_y_down() {
        // Icon at the world origin: centered on the surface.
        let rects = icon_input_rects([(Vec3::ZERO, 1.0)], 60.0, SURFACE);

        assert_eq!(rects, [IRect::new(470, 370, 530, 430)]);
    }

    #[test]
    fn follows_icon_scale_and_position() {
        // Bottom-left icon, scaled 1.5x: 90px box centered at (100, 700) on the surface.
        let rects = icon_input_rects([(Vec3::new(-400.0, -300.0, 0.0), 1.5)], 60.0, SURFACE);

        assert_eq!(rects, [IRect::new(55, 655, 145, 745)]);
    }

    #[test]
    fn clips_to_surface_and_drops_offscreen_icons() {
        let rects = icon_input_rects(
            [(Vec3::new(-500.0, 0.0, 0.0), 1.0), (Vec3::new(2000.0, 0.0, 0.0), 1.0)],
            60.0,
            SURFACE,
        );

        assert_eq!(rects, [IRect::new(0, 370, 30, 430)]);
    }
}
