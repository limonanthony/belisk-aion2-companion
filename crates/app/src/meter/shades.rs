//! Players of the same class told apart by shade (tokens.md, class shades).
//!
//! The first player of a class seen since the last reset keeps the class
//! colour; the next ones take lightness steps in a fixed order, then the
//! order starts over. A player keeps their shade until the meter is reset,
//! whatever happens to the ranking.

use std::collections::HashMap;

use crate::game_data::PlayerClass;

/// HSL lightness per step.
const STEP: f32 = 0.10;

/// The steps after the class colour, in the order players get them. A lighter
/// step is used only where white figures on the bar's bright end keep 4.5:1
/// over a dark scene; cleric and ranger are bright already, so they only
/// darken, and only the assassin's deep violet takes two lighter steps.
/// Chanter and fighter skip −2, where neither badge ink reaches 4.5:1.
fn steps(class: PlayerClass) -> &'static [i8] {
    match class {
        PlayerClass::Assassin => &[1, -1, 2, -2],
        PlayerClass::Cleric | PlayerClass::Ranger => &[-1, -2, -3],
        PlayerClass::Chanter | PlayerClass::Fighter => &[1, -1, -3, -4],
        _ => &[1, -1, -2, -3],
    }
}

/// The class colour for the `order`-th player of that class, as `0xRRGGBB`.
pub fn shade(class: PlayerClass, order: u32) -> u32 {
    let steps = steps(class);
    match order as usize % (steps.len() + 1) {
        0 => class.rgb(),
        n => lighten(class.rgb(), f32::from(steps[n - 1]) * STEP),
    }
}

/// Which shade each player has, keyed by name (ids change between zones) or
/// by id while the game has not named them.
#[derive(Default)]
pub struct ShadeBook {
    by_name: HashMap<String, u32>,
    by_id: HashMap<u32, u32>,
    seen_per_class: [u32; 9],
}

impl ShadeBook {
    /// The player's colour; the first call for a player fixes their shade.
    pub fn colour(&mut self, class: PlayerClass, actor_id: u32, actor_name: &str) -> u32 {
        let seen = &mut self.seen_per_class[class as usize];
        let next = || {
            let order = *seen;
            *seen += 1;
            order
        };
        let order = if actor_name.is_empty() {
            *self.by_id.entry(actor_id).or_insert_with(next)
        } else if let Some(&order) = self.by_name.get(actor_name) {
            order
        } else {
            // Named now: keep the shade they had while known only by id.
            let order = self.by_id.get(&actor_id).copied().unwrap_or_else(next);
            self.by_name.insert(actor_name.to_string(), order);
            order
        };
        shade(class, order)
    }

    /// The colour [`Self::colour`] gave the player, without giving one to a
    /// player the meter has not drawn.
    pub fn known(&self, class: PlayerClass, actor_id: u32, actor_name: &str) -> Option<u32> {
        let by_name = (!actor_name.is_empty()).then(|| self.by_name.get(actor_name)).flatten();
        by_name.or_else(|| self.by_id.get(&actor_id)).map(|&order| shade(class, order))
    }

    pub fn clear(&mut self) {
        self.by_name.clear();
        self.by_id.clear();
        self.seen_per_class = [0; 9];
    }
}

/// Text colour for a badge filled with `fill`: the dark ink the badges use,
/// or cream on the darker shades where the ink loses contrast.
pub fn ink_on(fill: u32) -> u32 {
    const INK: u32 = 0x17120d;
    const CREAM: u32 = 0xfff6ec;
    if contrast(fill, INK) >= contrast(fill, CREAM) { INK } else { CREAM }
}

fn contrast(a: u32, b: u32) -> f32 {
    let (a, b) = (luminance(a), luminance(b));
    (a.max(b) + 0.05) / (a.min(b) + 0.05)
}

fn luminance(rgb: u32) -> f32 {
    let linear = |shift: u32| {
        let channel = ((rgb >> shift) & 0xff) as f32 / 255.0;
        if channel <= 0.04045 { channel / 12.92 } else { ((channel + 0.055) / 1.055).powf(2.4) }
    };
    0.2126 * linear(16) + 0.7152 * linear(8) + 0.0722 * linear(0)
}

