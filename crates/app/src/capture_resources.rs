//! Off-screen captures of the values the game fills, run by `--capture
//! --fixtures` inside the to-do window's run: the to-do window before the
//! first login, from an earlier login, typed over, on a character not seen
//! at login, just logged in, live and this week's rows; then the Resources
//! window, opened from the toolbar's Odyle Energy count, live, typed over,
//! amounts typed before the game said any, full, and with the membership
//! off. The values are invented (`fixtures::MIRELLE_TICKETS`). Both windows
//! stay past every monitor and are checked never to take the focus.

use std::path::Path;
use std::time::Duration;

use gpui::{AsyncApp, Entity, WindowHandle};
use windows::Win32::UI::WindowsAndMessaging::{WM_CHAR, WM_MOUSEMOVE};

use crate::capture_reminders;
use crate::capture_secondary::{close, pack, pause, post, report_window, shoot_window, window_handle, window_hwnd};
use crate::capture_todos::{device, gone, identify, key, known, row_bounds, set_membership, set_reset_ahead, shot, until_field_closes};
use crate::clock::now_ms;
use crate::fixtures;
use crate::meter::MeterModel;
use crate::overlay::off_screen;
use crate::secondary::{self, WindowKind};
use crate::todos::{self, CharacterId, RowId, RowKind, Section, TodoView, Todos, Typing};
use crate::capture_toolbar;
use crate::toolbar::Slot;

type Result<T, E = String> = std::result::Result<T, E>;

const TICK: Duration = Duration::from_millis(150);
/// Past GPUI's tooltip delay.
const TOOLTIP_WAIT: Duration = Duration::from_millis(900);
/// Past the login note's 4 s.
const NOTE_GONE: Duration = Duration::from_millis(4_300);
const VK_RETURN: usize = 0x0D;
const VK_ESCAPE: usize = 0x1B;
const DAY_MS: i64 = 86_400_000;

pub async fn from_fixtures(
    model: &Entity<MeterModel>,
    view: &WindowHandle<TodoView>,
    [mirelle, tavrin, sefa]: [CharacterId; 3],
    dir: &Path,
    cx: &mut AsyncApp,
) -> Result<()> {
    let todos = cx.update(|cx| todos::global(cx));
    edit(&todos, cx, |todos, cx| {
        for section in Section::ALL {
            todos.set_section_folded(section, false, cx);
        }
        todos.select(sefa, cx);
    });
    shot(dir, "todos-20-game-before-first-login", cx).await?;

    identify("Tavrin", cx);
    login(&todos, "Mirelle", now_ms() - DAY_MS + 50 * 60_000, fixtures::MIRELLE_TICKETS, cx);
    edit(&todos, cx, |todos, cx| todos.select(mirelle, cx));
    pause(cx, TICK).await;
    shot(dir, "todos-21-game-earlier-login", cx).await?;
    tooltip_on(view, WindowKind::Todos, "trial", cx).await?;
    shot(dir, "todos-22-game-tooltip", cx).await?;
    pointer_away(view, WindowKind::Todos, cx).await?;

    edit(&todos, cx, |todos, cx| {
        todos.set_progress(mirelle, "trial", Some(1), cx);
    });
    shot(dir, "todos-23-game-typed-over", cx).await?;

    edit(&todos, cx, |todos, cx| todos.follow_the_game(cx));
    let shown = cx.update(|cx| todos.read(cx).selected());
    println!("game values: Tavrin in game, never seen at login: window on Tavrin {}", shown == Some(tavrin));
    shot(dir, "todos-24-game-not-seen-yet", cx).await?;

    identify("Mirelle", cx);
    pause(cx, TICK).await;
    login(&todos, "Mirelle", now_ms(), fixtures::MIRELLE_TICKETS, cx);
    pause(cx, TICK).await;
    let trial = count_of(&todos, mirelle, "trial", cx);
    println!("game values: Mirelle logs in: Ascension Trial {trial:?} done (the typed 1 replaced by the game's 2)");
    if trial != Some(2) {
        return Err("the login did not replace the count typed over the game's".to_string());
    }
    shot(dir, "todos-25-game-just-logged-in", cx).await?;
    pause(cx, NOTE_GONE).await;
    shot(dir, "todos-26-game-live", cx).await?;

    edit(&todos, cx, |todos, cx| todos.set_section_folded(Section::Today, true, cx));
    shot(dir, "todos-29-game-this-week", cx).await?;
    type_over_weekly(view, &todos, mirelle, dir, cx).await?;
    edit(&todos, cx, |todos, cx| todos.set_section_folded(Section::Today, false, cx));

    resources_window(model, &todos, [mirelle, sefa], dir, cx).await
}

