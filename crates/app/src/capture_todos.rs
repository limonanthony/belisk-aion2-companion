//! Off-screen captures and checks of the to-do window, run by `--capture
//! --fixtures` after the PvP window's. The characters and their progress are
//! invented. The window is opened from the toolbar and stays past
//! every monitor; fields take the keyboard inside GPUI only, so nothing asks
//! Windows for the foreground, and the only input is mouse and key messages
//! posted to our own windows. The run fails when the window could take
//! focus, a row's controls flash while the pointer moves inside it, or the
//! row that slides under the pointer after a hide shows its controls before
//! the pointer moves.

use std::path::Path;
use std::time::Duration;

use meter_core::identity::KnownCharacter;
use gpui::{AsyncApp, Entity, WindowHandle};
use windows::Win32::UI::WindowsAndMessaging::{WM_CHAR, WM_KEYDOWN, WM_KEYUP, WM_LBUTTONDOWN, WM_LBUTTONUP, WM_MOUSEMOVE};

use crate::capture::MK_LBUTTON;
use crate::capture_reminders;
use crate::capture_resources;
use crate::capture_toolbar;
use crate::capture_secondary::{
    check_title, close, pack, pause, post, report_window, shoot_window, show_scene, until_handled, window_handle,
    window_hwnd,
};
use crate::fixtures;
use crate::meter::MeterModel;
use crate::overlay::off_screen;
use crate::secondary::{self, WindowKind};
use crate::settings::{self, Section, SettingsView};
use crate::todos::{self, Cadence, CharacterId, HiddenIn, Identified, NewOwnTodo, OwnKind, PlayerRef, RowId, TodoView, Todos, Typing};
use crate::toolbar::{Slot, Tool};

const TICK: Duration = Duration::from_millis(150);
const MOVE_GAP: Duration = Duration::from_millis(60);
const KEY_UP: isize = 0xC000_0001;
const VK_TAB: usize = 0x09;
const VK_RETURN: usize = 0x0D;
const VK_ESCAPE: usize = 0x1B;

type Result<T, E = String> = std::result::Result<T, E>;

/// Invented characters: the one in game first.
const CHARACTERS: [(&str, u32, &str); 3] = [("Mirelle", 1002, "RANGER"), ("Tavrin", 1002, "TEMPLAR"), ("Sefa", 1003, "CLERIC")];

