//! Safe-area padding for edge-to-edge Android. Plain geometry with no Android or
//! egui types, so the pixel-to-point conversion can be tested on the desktop.

/// Top padding in points used when the measured inset is smaller. This is an
/// estimate, not a measured value: a typical status bar plus a little room.
pub const FALLBACK_TOP: f32 = 36.0;

/// Bottom padding in points used when the measured inset is smaller. This is an
/// estimate for the navigation bar, not a measured value.
pub const FALLBACK_BOTTOM: f32 = 32.0;

/// A rectangle in physical pixels, with the same edges as android-activity's `Rect`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PixelRect {
    pub left: i32,
    pub top: i32,
    pub right: i32,
    pub bottom: i32,
}

/// Space between each window edge and the content, in points.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct Insets {
    pub left: f32,
    pub top: f32,
    pub right: f32,
    pub bottom: f32,
}

/// Converts the content rectangle into insets in points. `window_width` and
/// `window_height` are the window size in pixels. An empty rectangle, which
/// Android reports before its first measurement, gives zero insets.
pub fn insets_from_content_rect(
    content: PixelRect,
    window_width: i32,
    window_height: i32,
    pixels_per_point: f32,
) -> Insets {
    let empty = content.right <= content.left || content.bottom <= content.top;
    if empty || !pixels_per_point.is_finite() || pixels_per_point <= 0.0 {
        return Insets::default();
    }
    let to_points = |pixels: i32| pixels.max(0) as f32 / pixels_per_point;
    Insets {
        left: to_points(content.left),
        top: to_points(content.top),
        right: to_points(window_width - content.right),
        bottom: to_points(window_height - content.bottom),
    }
}

/// Raises the top and bottom insets to the fallback estimates. A larger measured
/// inset is kept.
pub fn with_fallback(measured: Insets) -> Insets {
    Insets {
        top: measured.top.max(FALLBACK_TOP),
        bottom: measured.bottom.max(FALLBACK_BOTTOM),
        ..measured
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn assert_close(actual: f32, expected: f32) {
        assert!(
            (actual - expected).abs() < 0.01,
            "expected {expected}, got {actual}"
        );
    }

    #[test]
    fn pixels_become_points_at_the_window_density() {
        // 1080x2400 px window at 2.625 px per point, with a 96 px status bar and
        // a 116 px navigation bar.
        let content = PixelRect {
            left: 0,
            top: 96,
            right: 1080,
            bottom: 2284,
        };
        let insets = insets_from_content_rect(content, 1080, 2400, 2.625);
        assert_close(insets.left, 0.0);
        assert_close(insets.top, 96.0 / 2.625);
        assert_close(insets.right, 0.0);
        assert_close(insets.bottom, 116.0 / 2.625);
    }

    #[test]
    fn landscape_insets_include_the_sides() {
        let content = PixelRect {
            left: 132,
            top: 72,
            right: 2268,
            bottom: 1080,
        };
        let insets = insets_from_content_rect(content, 2400, 1080, 3.0);
        assert_close(insets.left, 44.0);
        assert_close(insets.top, 24.0);
        assert_close(insets.right, 44.0);
        assert_close(insets.bottom, 0.0);
    }

    #[test]
    fn empty_or_invalid_input_gives_no_insets() {
        let empty = PixelRect {
            left: 0,
            top: 0,
            right: 0,
            bottom: 0,
        };
        assert_eq!(
            insets_from_content_rect(empty, 1080, 2400, 2.625),
            Insets::default()
        );
        let content = PixelRect {
            left: 0,
            top: 96,
            right: 1080,
            bottom: 2284,
        };
        assert_eq!(
            insets_from_content_rect(content, 1080, 2400, 0.0),
            Insets::default()
        );
        assert_eq!(
            insets_from_content_rect(content, 1080, 2400, f32::NAN),
            Insets::default()
        );
    }

    #[test]
    fn a_content_rect_larger_than_the_window_gives_no_negative_inset() {
        let content = PixelRect {
            left: 0,
            top: 0,
            right: 1200,
            bottom: 2500,
        };
        let insets = insets_from_content_rect(content, 1080, 2400, 2.0);
        assert_eq!(insets.right, 0.0);
        assert_eq!(insets.bottom, 0.0);
    }

    #[test]
    fn fallback_sets_a_minimum_for_top_and_bottom_only() {
        let small = with_fallback(Insets {
            left: 4.0,
            top: 10.0,
            right: 5.0,
            bottom: 12.0,
        });
        assert_eq!(small.top, FALLBACK_TOP);
        assert_eq!(small.bottom, FALLBACK_BOTTOM);
        assert_eq!((small.left, small.right), (4.0, 5.0));

        let large = with_fallback(Insets {
            left: 0.0,
            top: 48.0,
            right: 0.0,
            bottom: 60.0,
        });
        assert_eq!((large.top, large.bottom), (48.0, 60.0));
    }
}