async fn resources_window(model: &Entity<MeterModel>, todos: &Entity<Todos>, [mirelle, sefa]: [CharacterId; 2], dir: &Path, cx: &mut AsyncApp) -> Result<()> {
    let foreground_before = capture_reminders::foreground();
    open_from_toolbar(model, cx).await?;
    report_window(WindowKind::Resources, cx)?;
    let view = window_handle::<TodoView>(WindowKind::Resources, cx)?;
    view.update(cx, |view, _, _| view.keep_keyboard_in_app()).map_err(gone)?;
    resources_shot(dir, "resources-01-live", cx).await?;

    tooltip_on(&view, WindowKind::Resources, "shugo", cx).await?;
    resources_shot(dir, "resources-03-tooltip", cx).await?;
    pointer_away(&view, WindowKind::Resources, cx).await?;
    account_tooltip_on(&view, WindowKind::Resources, "shugo", cx).await?;
    resources_shot(dir, "resources-03b-account-tooltip", cx).await?;
    pointer_away(&view, WindowKind::Resources, cx).await?;
    type_over(&view, todos, mirelle, dir, cx).await?;

    edit(todos, cx, |todos, cx| todos.select(sefa, cx));
    type_an_amount(&view, todos, sefa, dir, cx).await?;
    edit(todos, cx, |todos, cx| {
        todos.set_amount(sefa, "shugo", Some(24), cx);
        todos.set_amount(sefa, "nightmare", Some(14), cx);
    });
    resources_shot(dir, "resources-09-typed-full", cx).await?;
    edit(todos, cx, |todos, cx| todos.follow_the_game(cx));
    pointer_away(&view, WindowKind::Resources, cx).await?;

    let full: Vec<(u32, u32, u32)> = fixtures::MIRELLE_TICKETS
        .iter()
        .map(|&(ticket, count, charge)| match ticket {
            60_000_001 => (ticket, 840, 245),
            1 => (ticket, 21, 3),
            8 => (ticket, 2, 12),
            4 => (ticket, 14, 0),
            101 => (ticket, 14, 7),
            _ => (ticket, count, charge),
        })
        .collect();
    login(todos, "Mirelle", now_ms(), &full, cx);
    set_reset_ahead(5, cx);
    pause(cx, NOTE_GONE).await;
    resources_shot(dir, "resources-04a-full-calm", cx).await?;
    shot(dir, "todos-31a-game-full-calm", cx).await?;
    set_reset_ahead(1, cx);
    pause(cx, TICK).await;
    resources_shot(dir, "resources-04-full", cx).await?;
    shot(dir, "todos-31-game-full", cx).await?;

    set_membership(false, cx);
    pause(cx, TICK).await;
    resources_shot(dir, "resources-05-membership-off", cx).await?;

    identify("Tavrin", cx);
    pause(cx, TICK).await;
    let shown = cx.update(|cx| todos.read(cx).list()).map(|list| list.character.name);
    println!("resources: Tavrin in game: the window shows {shown:?}");
    if shown.as_deref() != Some("Tavrin") {
        return Err("the Resources window did not follow the character in game".to_string());
    }
    resources_shot(dir, "resources-06-follows-tavrin", cx).await?;
    identify("Mirelle", cx);

    println!("the foreground window during the resources run: {:?} at the start", foreground_before.0);
    capture_reminders::check_never_focused(WindowKind::Resources, "the Resources window after its run", capture_reminders::foreground(), cx)?;
    capture_reminders::check_never_focused(WindowKind::Todos, "the to-do window beside it", capture_reminders::foreground(), cx)?;
    close(WindowKind::Resources, cx).await?;
    println!("resources: closed, the registry let go of it");
    Ok(())
}

/// The real entry point: a click on Odyle Energy's count on the toolbar.
async fn open_from_toolbar(model: &Entity<MeterModel>, cx: &mut AsyncApp) -> Result<()> {
    capture_toolbar::click(Slot::Odyle, cx).await?;
    let open = cx.update(|cx| secondary::window_of(WindowKind::Resources, cx).is_some());
    println!("entry point: click on Odyle Energy on the toolbar: Resources window open {open}");
    if !open {
        return Err("the Resources window did not open from the toolbar".to_string());
    }
    let before = cx.update(|cx| cx.windows().len());
    cx.update(|cx| secondary::open_resources(model, cx));
    pause(cx, TICK).await;
    let after = cx.update(|cx| cx.windows().len());
    println!("single instance: the Resources window opened twice, {before} windows before the second open, {after} after");
    if before != after {
        return Err("a second open made a second Resources window".to_string());
    }
    Ok(())
}