pub async fn from_fixtures(model: &Entity<MeterModel>, dir: &Path, cx: &mut AsyncApp) -> Result<()> {
    show_scene(model, fixtures::open_world(), cx);
    let foreground_before = capture_reminders::foreground();
    open_from_toolbar(model, cx).await?;
    report_window(WindowKind::Todos, cx)?;
    let view = window_handle::<TodoView>(WindowKind::Todos, cx)?;
    view.update(cx, |view, _, _| view.keep_keyboard_in_app()).map_err(gone)?;
    shot(dir, "todos-00-no-character", cx).await?;

    let todos = cx.update(|cx| todos::global(cx));
    remember_characters(cx);
    set_membership(true, cx);
    identify("Mirelle", cx);
    let ids: Vec<CharacterId> = cx.update(|cx| todos.read(cx).characters().into_iter().map(|character| character.id).collect());
    let [mirelle, tavrin, sefa] = ids[..] else {
        return Err(format!("the tracker made {} lists for the 3 remembered characters", ids.len()));
    };
    shot(dir, "todos-01-fresh-week", cx).await?;
    check_title(WindowKind::Todos, "fresh week", cx).await?;

    let legion = edit(&todos, cx, |todos, cx| {
        let legion = todos
            .add_own(mirelle, NewOwnTodo { name: "Legion check-in".into(), kind: OwnKind::Tick, cadence: Cadence::Daily }, cx)
            .map_err(|error| format!("cannot add the invented own to-do: {error:?}"))?;
        todos.toggle_tick(mirelle, RowId::Own(legion), cx);
        for (row, by) in [("duty", 2), ("trial", 3), ("subj", 1), ("fissure", 9), ("transc", 12), ("conqk", 35), ("cradle", 4), ("ordeal", 2), ("forge", 2)] {
            todos.adjust_count(mirelle, RowId::Builtin(row), by, cx);
        }
        todos.toggle_tick(mirelle, RowId::Builtin("forgeboss"), cx);
        todos.set_amount(mirelle, "odyle", Some(425), cx);
        todos.set_section_folded(todos::Section::Recharges, true, cx);
        todos.set_group_open("sanct", true, cx);
        Ok(legion)
    })?;
    shot(dir, "todos-02-under-way", cx).await?;

    view.update(cx, |view, _, cx| view.force_pointer_row(Some(RowId::Own(legion)), cx)).map_err(gone)?;
    shot(dir, "todos-03-pointer-on-done-row", cx).await?;
    view.update(cx, |view, _, cx| view.force_pointer_row(Some(RowId::Builtin("subj")), cx)).map_err(gone)?;
    shot(dir, "todos-04-pointer-on-count", cx).await?;
    view.update(cx, |view, _, cx| view.force_pointer_row(None, cx)).map_err(gone)?;

    edit(&todos, cx, |todos, cx| {
        todos.adjust_count(mirelle, RowId::Builtin("duty"), 3, cx);
        todos.set_section_folded(todos::Section::Recharges, false, cx);
        todos.set_group_open("sanct", false, cx);
        Ok(())
    })?;
    shot(dir, "todos-05-count-done-amount-typed", cx).await?;

    hover_inside_a_row(&view, cx).await?;
    hide_under_the_pointer(&view, &todos, mirelle, cx).await?;

    type_an_amount(&view, &todos, mirelle, dir, cx).await?;
    edit(&todos, cx, |todos, cx| {
        todos.set_amount(mirelle, "shugo", Some(24), cx);
        todos.set_amount(mirelle, "nightmare", Some(14), cx);
        Ok(())
    })?;
    shot(dir, "todos-08-full", cx).await?;

    edit(&todos, cx, |todos, cx| {
        for row in ["deus", "arkanis", "balaur", "fafnium", "trial", "subj"] {
            todos.hide(mirelle, row, cx);
        }
        todos.set_amount(mirelle, "shugo", None, cx);
        todos.set_amount(mirelle, "nightmare", None, cx);
        Ok(())
    })?;
    shot(dir, "todos-09-hidden", cx).await?;
    view.update(cx, |view, _, cx| view.list_hidden(HiddenIn::Section(todos::Section::Recharges), cx)).map_err(gone)?;
    shot(dir, "todos-10-show", cx).await?;

    add_by_typing(&view, &todos, mirelle, dir, cx).await?;
    keyboard_focus(&view, legion, dir, cx).await?;

    let removed = edit(&todos, cx, |todos, cx| Ok(todos.remove_own(mirelle, legion, cx)))?;
    view.update(cx, |view, window, cx| view.show_removed_note(removed, window, cx)).map_err(gone)?;
    shot(dir, "todos-13-removed", cx).await?;

    view.update(cx, |view, _, cx| view.show_picker(cx)).map_err(gone)?;
    edit(&todos, cx, |todos, cx| {
        todos.set_amount(tavrin, "shugo", Some(24), cx);
        todos.adjust_count(sefa, RowId::Builtin("fissure"), 14, cx);
        Ok(())
    })?;
    shot(dir, "todos-14-picker", cx).await?;
    view.update(cx, |view, window, cx| view.close_character_picker(window, cx)).map_err(gone)?;

    everything_done(&todos, mirelle, cx);
    shot(dir, "todos-15-everything-done", cx).await?;

    edit(&todos, cx, |todos, cx| {
        todos.set_group_open("explore", true, cx);
        todos.set_group_open("sanct", true, cx);
        for section in todos::Section::ALL {
            todos.set_section_folded(section, false, cx);
        }
        Ok(())
    })?;
    cx.update(|cx| todos.update(cx, |todos, cx| todos.reset_completely(cx)));
    shot(dir, "todos-16-long-list-scrolls", cx).await?;
    cx.update(|cx| settings::global(cx).update(cx, |settings, cx| settings.update(cx, |all| all.overlay.content_scale = 1.25)));
    shot(dir, "todos-17-scale-125", cx).await?;
    cx.update(|cx| settings::global(cx).update(cx, |settings, cx| settings.update(cx, |all| all.overlay.content_scale = 1.0)));

    capture_resources::from_fixtures(model, &view, [mirelle, tavrin, sefa], dir, cx).await?;
    set_membership(false, cx);

    // The run takes minutes; whoever uses the PC may switch windows meanwhile,
    // so only "the to-do window never took the focus" is checked here.
    println!("the foreground window during the run: {:?} at the start", foreground_before.0);
    capture_reminders::check_never_focused(WindowKind::Todos, "the to-do window after the run", capture_reminders::foreground(), cx)?;
    change_opens_settings(model, cx).await?;
    close(WindowKind::Todos, cx).await?;
    println!("todos: closed, the registry let go of it");
    Ok(())
}

