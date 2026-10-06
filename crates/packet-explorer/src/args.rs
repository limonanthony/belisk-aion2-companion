use std::collections::HashMap;
use std::path::PathBuf;
use std::str::FromStr;

use crate::capture::{labels::format_opcode, Direction, FileInfo, Packet};
use crate::clock::TimeSpec;

const BOOLEAN_FLAGS: &[&str] = &["json", "show-text", "all", "c2s-sizes", "help"];
const RECORDINGS_SUBPATH: &str = "com.helveticxa.aether/recordings";

pub struct Args {
    pub command: String,
    pub positional: Vec<String>,
    options: HashMap<String, Vec<String>>,
}

impl Args {
    pub fn parse(raw: impl IntoIterator<Item = String>) -> Result<Args, String> {
        let mut raw = raw.into_iter();
        let command = raw.next().unwrap_or_else(|| "help".to_string());
        let mut args = Args { command, positional: Vec::new(), options: HashMap::new() };
        while let Some(token) = raw.next() {
            let Some(name) = token.strip_prefix("--") else {
                args.positional.push(token);
                continue;
            };
            let (name, inline_value) = match name.split_once('=') {
                Some((name, value)) => (name, Some(value.to_string())),
                None => (name, None),
            };
            let value = if BOOLEAN_FLAGS.contains(&name) {
                inline_value.unwrap_or_default()
            } else {
                match inline_value.or_else(|| raw.next()) {
                    Some(value) => value,
                    None => return Err(format!("--{name} needs a value")),
                }
            };
            args.options.entry(name.to_string()).or_default().push(value);
        }
        Ok(args)
    }

    pub fn flag(&self, name: &str) -> bool {
        self.options.contains_key(name)
    }

    pub fn value(&self, name: &str) -> Option<&str> {
        self.options.get(name)?.last().map(String::as_str)
    }

    pub fn values(&self, name: &str) -> &[String] {
        self.options.get(name).map_or(&[], Vec::as_slice)
    }

    pub fn parsed<T: FromStr>(&self, name: &str) -> Result<Option<T>, String> {
        self.value(name)
            .map(|text| text.parse().map_err(|_| format!("--{name}: cannot read '{text}'")))
            .transpose()
    }

    pub fn parsed_or<T: FromStr>(&self, name: &str, default: T) -> Result<T, String> {
        Ok(self.parsed(name)?.unwrap_or(default))
    }

    pub fn input_files(&self) -> Result<Vec<PathBuf>, String> {
        if self.positional.is_empty() {
            return Err("name at least one recording (a path, a folder, or part of a file name in the recordings folder)".to_string());
        }
        let mut files = Vec::new();
        for argument in &self.positional {
            files.extend(resolve_input(argument)?);
        }
        Ok(files)
    }
}

fn recordings_dir() -> Option<PathBuf> {
    std::env::var_os("APPDATA").map(|appdata| PathBuf::from(appdata).join(RECORDINGS_SUBPATH))
}

fn recordings_in(dir: &std::path::Path) -> Vec<PathBuf> {
    let mut files: Vec<PathBuf> = std::fs::read_dir(dir)
        .into_iter()
        .flatten()
        .flatten()
        .map(|entry| entry.path())
        .filter(|path| path.extension().is_some_and(|extension| extension == "aetherpc"))
        .collect();
    files.sort();
    files
}