/// A click on Shugo keys' figure opens the field on the game's 7;
/// "9" typed and Enter keeps it over the game's value.
async fn type_over(view: &WindowHandle<TodoView>, todos: &Entity<Todos>, mirelle: CharacterId, dir: &Path, cx: &mut AsyncApp) -> Result<()> {
    let hwnd = window_hwnd(WindowKind::Resources, cx)?;
    view.update(cx, |view, window, cx| view.open_amount_field("shugo", Typing::Amount, Some(7), window, cx)).map_err(gone)?;
    pause(cx, TICK).await;
    post(hwnd, WM_CHAR, '9' as usize, 0);
    pause(cx, TICK).await;
    resources_shot(dir, "resources-02a-typing-over", cx).await?;
    key(hwnd, VK_RETURN, cx).await;
    until_field_closes(view, cx).await;
    let value = amount_of(todos, mirelle, "shugo", cx);
    println!("typing over: \"9\" over the game's 7 on Shugo keys, then Enter: {value:?}");
    if value != Some(9) {
        return Err("typing over: Enter did not keep the value typed over the game's".to_string());
    }
    resources_shot(dir, "resources-02b-typed-over-live", cx).await?;
    edit(todos, cx, |todos, cx| {
        todos.set_amount(mirelle, "shugo", None, cx);
    });
    let value = amount_of(todos, mirelle, "shugo", cx);
    println!("typing over: cleared: back to the game's {value:?}");
    if value != Some(7) {
        return Err("typing over: clearing did not go back to the game's value".to_string());
    }
    Ok(())
}

/// Before the game said anything (Sefa): a field opened on Shugo keys takes
/// typed digits and Enter keeps them; one on Invasion keys keeps nothing on
/// Escape.
async fn type_an_amount(view: &WindowHandle<TodoView>, todos: &Entity<Todos>, sefa: CharacterId, dir: &Path, cx: &mut AsyncApp) -> Result<()> {
    let hwnd = window_hwnd(WindowKind::Resources, cx)?;
    view.update(cx, |view, window, cx| view.open_amount_field("shugo", Typing::Amount, None, window, cx)).map_err(gone)?;
    pause(cx, TICK).await;
    for digit in "24".chars() {
        post(hwnd, WM_CHAR, digit as usize, 0);
    }
    pause(cx, TICK).await;
    resources_shot(dir, "resources-07-typing-an-amount", cx).await?;
    key(hwnd, VK_RETURN, cx).await;
    until_field_closes(view, cx).await;
    let value = amount_of(todos, sefa, "shugo", cx);
    let open = view.update(cx, |view, _, _| view.amount_field_is_open()).map_err(gone)?;
    println!("amount: \"24\" typed on Shugo keys, then Enter: tracked {value:?}, field still open {open}");
    if value != Some(24) || open {
        return Err("amount: Enter did not keep the typed amount".to_string());
    }
    resources_shot(dir, "resources-08-amount-kept", cx).await?;

    view.update(cx, |view, window, cx| view.open_amount_field("invasion", Typing::Amount, None, window, cx)).map_err(gone)?;
    pause(cx, TICK).await;
    post(hwnd, WM_CHAR, '3' as usize, 0);
    key(hwnd, VK_ESCAPE, cx).await;
    let value = amount_of(todos, sefa, "invasion", cx);
    println!("amount: \"3\" typed on Invasion keys, then Escape: tracked {value:?}");
    if value.is_some() {
        return Err("amount: Escape kept what was typed".to_string());
    }
    capture_reminders::check_never_focused(WindowKind::Resources, "after typing an amount", capture_reminders::foreground(), cx)
}

fn amount_of(todos: &Entity<Todos>, id: CharacterId, row: &'static str, cx: &mut AsyncApp) -> Option<u32> {
    let list = cx.update(|cx| todos.read(cx).list_of(id))?;
    list.resources.rows.iter().find_map(|candidate| match &candidate.kind {
        RowKind::Amount(amount) if candidate.id == RowId::Builtin(row) && amount.tracked => Some(amount.value),
        _ => None,
    })
}

