//! What the toolbar shows of the resources: Odyle Energy and Shugo keys
//! always, and the others that are full with a refill coming, from the to-do
//! data of the character the to-dos follow.

use crate::clock::{clock_time, LocalZone};
use crate::i18n::{duration_words, fl};
use crate::todos::{self, AmountView, RowId, RowKind, RowView, TodoList, Todos, ODYLE, SHUGO_KEYS};

#[derive(Clone, Debug, PartialEq)]
pub struct Counts {
    pub odyle: Pinned,
    pub shugo: Pinned,
    /// Full with a refill coming, Odyle Energy and Shugo keys left out: the
    /// warning's dropdown.
    pub others: Vec<FullLine>,
    /// Every resource full with a refill coming: the folded toolbar's warning.
    pub full_total: usize,
}

/// A count always on the toolbar.
#[derive(Clone, Debug, PartialEq)]
pub struct Pinned {
    pub name: String,
    /// None before any value, from the game or typed.
    pub figure: Option<Figure>,
    /// The tooltip's second line.
    pub tip: String,
    pub label: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Figure {
    pub value: u32,
    pub cap: u32,
    pub extra: u32,
    /// The regular pool is at its cap and the refill that would be lost is close.
    pub full: bool,
}

/// A row of the warning's dropdown: "Invasion keys 7/7 · refills 09:00".
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FullLine {
    pub name: String,
    pub value: u32,
    pub cap: u32,
    pub refills_at: String,
}

impl Counts {
    pub fn of(todos: &Todos, now_ms: i64) -> Self {
        let list = todos.list();
        let full = todos.full_resources();
        let others = full
            .iter()
            .filter(|resource| resource.id != ODYLE && resource.id != SHUGO_KEYS)
            .map(|resource| FullLine {
                name: resource.label.clone(),
                value: resource.value,
                cap: resource.cap,
                refills_at: clock_time(&LocalZone, resource.refill_at_ms).to_string(),
            })
            .collect();
        Self {
            odyle: pinned(list.as_ref(), ODYLE, now_ms),
            shugo: pinned(list.as_ref(), SHUGO_KEYS, now_ms),
            others,
            full_total: full.len(),
        }
    }
}

fn pinned(list: Option<&TodoList>, id: &'static str, now_ms: i64) -> Pinned {
    let row = list.and_then(|list| list.resources.rows.iter().find(|row| row.id == RowId::Builtin(id)));
    let name = row.map_or_else(|| todos::row_label(id).unwrap_or(id).to_string(), |row| row.label.clone());
    match row.and_then(|row| tracked_amount(row).map(|amount| (row, amount))) {
        Some((row, amount)) => {
            let figure = Figure {
                value: amount.value,
                cap: amount.cap,
                extra: row.extra,
                full: amount.warned,
            };
            Pinned { tip: tip_line(&figure, amount, now_ms), label: label(&name, &figure), name, figure: Some(figure) }
        }
        None => Pinned {
            tip: fl!("toolbar-count-no-value"),
            label: fl!("toolbar-count-label-none", name = name.clone()),
            name,
            figure: None,
        },
    }
}

fn tracked_amount(row: &RowView) -> Option<&AmountView> {
    match &row.kind {
        RowKind::Amount(amount) if amount.tracked => Some(amount),
        _ => None,
    }
}

/// "245 of 840, +380 extra · full in 2 d 14 h. Click for all resources."
fn tip_line(figure: &Figure, amount: &AmountView, now_ms: i64) -> String {
    let of = if figure.extra > 0 {
        fl!("toolbar-count-of-extra", value = figure.value, cap = figure.cap, extra = figure.extra)
    } else {
        fl!("toolbar-count-of", value = figure.value, cap = figure.cap)
    };
    let when = if amount.full {
        amount.status_text.clone()
    } else {
        amount.full_at_ms.map(|at| fl!("toolbar-count-full-in", time = duration_words(at - now_ms)))
    };
    let line = match when {
        Some(when) => format!("{of} \u{b7} {when}"),
        None => of,
    };
    format!("{line}. {}", fl!("toolbar-count-click"))
}

fn label(name: &str, figure: &Figure) -> String {
    let (value, cap, extra) = (figure.value, figure.cap, figure.extra);
    match (figure.full, extra > 0) {
        (false, false) => fl!("toolbar-count-label", name = name, value = value, cap = cap),
        (false, true) => fl!("toolbar-count-label-extra", name = name, value = value, cap = cap, extra = extra),
        (true, false) => fl!("toolbar-count-label-full", name = name, value = value, cap = cap),
        (true, true) => fl!("toolbar-count-label-full-extra", name = name, value = value, cap = cap, extra = extra),
    }
}
