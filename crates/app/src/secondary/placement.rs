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
}
