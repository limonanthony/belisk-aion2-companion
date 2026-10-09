//! The startup gate: everything that must be true before Belisk can measure.
//!
//! Two ideas drive this module.
//!
//! First, a backend that *exists* is not a backend that *works*. Loading
//! `wpcap.dll` keeps succeeding after the driver service is stopped or the
//! adapter list goes empty -- a check that stopped there would report capture
//! as ready when it can never see a packet. Here the probe talks to the driver.
//!
//! Second, not every failure deserves to stop anyone. Capture needs *one*
//! backend, not both, so WinDivert missing while Npcap works is a footnote, not
//! an alarm. [`Weight`] carries that distinction to the UI so the gate holds for
//! real problems and stays quiet about the rest.

use serde::Serialize;

use crate::capture::capturer;
use crate::capture::windivert_capturer;

/// Which check a row reports on. The frontend keys its icons and fix buttons
/// off this rather than off the label, so labels stay free to be reworded.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum CheckId {
    Elevation,
    Npcap,
    Windivert,
}

/// Whether a failing check stops startup.
///
/// Computed per report, not fixed per check: Npcap is required only while
/// WinDivert cannot stand in for it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum Weight {
    Required,
    Optional,
}

/// The remedy offered for a failing check. `None` means there is nothing the
/// app can usefully do on the operator's behalf.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum Fix {
    /// Download and launch the official Npcap installer.
    InstallNpcap,
    /// `WinDivert.dll` or `WinDivert64.sys` is missing beside the exe; the
    /// fallback needs both, and it is offered even on a passing check.
    PlaceWinDivertFiles,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Check {
    pub id: CheckId,
    pub label: &'static str,
    /// One sentence, always populated -- on success too, so the row says
    /// something more useful than "Available".
    pub detail: String,
    pub ok: bool,
    pub weight: Weight,
    pub fix: Option<Fix>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Report {
    /// True when nothing required is failing. The gate opens on this alone.
    pub ready: bool,
    /// One line for the header: what is being waited on, or that all is well.
    pub summary: String,
    pub checks: Vec<Check>,
}

/// True when every required check passes. Optional checks never gate.
fn is_ready(checks: &[Check]) -> bool {
    checks
        .iter()
        .all(|check| check.ok || check.weight == Weight::Optional)
}

fn summarise(checks: &[Check]) -> String {
    let blocking: Vec<&str> = checks
        .iter()
        .filter(|check| !check.ok && check.weight == Weight::Required)
        .map(|check| check.label)
        .collect();

    match blocking.as_slice() {
        [] => "Everything the meter needs is in place.".to_string(),
        [one] => format!("{one} has to be sorted out before the meter can run."),
        many => format!(
            "{} have to be sorted out before the meter can run.",
            many.join(" and ")
        ),
    }
}

/// Whether `WinDivert.dll` or `WinDivert64.sys` is absent from the folder of
/// the exe. The zip ships both, but only the fallback needs them.
fn windivert_files_missing() -> bool {
    let Some(dir) = std::env::current_exe().ok().and_then(|exe| exe.parent().map(|dir| dir.to_path_buf())) else {
        return false;
    };
    ["WinDivert.dll", "WinDivert64.sys"].iter().any(|name| !dir.join(name).exists())
}

/// Build the report. Pure assembly once the probes have run, so the weighting
/// rules stay testable on a machine with no capture driver at all.
fn assemble(
    elevated: bool,
    npcap: Result<usize, String>,
    windivert: Option<String>,
    files_missing: bool,
) -> Report {
    let windivert_ok = windivert.is_none();
    let npcap_ok = npcap.is_ok();

    let elevation = Check {
        id: CheckId::Elevation,
        label: "Administrator rights",
        detail: if elevated {
            "Running elevated, which packet capture requires.".to_string()
        } else {
            "The meter is not running as Administrator. Capture drivers cannot be opened without it \
             -- close the meter and start it again."
                .to_string()
        },
        ok: elevated,
        weight: Weight::Required,
        fix: None,
    };

    let npcap = Check {
        id: CheckId::Npcap,
        label: "Npcap",
        detail: match &npcap {
            Ok(count) => format!(
                "Installed and answering -- {count} network adapter{} visible.",
                if *count == 1 { "" } else { "s" }
            ),
            Err(error) => error.clone(),
        },
        ok: npcap_ok,
        // Preferred backend, but capture needs only one of the two.
        weight: if windivert_ok {
            Weight::Optional
        } else {
            Weight::Required
        },
        fix: if npcap_ok {
            None
        } else {
            Some(Fix::InstallNpcap)
        },
    };

    let windivert = Check {
        id: CheckId::Windivert,
        label: "WinDivert",
        detail: match &windivert {
            None if npcap_ok && files_missing => "Not beside the meter, which is fine while Npcap \
                                                  works. It is only a fallback."
                .to_string(),
            None if npcap_ok => "Bundled as a fallback. Its driver is loaded only if Npcap cannot \
                                 capture, and unloaded again when the meter is done with it."
                .to_string(),
            None => "Bundled driver answering, and used for capture.".to_string(),
            Some(error) if files_missing => format!(
                "{error}. Install Npcap, or put WinDivert.dll and WinDivert64.sys beside the meter."
            ),
            Some(error) => error.clone(),
        },
        ok: windivert_ok,
        // Always optional: it exists to cover Npcap, never to become a
        // requirement of its own. When both are down, Npcap carries the
        // blocking weight, because Npcap is the one Belisk can install.
        weight: Weight::Optional,
        fix: files_missing.then_some(Fix::PlaceWinDivertFiles),
    };

    let checks = vec![elevation, npcap, windivert];
    Report {
        ready: is_ready(&checks),
        summary: summarise(&checks),
        checks,
    }
}

/// `None` when the question could not be answered.
///
/// The distinction matters because this check can lock the user out: there is
/// no fix button for it, only Quit. A failed API call is this code's problem,
/// not the machine's, and the manifest already makes Windows refuse to launch
/// Belisk unelevated -- so an unanswerable query is treated as elevated. Only a
/// query that succeeds and says "no" holds the gate closed.
#[cfg(windows)]
fn query_elevated() -> Option<bool> {
    use windows::Win32::Foundation::CloseHandle;
    use windows::Win32::Security::{
        GetTokenInformation, TokenElevation, TOKEN_ELEVATION, TOKEN_QUERY,
    };
    use windows::Win32::System::Threading::{GetCurrentProcess, OpenProcessToken};

    unsafe {
        let mut token = Default::default();
        OpenProcessToken(GetCurrentProcess(), TOKEN_QUERY, &mut token).ok()?;

        let mut elevation = TOKEN_ELEVATION::default();
        let mut returned = 0u32;
        let queried = GetTokenInformation(
            token,
            TokenElevation,
            Some(&mut elevation as *mut _ as *mut std::ffi::c_void),
            std::mem::size_of::<TOKEN_ELEVATION>() as u32,
            &mut returned,
        )
        .is_ok();
        let _ = CloseHandle(token);

        queried.then(|| elevation.TokenIsElevated != 0)
    }
}

#[cfg(not(windows))]
fn query_elevated() -> Option<bool> {
    Some(true)
}

/// Named and separate so the fail-open decision above is visible to anyone
/// tempted to "tighten" it into a lock-out.
fn resolve_elevation(queried: Option<bool>) -> bool {
    queried.unwrap_or(true)
}

fn is_elevated() -> bool {
    resolve_elevation(query_elevated())
}

/// Run every check and weigh the results.
///
/// WinDivert is only opened when Npcap fails. Opening it loads a kernel
/// driver, and a loaded WinDivert is something some anti-cheats frown on, so
/// with Npcap working the fallback is never touched, not even its files.
pub fn run() -> Report {
    let npcap = capturer::probe_npcap();
    let files_missing = windivert_files_missing();
    let windivert = if npcap.is_ok() {
        None
    } else if files_missing {
        Some("WinDivert.dll or WinDivert64.sys was not found".to_string())
    } else {
        let status = windivert_capturer::check_windivert_status();
        (!status.available).then(|| {
            status
                .error
                .unwrap_or_else(|| "WinDivert could not be opened".to_string())
        })
    };

    assemble(is_elevated(), npcap, windivert, files_missing)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn failing(report: &Report, weight: Weight) -> Vec<CheckId> {
        report
            .checks
            .iter()
            .filter(|check| check.weight == weight && !check.ok)
            .map(|check| check.id)
            .collect()
    }

    #[test]
    fn absent_windivert_files_do_not_gate_when_npcap_works() {
        // The case that prompted this module: Npcap fine, WinDivert absent, and
        // the old screen put an amber warning next to "starting...".
        let report = assemble(
            true,
            Ok(4),
            None,
            true,
        );

        assert!(report.ready);
        assert!(failing(&report, Weight::Required).is_empty());
        assert!(failing(&report, Weight::Optional).is_empty());
    }

    #[test]
    fn losing_both_backends_gates_on_npcap() {
        let report = assemble(
            true,
            Err("wpcap.dll not found".into()),
            Some("no driver".into()),
            false,
        );

        assert!(!report.ready);
        assert_eq!(failing(&report, Weight::Required), vec![CheckId::Npcap]);
        assert_eq!(
            report.checks[1].fix,
            Some(Fix::InstallNpcap),
            "the blocking row must offer the one fix Belisk can perform"
        );
    }

    #[test]
    fn windivert_alone_is_enough_to_start() {
        let report = assemble(true, Err("wpcap.dll not found".into()), None, false);

        assert!(report.ready, "one working backend is the whole requirement");
    }

    #[test]
    fn a_standby_windivert_says_it_is_not_loaded() {
        let report = assemble(true, Ok(2), None, false);
        assert!(report.checks[2].ok);
        assert!(report.checks[2].detail.contains("only if Npcap"));

        let fallback = assemble(true, Err("gone".into()), None, false);
        assert!(fallback.checks[2].detail.contains("used for capture"));
    }

    #[test]
    fn losing_elevation_gates_on_its_own() {
        let report = assemble(false, Ok(4), None, false);

        assert!(!report.ready);
        assert_eq!(failing(&report, Weight::Required), vec![CheckId::Elevation]);
    }

    #[test]
    fn absent_windivert_files_are_only_a_footnote_while_npcap_works() {
        let report = assemble(true, Ok(1), None, true);

        assert!(report.ready);
        assert!(report.checks[2].ok, "a missing fallback is not an error while Npcap works");
        assert_eq!(report.checks[2].fix, Some(Fix::PlaceWinDivertFiles));
        assert!(report.checks[2].detail.contains("Npcap works"));
    }

    #[test]
    fn a_driver_present_but_blocked_offers_no_reinstall() {
        // Error 1275, say: the file is there, so putting it back changes nothing.
        let report = assemble(true, Ok(1), Some("blocked by security software".into()), false);

        assert_eq!(report.checks[2].fix, None);
        assert!(!report.checks[2].detail.contains("antivirus"));
    }

    #[test]
    fn an_unanswerable_elevation_query_does_not_lock_anyone_out() {
        // This check has no fix button, so a false negative traps the user on a
        // screen whose only other control is Quit.
        assert!(resolve_elevation(None));
        assert!(resolve_elevation(Some(true)));
        assert!(!resolve_elevation(Some(false)));
    }

    #[test]
    fn the_summary_names_what_is_blocking() {
        let ready = assemble(true, Ok(2), None, false);
        assert!(ready.summary.contains("in place"));

        let blocked = assemble(false, Err("gone".into()), Some("gone".into()), false);
        assert!(blocked.summary.contains("Administrator rights"));
        assert!(blocked.summary.contains("Npcap"));
        assert!(
            !blocked.summary.contains("WinDivert"),
            "optional failures never appear in the blocking summary"
        );
    }
}
