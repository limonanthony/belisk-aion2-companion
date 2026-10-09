use std::path::PathBuf;

#[cfg(feature = "probes")]
use crate::i18n::LanguageChoice;

pub struct Args {
    pub replay: Option<PathBuf>,
    pub speed: f64,
    pub exit_after_replay: bool,
    #[cfg(feature = "probes")]
    pub probe: Option<Probe>,
    /// Where `--capture` writes its images.
    #[cfg(feature = "probes")]
    pub capture: Option<PathBuf>,
    #[cfg(feature = "probes")]
    pub fixtures: bool,
    /// The language `--capture` draws in; Windows' own when not given.
    #[cfg(feature = "probes")]
    pub language: Option<LanguageChoice>,
}

#[cfg(feature = "probes")]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Probe {
    Overlay,
    Font,
}

#[cfg(not(feature = "probes"))]
pub const USAGE: &str = "usage: belisk [--replay <recording> [--speed <factor>] [--exit-after-replay]]";

#[cfg(feature = "probes")]
pub const USAGE: &str = "usage: belisk [--replay <recording> [--speed <factor>] [--exit-after-replay]]\n\
       belisk --overlay-probe [--replay <recording>]\n\
       belisk --font-probe [--capture <dir>]\n\
       belisk --capture <dir> (--fixtures | --replay <recording> [--speed <factor>]) [--language <english|french>]";

impl Args {
    /// Without a recording, and not one of the development checks, the app
    /// captures the game's traffic.
    pub fn wants_live(&self) -> bool {
        #[cfg(feature = "probes")]
        if self.probe.is_some() || self.capture.is_some() {
            return false;
        }
        self.replay.is_none()
    }

    pub fn parse(args: impl IntoIterator<Item = String>) -> Result<Self, String> {
        let mut parsed = Self {
            replay: None,
            speed: 1.0,
            exit_after_replay: false,
            #[cfg(feature = "probes")]
            probe: None,
            #[cfg(feature = "probes")]
            capture: None,
            #[cfg(feature = "probes")]
            fixtures: false,
            #[cfg(feature = "probes")]
            language: None,
        };
        let mut args = args.into_iter();
        while let Some(arg) = args.next() {
            match arg.as_str() {
                "--replay" => {
                    let path = args.next().ok_or("--replay needs a recording path")?;
                    parsed.replay = Some(PathBuf::from(path));
                }
                "--speed" => {
                    let value = args.next().ok_or("--speed needs a number")?;
                    parsed.speed = value
                        .parse()
                        .map_err(|_| format!("--speed expects a number, got {value:?}"))?;
                }
                "--exit-after-replay" => parsed.exit_after_replay = true,
                #[cfg(feature = "probes")]
                "--overlay-probe" => parsed.probe = Some(Probe::Overlay),
                #[cfg(feature = "probes")]
                "--font-probe" => parsed.probe = Some(Probe::Font),
                #[cfg(feature = "probes")]
                "--capture" => {
                    let dir = args.next().ok_or("--capture needs a directory")?;
                    parsed.capture = Some(PathBuf::from(dir));
                }
                #[cfg(feature = "probes")]
                "--fixtures" => parsed.fixtures = true,
                #[cfg(feature = "probes")]
                "--language" => {
                    let value = args.next().ok_or("--language needs english or french")?;
                    parsed.language = Some(match value.as_str() {
                        "english" => LanguageChoice::English,
                        "french" => LanguageChoice::French,
                        other => return Err(format!("--language expects english or french, got {other:?}")),
                    });
                }
                other => return Err(format!("unknown argument {other:?}")),
            }
        }
        if parsed.exit_after_replay && parsed.replay.is_none() {
            return Err("--exit-after-replay needs --replay".to_string());
        }
        #[cfg(feature = "probes")]
        {
            let font_specimen = parsed.probe == Some(Probe::Font);
            if parsed.language.is_some() && parsed.capture.is_none() {
                return Err("--language needs --capture".to_string());
            }
            match (parsed.capture.is_some(), parsed.fixtures, parsed.replay.is_some()) {
                (false, true, _) => return Err("--fixtures needs --capture".to_string()),
                (true, false, false) if !font_specimen => {
                    return Err("--capture needs --fixtures, --replay or --font-probe".to_string())
                }
                (true, true, true) => {
                    return Err("--capture takes --fixtures or --replay, not both".to_string())
                }
                _ => {}
            }
        }
        Ok(parsed)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(args: &[&str]) -> Result<Args, String> {
        Args::parse(args.iter().map(|arg| arg.to_string()))
    }

    #[test]
    fn replay_options_are_read() {
        let args = parse(&["--replay", "a.aetherpc", "--speed", "2.5", "--exit-after-replay"])
            .unwrap();
        assert_eq!(args.replay, Some(PathBuf::from("a.aetherpc")));
        assert_eq!(args.speed, 2.5);
        assert!(args.exit_after_replay);
    }

    #[test]
    fn no_recording_means_live_capture() {
        assert!(parse(&[]).unwrap().wants_live());
        assert!(!parse(&["--replay", "a.aetherpc"]).unwrap().wants_live());
    }

    #[cfg(feature = "probes")]
    #[test]
    fn capture_and_probe_runs_are_never_live() {
        assert!(!parse(&["--capture", "shots", "--fixtures"]).unwrap().wants_live());
        assert!(!parse(&["--capture", "shots", "--replay", "a.aetherpc"]).unwrap().wants_live());
        assert!(!parse(&["--overlay-probe"]).unwrap().wants_live());
        assert!(!parse(&["--font-probe"]).unwrap().wants_live());
    }

    #[test]
    fn speed_defaults_to_real_time() {
        assert_eq!(parse(&[]).unwrap().speed, 1.0);
    }

    #[test]
    fn exiting_after_a_replay_needs_a_replay() {
        assert!(parse(&["--exit-after-replay"]).is_err());
    }

    #[test]
    fn unknown_and_incomplete_arguments_are_refused() {
        assert!(parse(&["--bogus"]).is_err());
        assert!(parse(&["--collect-mobs"]).is_err(), "now a switch in Settings");
        assert!(parse(&["--debug-log"]).is_err(), "now a switch in Settings");
        assert!(parse(&["--replay"]).is_err());
        assert!(parse(&["--speed", "fast"]).is_err());
    }
}
