//! Off-screen captures and checks of the Pets window and the soul tracker
//! card, run by `--capture --fixtures` after the toolbar's. The collection is
//! invented (`fixtures::pet_login`) and reaches the app as the game's
//! messages would. The window opens from the toolbar while the meter is
//! closed, and every window stays past every monitor; the search takes the
//! keyboard inside GPUI only, so nothing asks Windows for the foreground, and
//! the only input is mouse and key messages posted to our own windows. The
//! run fails when the window or the card could take focus, a row's pin
//! flashes while the pointer moves inside the row, the card does not stack
//! beyond a reminder card, or it catches clicks while the meter is locked.

use std::path::Path;
use std::time::Duration;

use gpui::{AsyncApp, Entity, WindowHandle};
use meter_core::pet_sources::SoulSource;
use windows::Win32::Foundation::{HWND, RECT};
use windows::Win32::UI::WindowsAndMessaging::{
    GetWindowRect, IsWindowVisible, WM_CHAR, WM_LBUTTONDOWN, WM_LBUTTONUP, WM_MOUSEMOVE, WS_EX_LAYERED, WS_EX_TRANSPARENT,
};

use crate::capture::{MK_LBUTTON, SETTLE};
use crate::capture_reminders::{self, shoot_together};
use crate::capture_secondary::{
    check_scrollbar, check_title, close, pack, pause, post, report_window, shoot_window, show_scene, window_handle, window_hwnd,
};
use crate::capture_todos::{device, key};
use crate::capture_toolbar;
use crate::clock::now_ms;
use crate::fixtures::{self, BLUE_KALGOLEM_MOB, PET_FOREST_SPIRIT, PET_FOSSA, PET_KALGOLEM, PET_SPARKIE};
use crate::game_data::PetFamily;
use crate::meter::{MeterModel, MeterView};
use crate::overlay::{self, off_screen};
use crate::pets::{self, PetCardView, PetsView, Tab};
use crate::reminders::EventId;
use crate::secondary::{self, WindowKind};
use crate::settings;
use crate::toolbar::{self, Slot, Tool};

type Result<T, E = String> = std::result::Result<T, E>;

const TICK: Duration = Duration::from_millis(150);
const MOVE_GAP: Duration = Duration::from_millis(60);
const VK_TAB: usize = 0x09;
const VK_ESCAPE: usize = 0x1B;
const VK_RETURN: usize = 0x0D;
const VK_SPACE: usize = 0x20;
const DAY_SECS: f64 = 86_400.0;

