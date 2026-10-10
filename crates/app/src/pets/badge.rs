//! The level badge the Pets window's rows and the soul tracker card share
//! (tokens.md, "Level colours"): Lv 0 grey, Lv 1 and Lv 2 greener, Lv 3 a
//! solid green with ink text. "Lv N" always stays, so colour is never the
//! only cue.

use gpui::{div, prelude::*, rgba, Div, FontWeight};

use crate::i18n::fl;
use crate::secondary::chrome::Scale;

/// Each level's fill: Lv 0 to Lv 2 are their tint laid over the track's dark
/// plate `rgba(0, 0, 0, 0.34)` and blended into one colour, so they read over
/// a bright scene at low opacity; Lv 3 is `good` at 0.86.
const LEVEL_FILL: [u32; 4] = [0x39363366, 0x21382968, 0x48795a86, 0x86e3a8db];
/// `muted`, `good` at 0.66, `good`, ink.
const LEVEL_TEXT: [u32; 4] = [0xfff6eca8, 0x86e3a8a8, 0x86e3a8ff, 0x17120dff];

/// "Lv 2" in the level's colours: `width` fixes the window's 30 px badge,
/// none hugs the text with 6 px sides as the card's does.
pub fn level_badge(level: u32, width: Option<f32>, height: f32, s: Scale) -> Div {
    let colour = level.min(3) as usize;
    div()
        .flex_none()
        .h(s.px(height))
        .flex()
        .items_center()
        .justify_center()
        .when_some(width, |badge, width| badge.w(s.px(width)))
        .when(width.is_none(), |badge| badge.px(s.px(6.)))
        .rounded(s.px(4.))
        .bg(rgba(LEVEL_FILL[colour]))
        .text_color(rgba(LEVEL_TEXT[colour]))
        .text_size(s.px(11.))
        .font_weight(FontWeight::SEMIBOLD)
        .child(fl!("pets-level", level = level))
}