/// Moves `rgb`'s HSL lightness by `delta`, keeping hue and saturation.
fn lighten(rgb: u32, delta: f32) -> u32 {
    let channel = |shift: u32| ((rgb >> shift) & 0xff) as f32 / 255.0;
    let (r, g, b) = (channel(16), channel(8), channel(0));
    let (max, min) = (r.max(g).max(b), r.min(g).min(b));
    let lightness = (max + min) / 2.0;
    let chroma = max - min;
    let saturation = if chroma == 0.0 { 0.0 } else { chroma / (1.0 - (2.0 * lightness - 1.0).abs()) };
    let hue = if chroma == 0.0 {
        0.0
    } else if max == r {
        ((g - b) / chroma).rem_euclid(6.0)
    } else if max == g {
        (b - r) / chroma + 2.0
    } else {
        (r - g) / chroma + 4.0
    };

    let lightness = (lightness + delta).clamp(0.0, 1.0);
    let chroma = (1.0 - (2.0 * lightness - 1.0).abs()) * saturation;
    let x = chroma * (1.0 - (hue.rem_euclid(2.0) - 1.0).abs());
    let (r, g, b) = match hue as u32 {
        0 => (chroma, x, 0.0),
        1 => (x, chroma, 0.0),
        2 => (0.0, chroma, x),
        3 => (0.0, x, chroma),
        4 => (x, 0.0, chroma),
        _ => (chroma, 0.0, x),
    };
    let m = lightness - chroma / 2.0;
    let byte = |value: f32| ((value + m).clamp(0.0, 1.0) * 255.0).round() as u32;
    (byte(r) << 16) | (byte(g) << 8) | byte(b)
}

#[cfg(test)]
mod tests {
    use super::*;

    const ALL: [PlayerClass; 9] = [
        PlayerClass::Assassin,
        PlayerClass::Chanter,
        PlayerClass::Cleric,
        PlayerClass::Elementalist,
        PlayerClass::Fighter,
        PlayerClass::Gladiator,
        PlayerClass::Ranger,
        PlayerClass::Sorcerer,
        PlayerClass::Templar,
    ];

    #[test]
    fn the_first_player_of_a_class_keeps_the_class_colour() {
        let mut book = ShadeBook::default();
        assert_eq!(book.colour(PlayerClass::Ranger, 1, "A"), PlayerClass::Ranger.rgb());
        assert_eq!(book.colour(PlayerClass::Cleric, 2, "B"), PlayerClass::Cleric.rgb());
    }

    #[test]
    fn players_of_one_class_get_distinct_shades_until_the_order_starts_over() {
        for class in ALL {
            let mut book = ShadeBook::default();
            let count = steps(class).len() as u32 + 1;
            let colours: Vec<u32> = (0..count).map(|id| book.colour(class, id, &format!("P{id}"))).collect();
            for (index, colour) in colours.iter().enumerate() {
                assert!(!colours[..index].contains(colour), "{class:?} repeats a shade");
            }
            assert_eq!(book.colour(class, 99, "Next"), class.rgb(), "{class:?} starts over");
        }
    }

    #[test]
    fn a_shade_follows_the_player_not_the_rank() {
        let mut book = ShadeBook::default();
        let first = book.colour(PlayerClass::Ranger, 1, "First");
        let second = book.colour(PlayerClass::Ranger, 2, "Second");
        assert_ne!(first, second);
        assert_eq!(book.colour(PlayerClass::Ranger, 2, "Second"), second, "asked first after a rank swap");
        assert_eq!(book.colour(PlayerClass::Ranger, 1, "First"), first);
        assert_eq!(book.colour(PlayerClass::Ranger, 77, "Second"), second, "a new id after a zone change");
    }

    #[test]
    fn known_reports_drawn_players_only_and_assigns_nothing() {
        let mut book = ShadeBook::default();
        let first = book.colour(PlayerClass::Templar, 9, "First");
        let unnamed = book.colour(PlayerClass::Templar, 10, "");
        assert_eq!(book.known(PlayerClass::Templar, 9, "First"), Some(first));
        assert_eq!(book.known(PlayerClass::Templar, 10, ""), Some(unnamed));
        assert_eq!(book.known(PlayerClass::Templar, 11, "Other"), None);
        assert_eq!(book.colour(PlayerClass::Templar, 11, "Other"), shade(PlayerClass::Templar, 2));
    }

    #[test]
    fn a_player_named_later_keeps_the_shade_they_had_by_id() {
        let mut book = ShadeBook::default();
        book.colour(PlayerClass::Templar, 9, "First");
        let unnamed = book.colour(PlayerClass::Templar, 10, "");
        assert_eq!(book.colour(PlayerClass::Templar, 10, "A"), unnamed);
        assert_eq!(book.colour(PlayerClass::Templar, 11, "B"), shade(PlayerClass::Templar, 2), "no order was used up");
    }

