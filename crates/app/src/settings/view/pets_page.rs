//! Settings → Pets (docs/gpui-mockups/approved/settings.html, tokens.md
//! "Settings → Pets"): which soul gains show the soul tracker card, a test
//! card, and when the game last sent the collection of the server being
//! played.

use gpui::{HighlightStyle, StyledText};

use super::*;
use crate::clock::LocalZone;
use crate::game_data;
use crate::pets;

/// What the Pets page shows of the collection, to redraw only when it
/// changes: the server being played, when its collection was read and how
/// many of its pets are owned.
#[derive(Clone, Debug, Default, PartialEq)]
pub(super) struct PetsSeen {
    pub(super) server: Option<u32>,
    pub(super) read_at: Option<f64>,
    pub(super) owned: usize,
}

impl PetsSeen {
    pub(super) fn of(cx: &App) -> Self {
        let pets = pets::global(cx).read(cx);
        let collection = pets.collection();
        Self {
            server: pets.current_server(),
            read_at: collection.and_then(|collection| collection.snapshot_at),
            owned: collection.map_or(0, |collection| collection.pets.values().filter(|pet| pet.level >= 1).count()),
        }
    }
}

impl SettingsView {
    pub(super) fn pets_page(&mut self, settings: &AppSettings, window: &Window, cx: &mut Context<Self>) -> Div {
        let pets = &settings.pets;
        let on = pets.soul_tracker;
        let master = self.switch_row(
            "pets-tracker",
            fl!("settings-pets-tracker"),
            Some(fl!("settings-pets-tracker-text")),
            on,
            |all, on| all.pets.soul_tracker = on,
            window,
            cx,
        );
        let switches: [(&'static str, String, String, bool, fn(&mut AppSettings, bool)); 3] = [
            (
                "pets-only-pinned",
                fl!("settings-pets-only-pinned"),
                fl!("settings-pets-only-pinned-text"),
                pets.only_pinned,
                |all, on| all.pets.only_pinned = on,
            ),
            ("pets-maxed", fl!("settings-pets-maxed"), fl!("settings-pets-maxed-text"), pets.maxed_pets, |all, on| all.pets.maxed_pets = on),
            (
                "pets-black-market",
                fl!("settings-pets-black-market"),
                fl!("settings-pets-black-market-text"),
                pets.black_market,
                |all, on| all.pets.black_market = on,
            ),
        ];
        let mut rows = vec![master];
        for (key, label, text, value, apply) in switches {
            let focus = self.focus_of(key, true, window, cx);
            let toggle = on.then(|| self.set(apply));
            rows.push(reminder_row(on, label.clone(), text, switch(key, label, value, &focus, toggle)));
        }
        let test = self.test_pet_card_control(on, window, cx);
        rows.push(reminder_row(on, fl!("pets-card-test"), fl!("settings-pets-test-text"), test));

        div()
            .flex()
            .flex_col()
            .child(page_head(fl!("settings-section-pets"), Some(fl!("settings-pets-lead").into())))
            .child(group_title(fl!("settings-pets-card"), true))
            .child(group(rows))
            .child(group_title(fl!("settings-pets-collection"), false))
            .child(group(vec![self.pet_collection_row()]))
    }

    /// "Show a test card", and the badge saying where it went, which
    /// Settings may be covering: hung to the button's left without taking
    /// room, as the test reminder's.
    fn test_pet_card_control(&mut self, enabled: bool, window: &Window, cx: &mut Context<Self>) -> Div {
        let focus = self.focus_of("test-pet-card", true, window, cx);
        let press = enabled.then(|| self.button_handler(cx, |this, _, cx| this.show_test_pet_card(cx)));
        let shown = self.pet_test_shown.is_some().then(|| {
            let text = if toolbar::meter_is_open(cx) && !toolbar::meter_is_aside(cx) {
                fl!("settings-pets-test-shown-meter")
            } else {
                fl!("settings-pets-test-shown-toolbar")
            };
            let note = badge(Some(icon(Icon::Check, 12., rgb(widgets::GOOD)).into_any_element()), text.clone(), Tone::Good)
                .id("pet-test-shown")
                .debug_selector(|| "pet-test-shown".to_string())
                .role(Role::Status)
                .aria_label(text);
            let note = if cx.reduce_motion() {
                note.into_any_element()
            } else {
                note.with_animation("pet-test-shown-fade", Animation::new(CHIP_FADE), |note, t| note.opacity(t)).into_any_element()
            };
            div().absolute().top_0().bottom_0().right(relative(1.)).pr(px(10.)).flex().items_center().child(note)
        });
        div().relative().flex().items_center().children(shown).child(button(
            "test-pet-card",
            fl!("settings-pets-show-test"),
            Some(Icon::Play),
            ButtonKind::Secondary,
            false,
            &focus,
            press,
        ))
    }

    fn show_test_pet_card(&mut self, cx: &mut Context<Self>) {
        pets::global(cx).update(cx, |pets, cx| pets.show_test_card(cx));
        self.pet_test_shown = Some(cx.spawn(async move |this, cx| {
            cx.background_executor().timer(TEST_SHOWN_FOR).await;
            let _ = this.update(cx, |this, cx| {
                this.pet_test_shown = None;
                cx.notify();
            });
        }));
        cx.notify();
    }

    /// "Pets on Ossia", when the game sent them and how many are owned; or
    /// how to have them read.
    fn pet_collection_row(&self) -> Div {
        let seen = &self.pets_seen;
        let label = match seen.server {
            Some(server_id) => fl!("settings-pets-collection-on", server = game_data::server_name(&server_id.to_string())),
            None => fl!("settings-pets-collection-no-server"),
        };
        let quiet = |text: String| div().font_family(DISPLAY_FONT).text_size(px(13.5)).font_weight(FontWeight::SEMIBOLD).text_color(rgba(FAINT)).child(text);
        let Some(read_at) = seen.read_at else {
            let text = if seen.server.is_some() { fl!("settings-pets-not-read") } else { fl!("settings-pets-not-read-any") };
            return row(label, Some(description(text)), quiet("\u{2014}".to_string()).debug_selector(|| "pets-owned".to_string()));
        };
        let stamp = i18n::day_stamp(&LocalZone, (read_at * 1000.0) as i64);
        let text = fl!("settings-pets-read", stamp = stamp.as_str());
        let bright = HighlightStyle { color: Some(rgba(TEXT).into()), font_weight: Some(FontWeight::SEMIBOLD), ..Default::default() };
        let highlights: Vec<_> = text.find(stamp.as_str()).map(|start| (start..start + stamp.len(), bright)).into_iter().collect();
        let read = div()
            .max_w(px(400.))
            .text_size(px(12.))
            .line_height(relative(1.4))
            .text_color(rgba(MUTED))
            .child(StyledText::new(text).with_highlights(highlights))
            .into_any_element();
        let owned = div()
            .debug_selector(|| "pets-owned".to_string())
            .flex()
            .items_baseline()
            .gap(px(2.))
            .font_family(DISPLAY_FONT)
            .text_size(px(13.5))
            .font_weight(FontWeight::SEMIBOLD)
            .text_color(rgba(MUTED))
            .child(seen.owned.to_string())
            .child(
                div()
                    .text_size(px(12.))
                    .font_weight(FontWeight::MEDIUM)
                    .text_color(rgba(FAINT))
                    .child(fl!("settings-pets-owned-of", total = game_data::pet_ids().len())),
            );
        row(label, Some(read), owned)
    }

    #[cfg(any(test, feature = "probes"))]
    pub fn pet_test_note_is_shown(&self) -> bool {
        self.pet_test_shown.is_some()
    }
}