/// The real entry point: a click on To-dos on the toolbar.
async fn open_from_toolbar(model: &Entity<MeterModel>, cx: &mut AsyncApp) -> Result<()> {
    capture_toolbar::click(Slot::Tool(Tool::Todos), cx).await?;
    let open = cx.update(|cx| secondary::window_of(WindowKind::Todos, cx).is_some());
    println!("entry point: click on To-dos on the toolbar: to-do window open {open}");
    if !open {
        return Err("the to-do window did not open from the toolbar".to_string());
    }
    let before = cx.update(|cx| cx.windows().len());
    cx.update(|cx| secondary::open_todos(model, cx));
    pause(cx, TICK).await;
    let after = cx.update(|cx| cx.windows().len());
    println!("single instance: the to-do window opened twice, {before} windows before the second open, {after} after");
    if before != after {
        return Err("a second open made a second to-do window".to_string());
    }
    Ok(())
}

fn remember_characters(cx: &mut AsyncApp) {
    let known: Vec<KnownCharacter> = CHARACTERS
        .iter()
        .map(|&(name, server_id, class)| KnownCharacter { name: name.to_string(), server_id, class: class.to_string(), key: None })
        .collect();
    cx.update(|cx| settings::global(cx).update(cx, |settings, cx| settings.update(cx, |all| all.known_characters = known)));
}

/// The approved pages are drawn with the membership on (Shugo keys 21, Odyle
/// 840); Settings' own run starts from the default, off.
pub(crate) fn set_membership(on: bool, cx: &mut AsyncApp) {
    cx.update(|cx| settings::global(cx).update(cx, |settings, cx| settings.update(cx, |all| all.todos.membership = on)));
}

pub(crate) fn identify(name: &str, cx: &mut AsyncApp) {
    let (_, server_id, _) = CHARACTERS.iter().copied().find(|(known, _, _)| *known == name).unwrap_or(CHARACTERS[0]);
    let player = Identified::Named(PlayerRef { name: name.to_string(), server_id: Some(server_id), key: None });
    cx.update(|cx| todos::follow_player(cx, vec![player]));
}

/// The invented character `name` as the game names it.
pub(crate) fn known(name: &str) -> KnownCharacter {
    let (name, server_id, class) = CHARACTERS.iter().copied().find(|(known, _, _)| *known == name).unwrap_or(CHARACTERS[0]);
    KnownCharacter { name: name.to_string(), server_id, class: class.to_string(), key: None }
}

fn edit<R>(todos: &Entity<Todos>, cx: &mut AsyncApp, change: impl FnOnce(&mut Todos, &mut gpui::Context<Todos>) -> Result<R>) -> Result<R> {
    cx.update(|cx| todos.update(cx, change))
}

