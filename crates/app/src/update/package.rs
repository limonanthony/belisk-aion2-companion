//! What an update archive may hold, and reading it. An archive is flat: the
//! exe and the two WinDivert files, by these exact names and no others. Nothing
//! the archive says about names, folders or sizes is taken as it is.

use std::io::{Cursor, Read};

use super::state::Failure;

/// The exe's name inside an archive; installed under the running exe's name.
pub const EXE_ENTRY: &str = "belisk.exe";
const ALLOWED: [&str; 3] = [EXE_ENTRY, "WinDivert.dll", "WinDivert64.sys"];
/// The names the update may write beside the exe, `.exe` aside.
pub const COMPANION_FILES: [&str; 2] = ["WinDivert.dll", "WinDivert64.sys"];

pub const MAX_ARCHIVE_BYTES: u64 = 64 * 1024 * 1024;
const MAX_UNPACKED_BYTES: u64 = 96 * 1024 * 1024;

pub struct Entry {
    /// One of the allowed names.
    pub name: &'static str,
    pub bytes: Vec<u8>,
}

/// Reads every file of a verified archive into memory, refusing the whole
/// archive on the first thing that is not allowed.
pub fn extract(archive: &[u8]) -> Result<Vec<Entry>, Failure> {
    read(archive).map_err(Failure::Stage)
}

fn read(archive: &[u8]) -> Result<Vec<Entry>, String> {
    let mut zip = zip::ZipArchive::new(Cursor::new(archive)).map_err(|error| format!("the archive is not a zip: {error}"))?;
    let mut entries: Vec<Entry> = Vec::new();
    let mut declared_total = 0u64;
    for index in 0..zip.len() {
        let mut file = zip.by_index(index).map_err(|error| format!("cannot read the archive: {error}"))?;
        let name = ALLOWED
            .iter()
            .copied()
            .find(|allowed| file.name() == *allowed)
            .ok_or_else(|| format!("the archive holds {:?}, which an update may not install", file.name()))?;
        if file.is_dir() || entries.iter().any(|entry| entry.name == name) {
            return Err(format!("the archive holds {name} more than once or as a folder"));
        }
        let size = file.size();
        declared_total = declared_total.saturating_add(size);
        if declared_total > MAX_UNPACKED_BYTES {
            return Err("the archive unpacks to more than 96 MiB".to_owned());
        }
        let mut bytes = Vec::new();
        (&mut file)
            .take(size + 1)
            .read_to_end(&mut bytes)
            .map_err(|error| format!("cannot unpack {name}: {error}"))?;
        if bytes.len() as u64 != size {
            return Err(format!("{name} is not the size the archive says"));
        }
        entries.push(Entry { name, bytes });
    }
    if !entries.iter().any(|entry| entry.name == EXE_ENTRY) {
        return Err(format!("the archive has no {EXE_ENTRY}"));
    }
    Ok(entries)
}

#[cfg(test)]
pub mod tests {
    use std::io::Write;

    use zip::write::SimpleFileOptions;

    use super::*;

    pub fn zip_of(files: &[(&str, &[u8])]) -> Vec<u8> {
        let mut bytes = Vec::new();
        {
            let mut zip = zip::ZipWriter::new(Cursor::new(&mut bytes));
            for (name, contents) in files {
                zip.start_file(*name, SimpleFileOptions::default()).unwrap();
                zip.write_all(contents).unwrap();
            }
            zip.finish().unwrap();
        }
        bytes
    }

    fn refused(files: &[(&str, &[u8])]) -> bool {
        matches!(extract(&zip_of(files)), Err(Failure::Stage(_)))
    }

    #[test]
    fn the_exe_and_the_driver_files_are_read_with_their_bytes() {
        let entries = extract(&zip_of(&[(EXE_ENTRY, b"exe"), ("WinDivert.dll", b"dll"), ("WinDivert64.sys", b"sys")])).unwrap();

        let by_name = |name: &str| entries.iter().find(|entry| entry.name == name).unwrap().bytes.clone();
        assert_eq!(by_name(EXE_ENTRY), b"exe");
        assert_eq!(by_name("WinDivert.dll"), b"dll");
        assert_eq!(by_name("WinDivert64.sys"), b"sys");
    }

    #[test]
    fn the_driver_files_are_optional_and_the_exe_is_not() {
        assert!(extract(&zip_of(&[(EXE_ENTRY, b"exe")])).is_ok());
        assert!(refused(&[("WinDivert.dll", b"dll")]));
        assert!(refused(&[]));
    }

    #[test]
    fn any_other_name_is_refused_whatever_it_looks_like() {
        for name in [
            "sub/WinDivert.dll",
            "../evil.dll",
            "..\\evil.dll",
            "C:evil.dll",
            "C:\\Windows\\System32\\sethc.exe",
            "/WinDivert.dll",
            "windivert.dll",
            "WinDivert.dll.",
            "WinDivert.dll ",
            "READY",
            "notes.txt",
            "other.exe",
            "belisk.exe.old",
        ] {
            assert!(refused(&[(EXE_ENTRY, b"exe"), (name, b"x")]), "{name:?}");
        }
    }

    #[test]
    fn a_folder_entry_is_refused() {
        assert!(refused(&[(EXE_ENTRY, b"exe"), ("WinDivert.dll/", b"")]));
    }

    #[test]
    fn a_name_twice_still_installs_one_file_by_that_name() {
        let mut archive = zip_of(&[(EXE_ENTRY, b"exe"), ("WinDivert.dll", b"one"), ("WinDivert.dlX", b"two")]);
        let (from, to) = (b"WinDivert.dlX", b"WinDivert.dll");
        let mut at = 0;
        while let Some(found) = archive[at..].windows(from.len()).position(|window| window == from) {
            archive[at + found..at + found + from.len()].copy_from_slice(to);
            at += found + from.len();
        }

        let entries = extract(&archive).unwrap();

        assert_eq!(entries.iter().filter(|entry| entry.name == "WinDivert.dll").count(), 1, "the zip reader keeps one entry per name");
    }

    #[test]
    fn something_that_is_not_a_zip_is_refused() {
        assert!(matches!(extract(b"MZ not a zip"), Err(Failure::Stage(_))));
    }

    #[test]
    fn an_archive_that_unpacks_past_the_cap_is_refused_before_it_is_read() {
        let big = vec![0u8; 50 * 1024 * 1024];

        assert!(refused(&[(EXE_ENTRY, &big), ("WinDivert64.sys", &big)]));
    }
}