pub async fn from_fixtures(meter: WindowHandle<MeterView>, model: &Entity<MeterModel>, dir: &Path, cx: &mut AsyncApp) -> Result<()> {
    show_scene(model, fixtures::open_world(), cx);
    cx.update(toolbar::close_meter);
    let foreground_before = capture_reminders::foreground();
    record(fixtures::pet_unlock_elsewhere(1003, PET_FOSSA), cx);
    open_from_toolbar(cx).await?;
    report_window(WindowKind::Pets, cx)?;
    let view = window_handle::<PetsView>(WindowKind::Pets, cx)?;
    view.update(cx, |view, _, _| view.keep_keyboard_in_app()).map_err(gone)?;
    shot(dir, "pets-00-not-read", cx).await?;

    record(fixtures::pet_login(now_secs()), cx);
    let pets = cx.update(|cx| pets::global(cx));
    cx.update(|cx| {
        pets.update(cx, |pets, cx| {
            pets.pin(PET_FOSSA, cx);
            pets.pin(PET_KALGOLEM, cx);
        })
    });
    shot(dir, "pets-01-all", cx).await?;
    capture_toolbar::shot(dir, "toolbar-16-pets-open", cx).await?;
    check_title(WindowKind::Pets, "all", cx).await?;
    check_scrollbar(WindowKind::Pets, |cx| view.update(cx, |view, _, _| view.list_reading()).map_err(gone), cx).await?;

    view.update(cx, |view, _, cx| view.force_pointer_row(Some(PET_SPARKIE), cx)).map_err(gone)?;
    shot(dir, "pets-02-pointer-on-row", cx).await?;
    view.update(cx, |view, _, cx| view.force_pointer_row(None, cx)).map_err(gone)?;
    hover_inside_a_row(&view, cx).await?;
    pin_under_the_pointer(&view, &pets, cx).await?;

    view.update(cx, |view, _, cx| view.show_tab(Tab::Family(PetFamily::Fera), cx)).map_err(gone)?;
    shot(dir, "pets-03-fera", cx).await?;
    view.update(cx, |view, _, cx| view.show_menu(cx)).map_err(gone)?;
    shot(dir, "pets-04-menu", cx).await?;
    view.update(cx, |view, window, cx| view.close_the_menu(window, cx)).map_err(gone)?;
    set_choice(&pets, true, cx);
    shot(dir, "pets-05-not-at-lv3-closest-first", cx).await?;
    set_choice(&pets, false, cx);
    view.update(cx, |view, _, cx| view.show_tab(Tab::All, cx)).map_err(gone)?;

    search(&view, dir, cx).await?;
    keyboard(&view, &pets, dir, cx).await?;

    record(fixtures::pet_login(now_secs() - DAY_SECS), cx);
    shot(dir, "pets-09-earlier-login", cx).await?;
    record(fixtures::pet_login(now_secs()), cx);
    cx.update(|cx| settings::global(cx).update(cx, |settings, cx| settings.update(cx, |all| all.overlay.content_scale = 1.25)));
    shot(dir, "pets-10-scale-125", cx).await?;
    cx.update(|cx| settings::global(cx).update(cx, |settings, cx| settings.update(cx, |all| all.overlay.content_scale = 1.0)));

    println!("the foreground window during the run: {:?} at the start", foreground_before.0);
    capture_reminders::check_never_focused(WindowKind::Pets, "the Pets window after the run", capture_reminders::foreground(), cx)?;
    close(WindowKind::Pets, cx).await?;
    println!("pets: closed, the registry let go of it");

    card(meter, &pets, dir, cx).await
}

/// The real entry point: a click on Pets on the toolbar, with the meter
/// closed. Opening it again makes no second window.
async fn open_from_toolbar(cx: &mut AsyncApp) -> Result<()> {
    capture_toolbar::click(Slot::Tool(Tool::Pets), cx).await?;
    let open = cx.update(|cx| secondary::window_of(WindowKind::Pets, cx).is_some());
    let meter_open = cx.update(|cx| toolbar::meter_is_open(cx));
    println!("entry point: click on Pets on the toolbar with the meter open {meter_open}: Pets window open {open}");
    if !open {
        return Err("the Pets window did not open from the toolbar".to_string());
    }
    let before = cx.update(|cx| cx.windows().len());
    cx.update(secondary::open_pets);
    pause(cx, TICK).await;
    let after = cx.update(|cx| cx.windows().len());
    println!("single instance: the Pets window opened twice, {before} windows before the second open, {after} after");
    if before != after {
        return Err("a second open made a second Pets window".to_string());
    }
    Ok(())
}

/// The pointer moves about inside one row: its pin shows from the first move
/// to the last and never goes. Then it leaves for the header, and it goes.
async fn hover_inside_a_row(view: &WindowHandle<PetsView>, cx: &mut AsyncApp) -> Result<()> {
    let hwnd = window_hwnd(WindowKind::Pets, cx)?;
    let (bounds, scale) = view
        .update(cx, |view, window, _| view.row_bounds(PET_SPARKIE).map(|bounds| (bounds, window.scale_factor())))
        .map_err(gone)?
        .ok_or("the Sparkie row is not drawn")?;
    let y = f32::from(bounds.center().y);
    let points: Vec<(i32, i32)> = [0.05, 0.3, 0.6, 0.8, 0.97, 0.5, 0.1]
        .iter()
        .map(|fraction| device(f32::from(bounds.left()) + f32::from(bounds.size.width) * fraction, y + (fraction - 0.5) * 6., scale))
        .collect();
    let mut seen = Vec::new();
    for point in &points {
        post(hwnd, WM_MOUSEMOVE, 0, pack(*point));
        pause(cx, MOVE_GAP).await;
        seen.push(view.update(cx, |view, _, _| view.revealed_row()).map_err(gone)?);
    }
    let shown = seen.iter().filter(|row| **row == Some(PET_SPARKIE)).count();
    println!("hover: {} moves inside the Sparkie row: pin shown after {shown} of them ({seen:?})", points.len());
    if shown != points.len() {
        return Err("hover: the row's pin did not stay while the pointer moved inside the row".to_string());
    }
    post(hwnd, WM_MOUSEMOVE, 0, pack(device(60., 14., scale)));
    pause(cx, MOVE_GAP).await;
    let after = view.update(cx, |view, _, _| view.revealed_row()).map_err(gone)?;
    println!("hover: the pointer left for the header: pin on {after:?}");
    if after.is_some() {
        return Err("hover: the pin stayed after the pointer left the row".to_string());
    }
    Ok(())
}

