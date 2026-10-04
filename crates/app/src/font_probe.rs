//! `--font-probe`: shows whether GPUI can select the families and features the
//! approved design relies on. Prints measurements and draws a specimen sheet.

use std::ffi::c_void;
use std::path::PathBuf;
use std::sync::atomic::Ordering;
use std::sync::Arc;
use std::time::Duration;

use gpui::{
    div, font, prelude::*, px, rgb, size, App, Bounds, Context, Font, FontFeatures, FontWeight,
    IntoElement, Render, SharedString, TextRun, Window, WindowBounds, WindowOptions,
};
use windows::Win32::Foundation::HWND;

use crate::fonts::OVERLAY_FAMILY;

const SHEET_WIDTH: f32 = 760.0;
const SHEET_HEIGHT: f32 = 640.0;

const SAMPLE: &str = "Aether 0123456789 1,234,567 DPS";
const DIGITS_ONES: &str = "1111111111";
const DIGITS_ZEROS: &str = "0000000000";
const MEASURE_SIZE: f32 = 20.0;

struct Specimen {
    label: &'static str,
    font: Font,
}

fn tnum(on: bool) -> FontFeatures {
    FontFeatures(Arc::new(vec![("tnum".into(), u32::from(on))]))
}

fn specimen(label: &'static str, family: &'static str, weight: FontWeight, tabular: bool) -> Specimen {
    let mut font = font(family);
    font.weight = weight;
    if tabular {
        font.features = tnum(true);
    }
    Specimen { label, font }
}

fn specimens() -> Vec<Specimen> {
    let regular = FontWeight::NORMAL;
    let semibold = FontWeight::SEMIBOLD;
    vec![
        specimen("control: family that does not exist", "Aether Not A Font", regular, false),
        specimen("Bahnschrift 400", "Bahnschrift", regular, false),
        specimen("Bahnschrift 600", "Bahnschrift", semibold, false),
        specimen("Bahnschrift SemiCondensed 400", "Bahnschrift SemiCondensed", regular, false),
        specimen("Bahnschrift SemiCondensed 600", "Bahnschrift SemiCondensed", semibold, false),
        specimen("Bahnschrift SemiBold SemiCondensed", "Bahnschrift SemiBold SemiCondensed", regular, false),
        specimen("Bahnschrift Condensed 600", "Bahnschrift Condensed", semibold, false),
        specimen("Segoe UI Variable 400", "Segoe UI Variable", regular, false),
        specimen("Segoe UI Variable 600", "Segoe UI Variable", semibold, false),
        specimen("Segoe UI Variable Text 400", "Segoe UI Variable Text", regular, false),
        specimen("Segoe UI Variable Display 600", "Segoe UI Variable Display", semibold, false),
        specimen("Segoe UI 400", "Segoe UI", regular, false),
        specimen("Barlow Semi Condensed 400", OVERLAY_FAMILY, regular, false),
        specimen("Barlow Semi Condensed 500", OVERLAY_FAMILY, FontWeight::MEDIUM, false),
        specimen("Barlow Semi Condensed 600", OVERLAY_FAMILY, semibold, false),
        specimen("Barlow Semi Condensed 700", OVERLAY_FAMILY, FontWeight::BOLD, false),
    ]
}

fn width(window: &Window, font: &Font, text: &str) -> f32 {
    let run = TextRun {
        len: text.len(),
        font: font.clone(),
        color: gpui::black(),
        background_color: None,
        underline: None,
        strikethrough: None,
    };
    window
        .text_system()
        .shape_line(SharedString::from(text.to_string()), px(MEASURE_SIZE), &[run], None)
        .width
        .as_f32()
}

fn print_measurements(window: &Window, cx: &App) {
    let names = cx.text_system().all_font_names();
    for name in names
        .iter()
        .filter(|name| name.contains("Bahnschrift") || name.contains("Segoe UI Variable") || name.contains("Barlow"))
    {
        println!("installed family: {name}");
    }

    print_specimen_widths(window, cx, false);
    print_tnum_widths(window);

    let file = "C:/Windows/Fonts/bahnschrift.ttf";
    match std::fs::read(file) {
        Ok(bytes) => match cx.text_system().add_fonts(vec![bytes.into()]) {
            Ok(()) => {
                println!("after add_fonts({file}):");
                print_specimen_widths(window, cx, true);
            }
            Err(error) => println!("add_fonts({file}) failed: {error:#}"),
        },
        Err(error) => println!("cannot read {file}: {error}"),
    }
}

fn print_specimen_widths(window: &Window, cx: &App, fresh_cache_key: bool) {
    println!("width of {SAMPLE:?} at {MEASURE_SIZE}px:");
    for mut specimen in specimens() {
        if fresh_cache_key {
            specimen.font.features = FontFeatures(Arc::new(vec![("ss20".into(), 0)]));
        }
        let id = cx.text_system().resolve_font(&specimen.font);
        println!(
            "  {:<42} font id {:>3}  {:>7.2}px",
            specimen.label,
            id.0,
            width(window, &specimen.font, SAMPLE)
        );
    }

}