/// Every count and tick of the character done, every section folded.
fn everything_done(todos: &Entity<Todos>, id: CharacterId, cx: &mut AsyncApp) {
    cx.update(|cx| {
        todos.update(cx, |todos, cx| {
            let Some(list) = todos.list_of(id) else {
                return;
            };
            let rows = list.sections.iter().flat_map(|section| &section.rows).flat_map(|row| match &row.kind {
                todos::RowKind::Group(group) => group.children.iter().collect::<Vec<_>>(),
                _ => vec![row],
            });
            let pending: Vec<(RowId, todos::RowKind)> = rows.filter(|row| !row.hidden).map(|row| (row.id, row.kind.clone())).collect();
            for (row, kind) in pending {
                match kind {
                    todos::RowKind::Tick { done: false } => {
                        todos.toggle_tick(id, row, cx);
                    }
                    todos::RowKind::Count { count, target, done: false } => {
                        todos.adjust_count(id, row, (target - count) as i32, cx);
                    }
                    _ => {}
                }
            }
            for section in todos::Section::ALL {
                todos.set_section_folded(section, true, cx);
            }
        })
    });
}

/// The pointer moves about inside one row, over its name, its figure and
/// where its × is: the row's controls show from the first move to the last
/// and never go. Then it leaves for the header, and they go.
async fn hover_inside_a_row(view: &WindowHandle<TodoView>, cx: &mut AsyncApp) -> Result<()> {
    let hwnd = window_hwnd(WindowKind::Todos, cx)?;
    let (bounds, scale) = row_bounds(view, "fissure", cx)?;
    let y = f32::from(bounds.center().y);
    let points: Vec<(i32, i32)> = [0.05, 0.3, 0.6, 0.75, 0.9, 0.97, 0.5, 0.1]
        .iter()
        .map(|fraction| device(f32::from(bounds.left()) + f32::from(bounds.size.width) * fraction, y + (fraction - 0.5) * 6., scale))
        .collect();
    let mut seen = Vec::new();
    for point in &points {
        post(hwnd, WM_MOUSEMOVE, 0, pack(*point));
        pause(cx, MOVE_GAP).await;
        seen.push(view.update(cx, |view, _, _| view.revealed_row()).map_err(gone)?);
    }
    let shown = seen.iter().filter(|row| **row == Some(RowId::Builtin("fissure"))).count();
    println!("hover: {} moves inside the Daily Dungeon row: controls shown after {shown} of them ({seen:?})", points.len());
    if shown != points.len() {
        return Err("hover: the row's check and × did not stay while the pointer moved inside the row".to_string());
    }
    post(hwnd, WM_MOUSEMOVE, 0, pack(device(60., 14., scale)));
    pause(cx, MOVE_GAP).await;
    let after = view.update(cx, |view, _, _| view.revealed_row()).map_err(gone)?;
    println!("hover: the pointer left for the header: controls on {after:?}");
    if after.is_some() {
        return Err("hover: the controls stayed after the pointer left the row".to_string());
    }
    Ok(())
}

/// A click on a row's × hides it; the row that slides under the pointer
/// keeps its controls hidden until the pointer moves.
async fn hide_under_the_pointer(view: &WindowHandle<TodoView>, todos: &Entity<Todos>, id: CharacterId, cx: &mut AsyncApp) -> Result<()> {
    let hwnd = window_hwnd(WindowKind::Todos, cx)?;
    let (bounds, scale) = row_bounds(view, "shugo", cx)?;
    let x_centre = f32::from(bounds.right()) - 3. - 10.;
    let at = device(x_centre, f32::from(bounds.center().y), scale);
    post(hwnd, WM_MOUSEMOVE, 0, pack(at));
    pause(cx, MOVE_GAP).await;
    post(hwnd, WM_LBUTTONDOWN, MK_LBUTTON, pack(at));
    post(hwnd, WM_LBUTTONUP, 0, pack(at));
    pause(cx, TICK).await;
    let hidden = cx.update(|cx| todos.read(cx).list_of(id)).is_some_and(|list| {
        list.sections.iter().flat_map(|section| &section.rows).any(|row| row.id == RowId::Builtin("shugo") && row.hidden)
    });
    let (held, revealed) = view.update(cx, |view, _, _| (view.held_still(), view.revealed_row())).map_err(gone)?;
    println!("hide by pointer: a click on the × of Shugo Festival keys: hidden {hidden}; the row now under the pointer shows controls on {revealed:?}, held until the pointer moves {held}");
    if !hidden {
        return Err("hide by pointer: the click on the × did not hide the row".to_string());
    }
    if revealed.is_some() {
        return Err("hide by pointer: the row that slid under the pointer showed its controls at once".to_string());
    }
    post(hwnd, WM_MOUSEMOVE, 0, pack((at.0 - 4, at.1)));
    pause(cx, MOVE_GAP).await;
    let revealed = view.update(cx, |view, _, _| view.revealed_row()).map_err(gone)?;
    println!("hide by pointer: after the pointer moved: controls on {revealed:?}");
    if revealed != Some(RowId::Builtin("invasion")) {
        return Err("hide by pointer: the row under the pointer did not show its controls once the pointer moved".to_string());
    }
    cx.update(|cx| todos.update(cx, |todos, cx| todos.show(id, "shugo", cx)));
    post(hwnd, WM_MOUSEMOVE, 0, pack(device(60., 14., scale)));
    pause(cx, MOVE_GAP).await;
    Ok(())
}