/// A click on a row's pin pins the pet, which goes up among the pinned; the
/// row that slides under the pointer keeps its pin hidden until the pointer
/// moves.
async fn pin_under_the_pointer(view: &WindowHandle<PetsView>, pets: &Entity<pets::Pets>, cx: &mut AsyncApp) -> Result<()> {
    let hwnd = window_hwnd(WindowKind::Pets, cx)?;
    let (bounds, scale) = view
        .update(cx, |view, window, _| view.row_bounds(PET_SPARKIE).map(|bounds| (bounds, window.scale_factor())))
        .map_err(gone)?
        .ok_or("the Sparkie row is not drawn")?;
    let at = device(f32::from(bounds.right()) - 10., f32::from(bounds.center().y), scale);
    post(hwnd, WM_MOUSEMOVE, 0, pack(at));
    pause(cx, MOVE_GAP).await;
    post(hwnd, WM_LBUTTONDOWN, MK_LBUTTON, pack(at));
    post(hwnd, WM_LBUTTONUP, 0, pack(at));
    pause(cx, TICK).await;
    let pinned = cx.update(|cx| pets.read(cx).is_pinned(PET_SPARKIE));
    let (held, revealed) = view.update(cx, |view, _, _| (view.held_still(), view.revealed_row())).map_err(gone)?;
    println!("pin by pointer: a click on Sparkie's pin: pinned {pinned}; the row now under the pointer shows its pin for {revealed:?}, held until the pointer moves {held}");
    if !pinned {
        return Err("pin by pointer: the click on the pin did not pin the pet".to_string());
    }
    if revealed.is_some() {
        return Err("pin by pointer: the row that slid under the pointer showed its pin at once".to_string());
    }
    post(hwnd, WM_MOUSEMOVE, 0, pack((at.0 - 4, at.1)));
    pause(cx, MOVE_GAP).await;
    let revealed = view.update(cx, |view, _, _| view.revealed_row()).map_err(gone)?;
    println!("pin by pointer: after the pointer moved: pin shown for {revealed:?}");
    if revealed.is_none() {
        return Err("pin by pointer: the row under the pointer did not show its pin once the pointer moved".to_string());
    }
    cx.update(|cx| pets.update(cx, |pets, cx| pets.unpin(PET_SPARKIE, cx)));
    post(hwnd, WM_MOUSEMOVE, 0, pack(device(60., 14., scale)));
    pause(cx, MOVE_GAP).await;
    Ok(())
}

/// "kal" typed in the search, the same search in Fera where nothing
/// matches, then Escape clears it and a second Escape gives the keyboard
/// back.
async fn search(view: &WindowHandle<PetsView>, dir: &Path, cx: &mut AsyncApp) -> Result<()> {
    let hwnd = window_hwnd(WindowKind::Pets, cx)?;
    view.update(cx, |view, window, cx| view.begin_search(window, cx)).map_err(gone)?;
    pause(cx, TICK).await;
    for letter in "kal".chars() {
        post(hwnd, WM_CHAR, letter as usize, 0);
    }
    pause(cx, TICK).await;
    let typing = view.update(cx, |view, _, _| view.is_typing()).map_err(gone)?;
    println!("search: \"kal\" typed: typing {typing}");
    shot(dir, "pets-06-search", cx).await?;
    view.update(cx, |view, _, cx| view.show_tab(Tab::Family(PetFamily::Fera), cx)).map_err(gone)?;
    shot(dir, "pets-07-no-match-in-fera", cx).await?;
    key(hwnd, VK_ESCAPE, cx).await;
    key(hwnd, VK_ESCAPE, cx).await;
    let typing = view.update(cx, |view, _, _| view.is_typing()).map_err(gone)?;
    println!("search: Escape twice: still typing {typing}");
    if typing {
        return Err("search: a second Escape did not give the keyboard back".to_string());
    }
    view.update(cx, |view, _, cx| view.show_tab(Tab::All, cx)).map_err(gone)
}

