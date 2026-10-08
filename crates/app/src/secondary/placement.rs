/// A rectangle in device pixels, as Windows reports windows and monitors.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Rect {
    pub left: i32,
    pub top: i32,
    pub right: i32,
    pub bottom: i32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Side {
    Right,
    Left,
    Below,
    Above,
}

impl Side {
    fn opposite(self) -> Self {
        match self {
            Self::Right => Self::Left,
            Self::Left => Self::Right,
            Self::Below => Self::Above,
            Self::Above => Self::Below,
        }
    }
}

const GAP: i32 = 8;

/// Where a new window's top-left corner goes next to `anchor`: on the
/// preferred side if it fits the monitor's work area, else on the opposite
/// side, else squeezed back inside the work area.
pub fn beside(anchor: Rect, (width, height): (i32, i32), work: Rect, preferred: Side) -> (i32, i32) {
    let corner = |side: Side| match side {
        Side::Right => (anchor.right + GAP, anchor.top),
        Side::Left => (anchor.left - GAP - width, anchor.top),
        Side::Below => (anchor.left, anchor.bottom + GAP),
        Side::Above => (anchor.left, anchor.top - GAP - height),
    };
    let fits = |(x, y): (i32, i32)| {
        x >= work.left && y >= work.top && x + width <= work.right && y + height <= work.bottom
    };
    let (x, y) = [preferred, preferred.opposite()]
        .into_iter()
        .map(corner)
        .find(|corner| fits(*corner))
        .unwrap_or_else(|| corner(preferred));
    (
        x.min(work.right - width).max(work.left),
        y.min(work.bottom - height).max(work.top),
    )
}

/// Which side of its anchor a stacked window takes when both have room.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Stack {
    Above,
    Below,
}

/// Where a window `gap` from `anchor` goes, sharing its left edge: on the
/// `preferred` side, or the other one when the work area has no room there.
/// It is never moved sideways, so it stays with an anchor that sits partly
/// off the work area.
pub fn stacked(anchor: Rect, (_, height): (i32, i32), work: Rect, gap: i32, preferred: Stack) -> (i32, i32) {
    let above = anchor.top - gap - height;
    let below = anchor.bottom + gap;
    let fits_above = above >= work.top;
    let fits_below = below + height <= work.bottom;
    let y = match preferred {
        Stack::Above if fits_above || !fits_below => above,
        Stack::Above => below,
        Stack::Below if fits_below || !fits_above => below,
        Stack::Below => above,
    };
    (anchor.left, y)
}

/// Where a window opens: next to its anchor window, or in the middle of its
/// screen.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Anchor {
    Beside(Side),
    Centre,
    /// On the anchor's left edge, `gap` logical pixels from it.
    Stacked { gap: f32, preferred: Stack },
}

impl Anchor {
    /// `anchor`, `size` and `work` are in device pixels, at `scale` device
    /// pixels per logical one.
    pub fn corner(self, anchor: Rect, size: (i32, i32), work: Rect, scale: f32) -> (i32, i32) {
        match self {
            Self::Beside(side) => beside(anchor, size, work, side),
            Self::Centre => centred(size, work),
            Self::Stacked { gap, preferred } => stacked(anchor, size, work, (gap * scale).round() as i32, preferred),
        }
    }
}

/// The top-left corner that centres a window in the work area, kept inside it.
pub fn centred((width, height): (i32, i32), work: Rect) -> (i32, i32) {
    (
        (work.left + (work.right - work.left - width) / 2).max(work.left),
        (work.top + (work.bottom - work.top - height) / 2).max(work.top),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    const WORK: Rect = Rect { left: 0, top: 0, right: 1920, bottom: 1040 };

    fn meter(left: i32, top: i32) -> Rect {
        Rect { left, top, right: left + 310, bottom: top + 280 }
    }

    #[test]
    fn a_window_goes_on_the_preferred_side_when_it_fits() {
        assert_eq!(beside(meter(64, 64), (520, 600), WORK, Side::Right), (382, 64));
        assert_eq!(beside(meter(64, 64), (370, 400), WORK, Side::Below), (64, 352));
    }

    #[test]
    fn a_window_flips_to_the_other_side_at_the_screen_edge() {
        assert_eq!(beside(meter(1500, 64), (520, 600), WORK, Side::Right), (972, 64));
        assert_eq!(beside(meter(64, 700), (370, 400), WORK, Side::Below), (64, 292));
    }

    #[test]
    fn a_window_that_fits_nowhere_is_pulled_inside_the_work_area() {
        let narrow = Rect { left: 0, top: 0, right: 600, bottom: 1040 };

        assert_eq!(beside(meter(150, 64), (520, 600), narrow, Side::Right), (80, 64));
    }

    #[test]
    fn a_tall_window_is_pulled_up_off_the_taskbar() {
        assert_eq!(beside(meter(64, 900), (520, 300), WORK, Side::Right), (382, 740));
    }

    #[test]
    fn a_centred_window_sits_in_the_middle_of_the_work_area() {
        assert_eq!(centred((520, 420), WORK), (700, 310));
        assert_eq!(Anchor::Centre.corner(meter(64, 64), (520, 420), WORK, 1.0), (700, 310));
        assert_eq!(Anchor::Beside(Side::Right).corner(meter(64, 64), (520, 600), WORK, 1.0), (382, 64));
    }

    #[test]
    fn a_window_larger_than_the_work_area_keeps_its_top_left_corner_on_screen() {
        let small = Rect { left: 100, top: 50, right: 400, bottom: 300 };
        assert_eq!(centred((520, 420), small), (100, 50));
    }

    #[test]
    fn a_stacked_window_sits_above_the_meter_on_its_left_edge() {
        assert_eq!(stacked(meter(64, 400), (310, 84), WORK, 6, Stack::Above), (64, 310));
        let anchor = Anchor::Stacked { gap: 6.0, preferred: Stack::Above };
        assert_eq!(anchor.corner(meter(64, 400), (465, 126), WORK, 1.5), (64, 265));
    }

    #[test]
    fn a_stacked_window_goes_below_a_meter_at_the_top_of_the_screen() {
        assert_eq!(stacked(meter(64, 40), (310, 84), WORK, 6, Stack::Above), (64, 326));
    }

    #[test]
    fn a_stacked_window_never_overlaps_the_meter_and_follows_it_off_the_work_area() {
        let tall = Rect { left: 64, top: 40, right: 374, bottom: 1000 };
        assert_eq!(stacked(tall, (310, 84), WORK, 6, Stack::Above), (64, -50), "no room either side: above, off the top");
        assert_eq!(
            stacked(meter(-4000, 0), (310, 84), WORK, 6, Stack::Above),
            (-4000, 286),
            "kept beside a meter outside every monitor"
        );
    }

    #[test]
    fn a_window_stacked_under_the_toolbar_goes_above_it_when_the_toolbar_is_low() {
        let toolbar = Rect { left: 800, top: 8, right: 1230, bottom: 42 };
        assert_eq!(stacked(toolbar, (310, 84), WORK, 6, Stack::Below), (800, 48));
        let low = Rect { left: 800, top: 990, right: 1230, bottom: 1024 };
        assert_eq!(stacked(low, (310, 84), WORK, 6, Stack::Below), (800, 900));
    }
}