/// A field opened on Shugo Festival keys takes typed digits; Enter keeps
/// them. One on Invasion keys: Escape keeps nothing.
async fn type_an_amount(view: &WindowHandle<TodoView>, todos: &Entity<Todos>, id: CharacterId, dir: &Path, cx: &mut AsyncApp) -> Result<()> {
    let hwnd = window_hwnd(WindowKind::Todos, cx)?;
    view.update(cx, |view, window, cx| view.open_amount_field("shugo", Typing::Amount, None, window, cx)).map_err(gone)?;
    pause(cx, TICK).await;
    for digit in "24".chars() {
        post(hwnd, WM_CHAR, digit as usize, 0);
    }
    pause(cx, TICK).await;
    shot(dir, "todos-06-typing-an-amount", cx).await?;
    key(hwnd, VK_RETURN, cx).await;
    let value = amount_of(todos, id, "shugo", cx);
    let open = view.update(cx, |view, _, _| view.amount_field_is_open()).map_err(gone)?;
    println!("amount: \"24\" typed on Shugo Festival keys, then Enter: tracked {value:?}, field still open {open}");
    if value != Some(24) || open {
        return Err("amount: Enter did not keep the typed amount".to_string());
    }
    shot(dir, "todos-07-amount-kept", cx).await?;

    view.update(cx, |view, window, cx| view.open_amount_field("invasion", Typing::Amount, None, window, cx)).map_err(gone)?;
    pause(cx, TICK).await;
    post(hwnd, WM_CHAR, '3' as usize, 0);
    key(hwnd, VK_ESCAPE, cx).await;
    let value = amount_of(todos, id, "invasion", cx);
    println!("amount: \"3\" typed on Invasion keys, then Escape: tracked {value:?}");
    if value.is_some() {
        return Err("amount: Escape kept what was typed".to_string());
    }
    capture_reminders::check_never_focused(WindowKind::Todos, "after typing an amount", capture_reminders::foreground(), cx)
}

/// "+ Add a to-do" opened, a name typed, Count of 3, Weekly; Enter adds it.
async fn add_by_typing(view: &WindowHandle<TodoView>, todos: &Entity<Todos>, id: CharacterId, dir: &Path, cx: &mut AsyncApp) -> Result<()> {
    let hwnd = window_hwnd(WindowKind::Todos, cx)?;
    view.update(cx, |view, window, cx| view.open_add_form(true, Cadence::Weekly, window, cx)).map_err(gone)?;
    pause(cx, TICK).await;
    for letter in "Arena of Tactics wins".chars() {
        post(hwnd, WM_CHAR, letter as usize, 0);
    }
    pause(cx, TICK).await;
    shot(dir, "todos-11-adding", cx).await?;
    key(hwnd, VK_RETURN, cx).await;
    let added = cx.update(|cx| todos.read(cx).list_of(id)).is_some_and(|list| {
        list.sections.iter().flat_map(|section| &section.rows).any(|row| row.yours && row.label == "Arena of Tactics wins")
    });
    let (form, typing) = view.update(cx, |view, _, _| (view.form_is_open(), view.is_typing())).map_err(gone)?;
    println!("add: a name typed and Enter: added {added}, form still open {form}, still typing {typing}");
    if !added || form || typing {
        return Err("add: Enter did not add the to-do and close the form".to_string());
    }
    shot(dir, "todos-12-added", cx).await
}