/// Tab reaches a pin, which shows with its ring; Space then pins or unpins
/// the pet, and Enter puts it back.
async fn keyboard(view: &WindowHandle<PetsView>, pets: &Entity<pets::Pets>, dir: &Path, cx: &mut AsyncApp) -> Result<()> {
    let hwnd = window_hwnd(WindowKind::Pets, cx)?;
    let mut reached = None;
    for _ in 0..20 {
        key(hwnd, VK_TAB, cx).await;
        let focused = view.update(cx, |view, window, _| view.focused_control(window)).ok().flatten();
        if let Some(pin) = focused.filter(|key| key.starts_with("pin:")) {
            reached = Some(pin);
            break;
        }
    }
    let pin = reached.ok_or("keyboard: Tab never reached a pin")?;
    println!("keyboard: Tab reached {pin}");
    shot(dir, "pets-08-focus-on-pin", cx).await?;
    let pet_id: u32 = pin.trim_start_matches("pin:").parse().map_err(|error| format!("keyboard: {pin}: {error}"))?;
    let before = cx.update(|cx| pets.read(cx).is_pinned(pet_id));
    for (vk, name, expected) in [(VK_SPACE, "Space", !before), (VK_RETURN, "Enter", before)] {
        key(hwnd, vk, cx).await;
        let pinned = cx.update(|cx| pets.read(cx).is_pinned(pet_id));
        println!("keyboard: {name} on {pin}: pinned {pinned} (expected {expected})");
        if pinned != expected {
            return Err(format!("keyboard: {name} on a pin did not toggle it once"));
        }
    }
    Ok(())
}

fn set_choice(pets: &Entity<pets::Pets>, filtering: bool, cx: &mut AsyncApp) {
    use crate::pets::{ListChoice, Order, Show};
    let choice = if filtering { ListChoice { show: Show::NotMaxed, order: Order::ClosestToNextLevel } } else { ListChoice::default() };
    cx.update(|cx| pets.update(cx, |pets, cx| pets.set_list_choice(choice, cx)));
}

