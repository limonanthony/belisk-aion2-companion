//! Drawing pieces every overlay window shares: the user's content scale and
//! the soft class-coloured bar (docs/gpui-mockups/approved/tokens.md).

use gpui::{div, linear_color_stop, linear_gradient, prelude::*, px, relative, rgba, Background, Div, Pixels};

pub const TRACK: u32 = 0x00000057;
const NEUTRAL_FROM: u32 = 0xfff0e10a;
const NEUTRAL_TO: u32 = 0xfff0e129;
const HP_FROM: u32 = 0xe8735f29;
const HP_TO: u32 = 0xe8735f8c;

/// Content pixels at the user's content scale.
#[derive(Clone, Copy)]
pub struct Scale(pub f32);

impl Scale {
    pub fn px(self, value: f32) -> Pixels {
        px(value * self.0)
    }
}

/// `colour` is a class colour (or a player's shade of it) as `0xRRGGBB`;
/// none draws the quieter cream bar.
pub fn class_fill(colour: Option<u32>) -> Background {
    match colour {
        Some(colour) => {
            let rgb = colour << 8;
            linear_gradient(90., linear_color_stop(rgba(rgb | 0x24), 0.), linear_color_stop(rgba(rgb | 0x80), 1.))
        }
        None => linear_gradient(90., linear_color_stop(rgba(NEUTRAL_FROM), 0.), linear_color_stop(rgba(NEUTRAL_TO), 1.)),
    }
}

/// Health: the gauge colour as a bar.
pub fn hp_fill() -> Background {
    linear_gradient(90., linear_color_stop(rgba(HP_FROM), 0.), linear_color_stop(rgba(HP_TO), 1.))
}

/// A soft rectangle that brightens to its leading edge, clipped by the track.
pub fn track(fill: Background, length: f32, height: f32, s: Scale) -> Div {
    bar(fill, length, height, 5., s)
}

/// [`track`] with any corner radius: the 4 px coverage bars use 2 px.
pub fn bar(fill: Background, length: f32, height: f32, radius: f32, s: Scale) -> Div {
    div()
        .relative()
        .flex_1()
        .min_w_0()
        .h(s.px(height))
        .rounded(s.px(radius))
        .bg(rgba(TRACK))
        .overflow_hidden()
        .child(
            div()
                .absolute()
                .top_0()
                .bottom_0()
                .left_0()
                .w(relative(length.clamp(0.0, 1.0)))
                .rounded(s.px(radius))
                .bg(fill),
        )
}
