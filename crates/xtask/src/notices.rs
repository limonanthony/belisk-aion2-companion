use std::fs;
use std::path::Path;

use crate::crates::Metadata;
use crate::{spdx, Result};

const DIVIDER: &str = "==============================================================================";

pub fn third_party_notices(root: &Path, metadata: &Metadata, source_statement: &str) -> Result<String> {
    let mut text = String::from(include_str!("../dist/notices-header.txt")).replace("{source_statement}", source_statement);
    let gpui = metadata.crate_named("gpui-pre")?;
    for (title, path) in [
        ("WinDivert 2.2.2 licence", root.join("crates/app/windivert/LICENSE")),
        ("Barlow Semi Condensed licence", root.join("crates/app/assets/fonts/OFL.txt")),
        ("Lucide licence", root.join("crates/app/assets/icons/LICENSE")),
        ("embed-resource licence", root.join("vendor/embed-resource/LICENSE")),
        ("GPUI (Zed) licence, Apache-2.0", gpui.directory.join("LICENSE-APACHE")),
    ] {
        let licence = fs::read_to_string(&path).map_err(|error| format!("cannot read {}: {error}", path.display()))?;
        text.push_str(&format!("\n{DIVIDER}\n{title}\n{DIVIDER}\n\n{}\n", licence.trim_end()));
    }

    text.push_str(&format!(
        "\n{DIVIDER}\nRust crates linked into the executable\n{DIVIDER}\n\n\
         Generated from `cargo metadata` for the Windows build ({} crates). Each crate is under the licence\n\
         shown and ships with its own copyright notice in its source.\n\n",
        metadata.shipped_crates.len()
    ));
    let mut needs_review = Vec::new();
    for shipped in &metadata.shipped_crates {
        let license = shipped.license.as_deref().unwrap_or("(no licence declared)");
        text.push_str(&format!("{} {} - {license}", shipped.name, shipped.version));
        if let Some(repository) = &shipped.repository {
            text.push_str(&format!(" - {repository}"));
        }
        text.push('\n');
        if !shipped.license.as_deref().is_some_and(spdx::is_gpl3_compatible) {
            needs_review.push(format!("{} {} - {license}", shipped.name, shipped.version));
        }
    }

    if needs_review.is_empty() {
        println!("check: all {} crate licences are GPL-3.0 compatible", metadata.shipped_crates.len());
    } else {
        println!("WARNING: crates whose licence is not on the GPL-3.0-compatible list (review before sending):");
        for line in &needs_review {
            println!("  {line}");
        }
        text.push_str("\nLicences that need review for GPL-3.0 compatibility:\n");
        for line in &needs_review {
            text.push_str(&format!("  {line}\n"));
        }
    }
    Ok(text)
}