/// The card's states: a gain above the meter, three pets with a level
/// reached and an unlock, a pet at Lv 3, the line drained, beyond a reminder
/// card, under the toolbar with the meter closed, locked, and the test card.
async fn card(meter: WindowHandle<MeterView>, pets: &Entity<pets::Pets>, dir: &Path, cx: &mut AsyncApp) -> Result<()> {
    cx.update(toolbar::open_meter);
    pause(cx, TICK).await;
    let meter_hwnd = meter.update(cx, |_, window, _| overlay::hwnd(window)).map_err(|error| format!("overlay window gone: {error:#}"))??;
    let foreground_before = capture_reminders::foreground();

    record(fixtures::pet_souls(SoulSource::Loot, &[(PET_FOSSA, 1)]), cx);
    check_card("a gain", foreground_before, cx).await?;
    shoot_card(&[meter_hwnd], dir, "pet-card-01-gain", cx).await?;

    record(fixtures::pet_souls(SoulSource::Loot, &[(PET_SPARKIE, 1)]), cx);
    record(fixtures::pet_souls(SoulSource::BlackMarket, &[(PET_FOREST_SPIRIT, 1)]), cx);
    let shown = card_view(cx)?.update(cx, |view, _, _| view.shown().pets.len()).map_err(gone)?;
    println!("card: three gains: {shown} pets on one card");
    if shown != 3 {
        return Err(format!("the card shows {shown} pets after gains of three"));
    }
    shoot_card(&[meter_hwnd], dir, "pet-card-02-three", cx).await?;
    close(WindowKind::PetCard, cx).await?;

    record(fixtures::pet_maxed_loot(BLUE_KALGOLEM_MOB), cx);
    check_card("a pet at Lv 3", foreground_before, cx).await?;
    shoot_card(&[meter_hwnd], dir, "pet-card-03-at-lv3", cx).await?;
    card_view(cx)?.update(cx, |view, _, cx| view.force_time_left(0.35, cx)).map_err(gone)?;
    shoot_card(&[meter_hwnd], dir, "pet-card-04-drained", cx).await?;

    let card = card_view(cx)?;
    card.update(cx, |view, window, cx| view.force_pointer(Some(true), window, cx)).map_err(gone)?;
    let left_before = card.update(cx, |view, _, cx| view.time_left(cx)).map_err(gone)?;
    pause(cx, Duration::from_millis(600)).await;
    let left_after = card.update(cx, |view, _, cx| view.time_left(cx)).map_err(gone)?;
    println!("pointer over the card: {left_before:?} left, {left_after:?} after 600 ms");
    if left_after != left_before {
        return Err("the card's time ran on under the pointer".to_string());
    }
    card.update(cx, |view, window, cx| view.force_pointer(None, window, cx)).map_err(gone)?;
    close(WindowKind::PetCard, cx).await?;

    capture_reminders::show(vec![EventId::ShugoFestival], (16, 0), cx);
    pause(cx, TICK).await;
    record(fixtures::pet_souls(SoulSource::Loot, &[(PET_FOSSA, 1)]), cx);
    check_card("with a reminder card up", foreground_before, cx).await?;
    check_beyond_reminder(cx)?;
    let reminder_hwnd = window_hwnd(WindowKind::Reminder, cx)?;
    shoot_card(&[meter_hwnd, reminder_hwnd], dir, "pet-card-05-beyond-a-reminder", cx).await?;
    close(WindowKind::Reminder, cx).await?;
    close(WindowKind::PetCard, cx).await?;

    cx.update(toolbar::close_meter);
    pause(cx, TICK).await;
    record(fixtures::pet_souls(SoulSource::Loot, &[(PET_FOSSA, 1)]), cx);
    check_card("the meter closed", foreground_before, cx).await?;
    let (_, toolbar_hwnd) = capture_toolbar::toolbar_window(cx)?;
    let (bar, card_rect) = (window_rect(toolbar_hwnd)?, window_rect(window_hwnd(WindowKind::PetCard, cx)?)?);
    println!("card under the toolbar: left edges {} and {}, gap {} px", bar.left, card_rect.left, card_rect.top - bar.bottom);
    if card_rect.left != bar.left || card_rect.top <= bar.bottom {
        return Err("the soul tracker card is not under the toolbar while the meter is closed".to_string());
    }
    shoot_card(&[toolbar_hwnd], dir, "pet-card-06-meter-closed", cx).await?;
    close(WindowKind::PetCard, cx).await?;
    cx.update(toolbar::open_meter);
    pause(cx, TICK).await;

    meter.update(cx, |view, window, cx| view.set_locked(true, window, cx)).map_err(|error| format!("overlay window gone: {error:#}"))?;
    record(fixtures::pet_souls(SoulSource::Loot, &[(PET_FOSSA, 1)]), cx);
    pause(cx, TICK * 2).await;
    let card_hwnd = window_hwnd(WindowKind::PetCard, cx)?;
    let style = overlay::ex_style(card_hwnd);
    let click_through = style & WS_EX_TRANSPARENT.0 != 0 && style & WS_EX_LAYERED.0 != 0;
    let without_dismiss = card_view(cx)?.update(cx, |view, _, _| view.is_locked()).map_err(gone)?;
    println!("meter locked: the card is click-through {click_through}, drawn without × {without_dismiss}");
    if !click_through || !without_dismiss {
        return Err("the soul tracker card does not follow the meter's lock".to_string());
    }
    shoot_card(&[meter_hwnd], dir, "pet-card-07-locked", cx).await?;
    meter.update(cx, |view, window, cx| view.set_locked(false, window, cx)).map_err(|error| format!("overlay window gone: {error:#}"))?;
    close(WindowKind::PetCard, cx).await?;

    cx.update(|cx| pets.update(cx, |pets, cx| pets.show_test_card(cx)));
    check_card("the test card", foreground_before, cx).await?;
    shoot_card(&[meter_hwnd], dir, "pet-card-08-test", cx).await?;

    card_view(cx)?.update(cx, |view, _, cx| view.force_time_left(0.01, cx)).map_err(gone)?;
    pause(cx, Duration::from_secs(1)).await;
    let open = cx.update(|cx| secondary::window_of(WindowKind::PetCard, cx).is_some());
    println!("time-out: still open 1 s after the ring ran out: {open}");
    if open {
        return Err("the soul tracker card stayed after its time ran out".to_string());
    }
    record(fixtures::pet_souls(SoulSource::Loot, &[(PET_SPARKIE, 1)]), cx);
    pause(cx, TICK).await;
    let visible = unsafe { IsWindowVisible(window_hwnd(WindowKind::PetCard, cx)?) }.as_bool();
    let pets_on = card_view(cx)?.update(cx, |view, _, _| view.shown().pets.len()).map_err(gone)?;
    println!("a gain after the card closed: a new card, visible {visible}, {pets_on} pet on it");
    if !visible || pets_on != 1 {
        return Err("a gain after the card closed did not open a new card with that pet alone".to_string());
    }
    close(WindowKind::PetCard, cx).await
}