    #[test]
    fn unnamed_players_are_told_apart_by_id() {
        let mut book = ShadeBook::default();
        let a = book.colour(PlayerClass::Templar, 10, "");
        let b = book.colour(PlayerClass::Templar, 11, "");
        assert_ne!(a, b);
        assert_eq!(book.colour(PlayerClass::Templar, 11, ""), b);
    }

    #[test]
    fn classes_count_their_own_players() {
        let mut book = ShadeBook::default();
        book.colour(PlayerClass::Ranger, 1, "A");
        assert_eq!(book.colour(PlayerClass::Cleric, 2, "B"), PlayerClass::Cleric.rgb());
    }

    #[test]
    fn a_reset_clears_the_shades() {
        let mut book = ShadeBook::default();
        book.colour(PlayerClass::Ranger, 1, "A");
        let second = book.colour(PlayerClass::Ranger, 2, "B");
        book.clear();
        assert_eq!(book.colour(PlayerClass::Ranger, 2, "B"), PlayerClass::Ranger.rgb());
        assert_ne!(book.colour(PlayerClass::Ranger, 1, "A"), PlayerClass::Ranger.rgb());
        assert_ne!(second, PlayerClass::Ranger.rgb());
    }

    #[test]
    fn white_figures_keep_four_and_a_half_to_one_on_every_added_shade() {
        fn linear(channel: f32) -> f32 {
            if channel <= 0.04045 { channel / 12.92 } else { ((channel + 0.055) / 1.055).powf(2.4) }
        }
        fn luminance([r, g, b]: [f32; 3]) -> f32 {
            0.2126 * linear(r) + 0.7152 * linear(g) + 0.0722 * linear(b)
        }
        fn over(top: [f32; 3], alpha: f32, under: [f32; 3]) -> [f32; 3] {
            [0, 1, 2].map(|i| top[i] * alpha + under[i] * (1.0 - alpha))
        }
        let rgb = |value: u32| [16, 8, 0].map(|shift| ((value >> shift) & 0xff) as f32 / 255.0);
        // A dark scene under the card (alpha 150), the track, then the bar's
        // bright end at 0.5, with the figures' text colour over it.
        let card = over(rgb(0x100e0c), 150.0 / 255.0, rgb(0x141418));
        let track = over([0.0; 3], 0.34, card);
        // The class colours themselves are the approved palette (the ranger
        // sits right at 4.5:1); the shades added here must not go below it.
        for class in ALL {
            for order in 1..=steps(class).len() as u32 {
                let end = over(rgb(shade(class, order)), 128.0 / 255.0, track);
                let text = over(rgb(0xfff6ec), 0.96, end);
                let ratio = (luminance(text) + 0.05) / (luminance(end) + 0.05);
                assert!(ratio >= 4.5, "{class:?} shade {order}: {ratio:.2}:1");
            }
        }
    }

    #[test]
    fn badge_text_is_dark_on_bright_shades_and_cream_on_dark_ones() {
        assert_eq!(ink_on(PlayerClass::Ranger.rgb()), 0x17120d);
        assert_eq!(ink_on(shade(PlayerClass::Ranger, 3)), 0xfff6ec);
        for class in ALL {
            for order in 0..5 {
                let fill = shade(class, order);
                assert!(contrast(fill, ink_on(fill)) >= 4.5, "{class:?} shade {order}");
            }
        }
    }

    #[test]
    fn shades_match_the_approved_page() {
        assert_eq!(shade(PlayerClass::Ranger, 1), 0x8ace1a);
        assert_eq!(shade(PlayerClass::Ranger, 3), 0x4d740e);
        assert_eq!(shade(PlayerClass::Assassin, 1), 0xc186f9);
        assert_eq!(shade(PlayerClass::Templar, 4), 0x056792);
    }

    #[test]
    fn lightening_keeps_the_hue() {
        assert_eq!(lighten(0xa3e635, 0.0), 0xa3e635);
        let darker = lighten(0xa3e635, -0.1);
        assert!(darker & 0x00ff00 > (darker & 0xff0000) >> 8, "still green-led: {darker:06x}");
        assert!(lighten(0x808080, 0.1) > 0x808080);
    }
}