fn resolve_input(argument: &str) -> Result<Vec<PathBuf>, String> {
    let path = PathBuf::from(argument);
    if path.is_dir() {
        return Ok(recordings_in(&path));
    }
    if path.is_file() {
        return Ok(vec![path]);
    }
    let matches: Vec<PathBuf> = recordings_dir()
        .map(|dir| recordings_in(&dir))
        .unwrap_or_default()
        .into_iter()
        .filter(|candidate| {
            candidate
                .file_name()
                .is_some_and(|name| name.to_string_lossy().contains(argument))
        })
        .collect();
    if matches.is_empty() {
        return Err(format!("'{argument}' is not a file, a folder, or part of a recording name in the recordings folder"));
    }
    Ok(matches)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OpSel {
    Opcode([u8; 2]),
    FirstByte(u8),
    ClientToServer,
}

impl FromStr for OpSel {
    type Err = String;

    fn from_str(text: &str) -> Result<Self, String> {
        let compact: String = text.chars().filter(|c| !c.is_whitespace()).collect::<String>().to_lowercase();
        if compact == "c2s" {
            return Ok(OpSel::ClientToServer);
        }
        let byte = |digits: &str| u8::from_str_radix(digits, 16).map_err(|_| format!("'{text}' is not an opcode"));
        match compact.len() {
            2 => Ok(OpSel::FirstByte(byte(&compact)?)),
            4 => Ok(OpSel::Opcode([byte(&compact[..2])?, byte(&compact[2..])?])),
            _ => Err(format!("'{text}' is not an opcode: use 4 hex digits (0be3), 2 for a first byte, or c2s")),
        }
    }
}

impl OpSel {
    fn matches(self, packet: &Packet) -> bool {
        match (self, packet.opcode()) {
            (OpSel::ClientToServer, _) => packet.dir == Direction::ClientToServer,
            (OpSel::Opcode(want), Some(have)) => want == have,
            (OpSel::FirstByte(want), Some(have)) => want == have[0],
            _ => false,
        }
    }
}

pub fn describe_opcode(packet: &Packet) -> String {
    packet.opcode().map_or_else(|| "c2s".to_string(), format_opcode)
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum DirectionFilter {
    ServerOnly,
    Both,
    ClientOnly,
}

pub struct Filter {
    ops: Vec<OpSel>,
    direction: DirectionFilter,
    from: Option<TimeSpec>,
    to: Option<TimeSpec>,
}

impl Filter {
    pub fn from_args(args: &Args, default_direction: DirectionFilter) -> Result<Filter, String> {
        let mut ops = Vec::new();
        for list in args.values("op") {
            for item in list.split(',').filter(|item| !item.trim().is_empty()) {
                ops.push(item.parse()?);
            }
        }
        let direction = match args.value("dir") {
            None => {
                if ops.contains(&OpSel::ClientToServer) {
                    DirectionFilter::Both
                } else {
                    default_direction
                }
            }
            Some("s2c") => DirectionFilter::ServerOnly,
            Some("c2s") => DirectionFilter::ClientOnly,
            Some("both") => DirectionFilter::Both,
            Some(other) => return Err(format!("--dir: '{other}' is not s2c, c2s or both")),
        };
        Ok(Filter {
            ops,
            direction,
            from: args.parsed("from")?,
            to: args.parsed("to")?,
        })
    }

    pub fn bind<'a>(&'a self, files: &[FileInfo]) -> BoundFilter<'a> {
        let windows = files
            .iter()
            .map(|file| {
                (
                    self.from.map_or(f64::NEG_INFINITY, |spec| spec.resolve(file.start)),
                    self.to.map_or(f64::INFINITY, |spec| spec.resolve(file.start)),
                )
            })
            .collect();
        BoundFilter { filter: self, windows }
    }
}

pub struct BoundFilter<'a> {
    filter: &'a Filter,
    windows: Vec<(f64, f64)>,
}

impl BoundFilter<'_> {
    pub fn matches(&self, packet: &Packet) -> bool {
        let direction_ok = match self.filter.direction {
            DirectionFilter::Both => true,
            DirectionFilter::ServerOnly => packet.dir == Direction::ServerToClient,
            DirectionFilter::ClientOnly => packet.dir == Direction::ClientToServer,
        };
        let (from, to) = self.windows[packet.file];
        direction_ok
            && (from..=to).contains(&packet.t)
            && (self.filter.ops.is_empty() || self.filter.ops.iter().any(|op| op.matches(packet)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(tokens: &[&str]) -> Args {
        Args::parse(tokens.iter().map(|token| token.to_string())).unwrap()
    }

    #[test]
    fn options_take_values_and_flags_do_not() {
        let args = parse(&["dump", "rec1", "--op", "0be3,0143", "--json", "--limit=5", "rec2"]);
        assert_eq!(args.command, "dump");
        assert_eq!(args.positional, vec!["rec1", "rec2"]);
        assert!(args.flag("json"));
        assert_eq!(args.value("op"), Some("0be3,0143"));
        assert_eq!(args.parsed_or("limit", 0usize), Ok(5));
    }

    #[test]
    fn opcode_selectors_accept_spaced_pairs_first_bytes_and_c2s() {
        assert_eq!("33 36".parse(), Ok(OpSel::Opcode([0x33, 0x36])));
        assert_eq!("8D".parse(), Ok(OpSel::FirstByte(0x8d)));
        assert_eq!("C2S".parse(), Ok(OpSel::ClientToServer));
        assert!("123".parse::<OpSel>().is_err());
    }
}
