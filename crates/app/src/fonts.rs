//! Barlow Semi Condensed (SIL OFL 1.1, `assets/fonts/OFL.txt`), compiled into
//! the executable. It stands in for Bahnschrift SemiCondensed, which stock
//! GPUI cannot select (docs/GPUI_MIGRATION.md, phase 2a fonts).

use std::borrow::Cow;

use gpui::App;

/// DirectWrite files these faces under the family "Barlow" with a
/// semi-condensed width; GPUI matches the family name only, and the width
/// follows because these are the only Barlow faces it knows.
pub const OVERLAY_FAMILY: &str = "Barlow";

const FACES: [&[u8]; 4] = [
    include_bytes!("../assets/fonts/BarlowSemiCondensed-Regular.ttf"),
    include_bytes!("../assets/fonts/BarlowSemiCondensed-Medium.ttf"),
    include_bytes!("../assets/fonts/BarlowSemiCondensed-SemiBold.ttf"),
    include_bytes!("../assets/fonts/BarlowSemiCondensed-Bold.ttf"),
];

pub fn register(cx: &App) -> Result<(), String> {
    cx.text_system()
        .add_fonts(FACES.iter().map(|face| Cow::Borrowed(*face)).collect())
        .map_err(|error| format!("cannot load the embedded overlay font: {error:#}"))
}