async fn check_card(label: &str, foreground_before: HWND, cx: &mut AsyncApp) -> Result<()> {
    pause(cx, TICK * 2).await;
    capture_reminders::check_never_focused(WindowKind::PetCard, label, foreground_before, cx)
}

/// With a reminder card up, the pet card goes 6 px beyond it, on the side
/// the reminder took, left edges aligned.
fn check_beyond_reminder(cx: &mut AsyncApp) -> Result<()> {
    let reminder = window_rect(window_hwnd(WindowKind::Reminder, cx)?)?;
    let card = window_rect(window_hwnd(WindowKind::PetCard, cx)?)?;
    let overlaps = card.left < reminder.right && reminder.left < card.right && card.top < reminder.bottom && reminder.top < card.bottom;
    let gap = if card.bottom <= reminder.top { reminder.top - card.bottom } else { card.top - reminder.bottom };
    println!("card beyond the reminder: left edges {} and {}, gap {gap} px, overlap {overlaps}", card.left, reminder.left);
    if overlaps || card.left != reminder.left {
        return Err("the soul tracker card does not stack beyond the reminder card".to_string());
    }
    Ok(())
}

fn card_view(cx: &mut AsyncApp) -> Result<WindowHandle<PetCardView>> {
    window_handle::<PetCardView>(WindowKind::PetCard, cx)
}

/// The card with the windows it sits by, as they are on the (virtual)
/// screen.
async fn shoot_card(others: &[HWND], dir: &Path, name: &str, cx: &mut AsyncApp) -> Result<()> {
    pause(cx, SETTLE).await;
    let mut windows = others.to_vec();
    windows.push(window_hwnd(WindowKind::PetCard, cx)?);
    shoot_together(&windows, dir, name, cx).await
}

fn record(report: meter_core::pets::PetReport, cx: &mut AsyncApp) {
    cx.update(|cx| pets::record(cx, vec![report]));
}

fn now_secs() -> f64 {
    now_ms() as f64 / 1000.0
}

fn window_rect(hwnd: HWND) -> Result<RECT> {
    let mut rect = RECT::default();
    unsafe { GetWindowRect(hwnd, &mut rect) }.map_err(|error| format!("GetWindowRect failed: {error}"))?;
    Ok(rect)
}

async fn shot(dir: &Path, name: &str, cx: &mut AsyncApp) -> Result<()> {
    shoot_window(WindowKind::Pets, dir, name, cx).await?;
    let hwnd = window_hwnd(WindowKind::Pets, cx)?;
    println!("{name}: {}", off_screen::frame_report(hwnd));
    Ok(())
}

fn gone(error: impl std::fmt::Display) -> String {
    format!("a pets window is gone: {error:#}")
}