/// Tab walks the controls: the ring on a tick row (its check and × show),
/// then on a count's figure.
async fn keyboard_focus(view: &WindowHandle<TodoView>, legion: u32, dir: &Path, cx: &mut AsyncApp) -> Result<()> {
    let hwnd = window_hwnd(WindowKind::Todos, cx)?;
    for (target, name) in [(format!("row:own-{legion}"), "todos-18-focus-on-row"), ("fig:fissure".to_string(), "todos-19-focus-on-figure")] {
        let mut reached = false;
        for _ in 0..80 {
            key(hwnd, VK_TAB, cx).await;
            if view.update(cx, |view, window, _| view.focused_control(window)).ok().flatten().as_deref() == Some(target.as_str()) {
                reached = true;
                break;
            }
        }
        println!("keyboard: Tab reached {target}: {reached}");
        if !reached {
            return Err(format!("keyboard: Tab never reached {target}"));
        }
        shot(dir, name, cx).await?;
    }
    Ok(())
}

/// "Change" opens Settings on the To-dos page.
async fn change_opens_settings(model: &Entity<MeterModel>, cx: &mut AsyncApp) -> Result<()> {
    cx.update(|cx| secondary::open_settings_at(model, Section::Todos, cx));
    pause(cx, TICK * 3).await;
    let page = window_handle::<SettingsView>(WindowKind::Settings, cx)?
        .update(cx, |view, _, _| view.section())
        .map_err(|error| format!("the Settings window is gone: {error:#}"))?;
    println!("Change: Settings open on {page:?}");
    if page != Section::Todos {
        return Err("Change did not open Settings on To-dos".to_string());
    }
    close(WindowKind::Settings, cx).await
}

pub(crate) fn amount_of(todos: &Entity<Todos>, id: CharacterId, row: &'static str, cx: &mut AsyncApp) -> Option<u32> {
    let list = cx.update(|cx| todos.read(cx).list_of(id))?;
    list.sections.iter().flat_map(|section| &section.rows).find_map(|candidate| match &candidate.kind {
        todos::RowKind::Amount(amount) if candidate.id == RowId::Builtin(row) && amount.tracked => Some(amount.value),
        _ => None,
    })
}

pub(crate) fn row_bounds(view: &WindowHandle<TodoView>, key: &str, cx: &mut AsyncApp) -> Result<(gpui::Bounds<gpui::Pixels>, f32)> {
    view.update(cx, |view, window, _| view.row_bounds(key).map(|bounds| (bounds, window.scale_factor())))
        .map_err(gone)?
        .ok_or_else(|| format!("the {key} row is not drawn"))
}

/// Window coordinates (logical) as client device pixels.
pub(crate) fn device(x: f32, y: f32, scale: f32) -> (i32, i32) {
    ((x * scale).round() as i32, (y * scale).round() as i32)
}

pub(crate) async fn key(hwnd: windows::Win32::Foundation::HWND, vk: usize, cx: &mut AsyncApp) {
    post(hwnd, WM_KEYDOWN, vk, 1);
    post(hwnd, WM_KEYUP, vk, KEY_UP);
    until_handled(hwnd, cx).await;
    pause(cx, TICK).await;
}

pub(crate) async fn shot(dir: &Path, name: &str, cx: &mut AsyncApp) -> Result<()> {
    shoot_window(WindowKind::Todos, dir, name, cx).await?;
    let hwnd = window_hwnd(WindowKind::Todos, cx)?;
    println!("{name}: {}", off_screen::frame_report(hwnd));
    Ok(())
}

pub(crate) fn gone(error: impl std::fmt::Display) -> String {
    format!("the to-do window is gone: {error:#}")
}
