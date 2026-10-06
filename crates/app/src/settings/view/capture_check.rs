//! Settings → Capture → Capture check in the app's language. The core's
//! report is worded in English for its log; the window words it from each
//! check's id and state, and shows the core's own detail only where it holds
//! what the app cannot word: an OS or driver error.

use meter_core::preflight::{Check, CheckId, Fix, Report, Weight};

use crate::i18n::{fl, join_and};

/// One check as the page shows it: its line, and the core's detail under it.
pub(super) struct CheckLine {
    pub text: String,
    pub raw_detail: Option<String>,
}

pub(super) fn summary(report: &Report) -> String {
    let blocking: Vec<String> = report
        .checks
        .iter()
        .filter(|check| !check.ok && check.weight == Weight::Required)
        .map(|check| label(check.id))
        .collect();
    if blocking.is_empty() {
        fl!("settings-check-ready")
    } else {
        fl!("settings-check-blocked", names = join_and(&blocking))
    }
}

pub(super) fn line(check: &Check, report: &Report) -> CheckLine {
    let npcap_ok = report.checks.iter().any(|other| other.id == CheckId::Npcap && other.ok);
    let (status, raw_detail) = match (check.id, check.ok) {
        (CheckId::Elevation, true) => (fl!("settings-check-elevated"), None),
        (CheckId::Elevation, false) => (fl!("settings-check-not-elevated"), None),
        (CheckId::Npcap, true) => match adapter_count(&check.detail) {
            Some(count) => (fl!("settings-check-npcap-ok", count = count), None),
            None => (fl!("settings-check-available"), Some(check.detail.clone())),
        },
        (CheckId::Windivert, true) if npcap_ok && check.fix == Some(Fix::PlaceWinDivertFiles) => {
            (fl!("settings-check-windivert-absent"), None)
        }
        (CheckId::Windivert, true) if npcap_ok => (fl!("settings-check-windivert-fallback"), None),
        (CheckId::Windivert, true) => (fl!("settings-check-windivert-used"), None),
        (CheckId::Windivert, false) if check.fix == Some(Fix::PlaceWinDivertFiles) => {
            (fl!("settings-check-windivert-missing"), None)
        }
        (CheckId::Npcap | CheckId::Windivert, false) => (fl!("settings-check-unavailable"), Some(check.detail.clone())),
    };
    CheckLine { text: fl!("settings-check-line", label = label(check.id), status = status), raw_detail }
}

fn label(id: CheckId) -> String {
    match id {
        CheckId::Elevation => fl!("settings-check-elevation"),
        CheckId::Npcap => "Npcap".to_string(),
        CheckId::Windivert => "WinDivert".to_string(),
    }
}

/// The core words a working Npcap as "… N network adapters visible."
fn adapter_count(detail: &str) -> Option<usize> {
    detail.split_whitespace().find_map(|word| word.parse().ok())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::i18n::{set_language, Language};

    fn check(id: CheckId, ok: bool, weight: Weight, detail: &str) -> Check {
        Check { id, label: "", detail: detail.to_string(), ok, weight, fix: None }
    }

    #[test]
    fn a_report_is_worded_in_the_language_and_keeps_only_driver_errors_raw() {
        set_language(Language::French);
        let report = Report {
            ready: false,
            summary: "Administrator rights has to be sorted out before the meter can run.".to_string(),
            checks: vec![
                check(CheckId::Elevation, false, Weight::Required, "The meter is not running as Administrator."),
                check(CheckId::Npcap, true, Weight::Optional, "Installed and answering -- 1 network adapter visible."),
                check(CheckId::Windivert, false, Weight::Optional, "Access is denied (os error 5)"),
            ],
        };

        assert_eq!(summary(&report), "À régler avant que le compteur puisse tourner\u{a0}: Droits administrateur.");
        let lines: Vec<CheckLine> = report.checks.iter().map(|check| line(check, &report)).collect();
        assert!(lines[0].text.starts_with("Droits administrateur\u{a0}: "), "{}", lines[0].text);
        assert_eq!(lines[0].raw_detail, None);
        assert_eq!(lines[1].text, "Npcap\u{a0}: Installé et opérationnel, 1 carte réseau visible.");
        assert_eq!(lines[2].text, "WinDivert\u{a0}: Indisponible.");
        assert_eq!(lines[2].raw_detail.as_deref(), Some("Access is denied (os error 5)"));
    }

    #[test]
    fn absent_windivert_files_are_a_footnote_with_npcap_and_a_hint_without() {
        set_language(Language::English);
        let absent = |ok| Check { fix: Some(Fix::PlaceWinDivertFiles), ..check(CheckId::Windivert, ok, Weight::Optional, "") };
        let with_npcap = Report {
            ready: true,
            summary: String::new(),
            checks: vec![check(CheckId::Npcap, true, Weight::Optional, "1 adapter"), absent(true)],
        };
        assert!(line(&with_npcap.checks[1], &with_npcap).text.contains("fine while Npcap works"));

        let without_npcap = Report {
            ready: false,
            summary: String::new(),
            checks: vec![check(CheckId::Npcap, false, Weight::Required, "gone"), absent(false)],
        };
        let hint = line(&without_npcap.checks[1], &without_npcap);
        assert!(hint.text.contains("put WinDivert.dll and WinDivert64.sys beside the meter"), "{}", hint.text);
        assert_eq!(hint.raw_detail, None);
    }
}