fn count_of(todos: &Entity<Todos>, id: CharacterId, row: &'static str, cx: &mut AsyncApp) -> Option<u32> {
    let list = cx.update(|cx| todos.read(cx).list_of(id))?;
    list.sections.iter().flat_map(|section| &section.rows).find_map(|candidate| match candidate.kind {
        RowKind::Count { count, .. } if candidate.id == RowId::Builtin(row) => Some(count),
        _ => None,
    })
}

/// Ascension Trial's 2 of 3 from the game typed over as 1: the pencil, then
/// back to the game's value.
async fn type_over_weekly(view: &WindowHandle<TodoView>, todos: &Entity<Todos>, mirelle: CharacterId, dir: &Path, cx: &mut AsyncApp) -> Result<()> {
    let hwnd = window_hwnd(WindowKind::Todos, cx)?;
    view.update(cx, |view, window, cx| view.open_amount_field("trial", Typing::Progress, Some(2), window, cx)).map_err(gone)?;
    pause(cx, TICK).await;
    post(hwnd, WM_CHAR, '1' as usize, 0);
    key(hwnd, VK_RETURN, cx).await;
    until_field_closes(view, cx).await;
    shot(dir, "todos-29b-game-weekly-typed-over", cx).await?;
    edit(todos, cx, |todos, cx| {
        todos.set_progress(mirelle, "trial", None, cx);
    });
    Ok(())
}

/// The pointer on a row's figure until its tooltip shows. The figure's centre
/// is 50 px before the end of the row's controls (the extra pool's 26, a 1 px
/// gap, half of 46), which end 3 px from the row's edge, before the × slot
/// (20 and a 6 px gap) in the to-do window.
async fn tooltip_on(view: &WindowHandle<TodoView>, kind: WindowKind, row: &str, cx: &mut AsyncApp) -> Result<()> {
    let (bounds, scale) = row_bounds(view, row, cx)?;
    let x_slot = if kind == WindowKind::Resources { 0. } else { 26. };
    let figure = device(f32::from(bounds.right()) - 3. - x_slot - 50., f32::from(bounds.center().y), scale);
    hover_until_tooltip(kind, figure, &format!("the {row} figure"), cx).await
}

/// The pointer on a row's "Account" icon until its tooltip shows.
async fn account_tooltip_on(view: &WindowHandle<TodoView>, kind: WindowKind, row: &str, cx: &mut AsyncApp) -> Result<()> {
    let (bounds, scale) = row_bounds(view, &format!("account-{row}"), cx)?;
    let tag = device(f32::from(bounds.center().x), f32::from(bounds.center().y), scale);
    hover_until_tooltip(kind, tag, &format!("the {row} Account icon"), cx).await
}

async fn hover_until_tooltip(kind: WindowKind, at: (i32, i32), what: &str, cx: &mut AsyncApp) -> Result<()> {
    let hwnd = window_hwnd(kind, cx)?;
    post(hwnd, WM_MOUSEMOVE, 0, pack((at.0 - 2, at.1)));
    pause(cx, TICK).await;
    post(hwnd, WM_MOUSEMOVE, 0, pack(at));
    pause(cx, TOOLTIP_WAIT).await;
    println!("{kind:?}: pointer on {what}: {}", off_screen::frame_report(hwnd));
    Ok(())
}

async fn pointer_away(view: &WindowHandle<TodoView>, kind: WindowKind, cx: &mut AsyncApp) -> Result<()> {
    let hwnd = window_hwnd(kind, cx)?;
    let scale = view.update(cx, |_, window, _| window.scale_factor()).map_err(gone)?;
    post(hwnd, WM_MOUSEMOVE, 0, pack(device(60., 14., scale)));
    pause(cx, TICK).await;
    Ok(())
}

fn login(todos: &Entity<Todos>, name: &str, at_ms: i64, tickets: &[(u32, u32, u32)], cx: &mut AsyncApp) {
    let report = fixtures::ticket_login(&known(name), at_ms, tickets);
    cx.update(|cx| todos.update(cx, |todos, cx| todos.apply_tickets(&[report], cx)));
}

fn edit(todos: &Entity<Todos>, cx: &mut AsyncApp, change: impl FnOnce(&mut Todos, &mut gpui::Context<Todos>)) {
    cx.update(|cx| todos.update(cx, change));
}

async fn resources_shot(dir: &Path, name: &str, cx: &mut AsyncApp) -> Result<()> {
    shoot_window(WindowKind::Resources, dir, name, cx).await?;
    let hwnd = window_hwnd(WindowKind::Resources, cx)?;
    println!("{name}: {}", off_screen::frame_report(hwnd));
    Ok(())
}