fn print_tnum_widths(window: &Window) {
    println!("tnum: width of {DIGITS_ONES:?} vs {DIGITS_ZEROS:?} (equal when digits are tabular)");
    for (label, family, weight) in [
        ("Bahnschrift 600", "Bahnschrift", FontWeight::SEMIBOLD),
        ("Bahnschrift SemiCondensed 600", "Bahnschrift SemiCondensed", FontWeight::SEMIBOLD),
        ("Segoe UI Variable 400", "Segoe UI Variable", FontWeight::NORMAL),
        ("Segoe UI Variable Text 400", "Segoe UI Variable Text", FontWeight::NORMAL),
        ("Barlow Semi Condensed 500", OVERLAY_FAMILY, FontWeight::MEDIUM),
        ("Barlow Semi Condensed 600", OVERLAY_FAMILY, FontWeight::SEMIBOLD),
    ] {
        for on in [false, true] {
            let mut font = font(family);
            font.weight = weight;
            font.features = tnum(on);
            let ones = width(window, &font, DIGITS_ONES);
            let zeros = width(window, &font, DIGITS_ZEROS);
            println!(
                "  {label:<32} tnum {:<3}  1s {ones:>7.2}px  0s {zeros:>7.2}px  tabular: {}",
                if on { "on" } else { "off" },
                (ones - zeros).abs() < 0.01
            );
        }
    }
}

struct FontSheet;

impl Render for FontSheet {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        let mut sheet = div()
            .size_full()
            .p(px(16.))
            .flex()
            .flex_col()
            .gap(px(4.))
            .bg(rgb(0x100e0c))
            .text_color(rgb(0xfff6ec))
            .font_family("Segoe UI Variable")
            .text_size(px(12.))
            .child("Font specimen: label, then the sample set in that font");

        for specimen in specimens() {
            sheet = sheet.child(
                div()
                    .flex()
                    .items_baseline()
                    .gap(px(12.))
                    .child(
                        div()
                            .w(px(250.))
                            .text_size(px(11.))
                            .text_color(rgb(0x9a948c))
                            .child(specimen.label),
                    )
                    .child(
                        div()
                            .font_family(specimen.font.family.clone())
                            .font_weight(specimen.font.weight)
                            .font_features(specimen.font.features.clone())
                            .text_size(px(MEASURE_SIZE))
                            .child(SAMPLE),
                    ),
            );
        }

        sheet = sheet.child(div().mt(px(10.)).child("tnum off, then on: a column of figures"));
        for (family, weight) in [
            ("Bahnschrift SemiCondensed", FontWeight::SEMIBOLD),
            ("Segoe UI Variable", FontWeight::NORMAL),
            (OVERLAY_FAMILY, FontWeight::SEMIBOLD),
        ] {
            for on in [false, true] {
                sheet = sheet.child(
                    div()
                        .flex()
                        .gap(px(12.))
                        .child(
                            div()
                                .w(px(250.))
                                .text_size(px(11.))
                                .text_color(rgb(0x9a948c))
                                .child(format!("{family}, tnum {}", if on { "on" } else { "off" })),
                        )
                        .child(
                            div()
                                .font_family(family)
                                .font_weight(weight)
                                .font_features(tnum(on))
                                .text_size(px(MEASURE_SIZE))
                                .child("11111  00000  88888  12,345"),
                        ),
                );
            }
        }
        sheet
    }
}

/// With `capture`, the sheet is drawn off screen and saved as `specimen.png`
/// there; otherwise it is shown at the top left of the primary monitor.
pub fn open(cx: &mut App, capture: Option<PathBuf>) -> Result<(), String> {
    let bounds = Bounds::new(gpui::point(px(40.), px(40.)), size(px(SHEET_WIDTH), px(SHEET_HEIGHT)));
    let handle = cx
        .open_window(
            WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(bounds)),
                focus: false,
                show: capture.is_none(),
                ..Default::default()
            },
            |window, cx| {
                print_measurements(window, cx);
                if let Ok(hwnd) = crate::overlay::hwnd(window) {
                    println!("PHASE specimen {}", crate::probe::window_rect_text(hwnd));
                }
                cx.new(|_| FontSheet)
            },
        )
        .map_err(|error| format!("cannot open the font probe window: {error:#}"))?;

    let Some(dir) = capture else {
        return Ok(());
    };
    let hwnd = handle
        .update(cx, |_, window, _| -> Result<_, String> {
            let hwnd = crate::overlay::hwnd(window)?;
            let scale = window.scale_factor();
            crate::overlay::off_screen::show(hwnd, SHEET_WIDTH * scale, SHEET_HEIGHT * scale)?;
            Ok(hwnd)
        })
        .map_err(|error| format!("cannot reach the font probe window: {error:#}"))??;
    let raw = hwnd.0 as isize;
    cx.spawn(async move |cx| {
        cx.background_executor().timer(Duration::from_millis(1000)).await;
        let path = dir.join("specimen.png");
        let result = std::fs::create_dir_all(&dir)
            .map_err(|error| format!("cannot create {}: {error}", dir.display()))
            .and_then(|()| crate::capture::grab(HWND(raw as *mut c_void), &path));
        match result {
            Ok((width, height)) => println!("captured {} ({width}x{height})", path.display()),
            Err(error) => {
                eprintln!("specimen capture failed: {error}");
                crate::FAILED.store(true, Ordering::SeqCst);
            }
        }
        cx.update(|cx| cx.quit());
    })
    .detach();
    Ok(())
}
