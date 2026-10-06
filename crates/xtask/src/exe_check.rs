use object::pe::{IMAGE_SUBSYSTEM_WINDOWS_GUI, RT_MANIFEST};
use object::read::pe::{
    ImageNtHeaders, ImageOptionalHeader, PeFile64, ResourceDirectory, ResourceDirectoryEntryData,
    ResourceDirectoryTable, ResourceNameOrId,
};
use object::LittleEndian as LE;

use crate::Result;

const C_RUNTIME_DLL_PREFIXES: [&str; 7] = [
    "vcruntime",
    "msvcp",
    "concrt",
    "vccorlib",
    "ucrtbase",
    "api-ms-win-crt-",
    "msvcr",
];
const WINDIVERT_DLL: &str = "WinDivert.dll";

pub fn verify_shipping_exe(bytes: &[u8]) -> Result<()> {
    let exe = PeFile64::parse(bytes).map_err(|error| format!("the exe is not a 64-bit PE file: {error}"))?;
    let imports = imported_dlls(&exe)?;
    println!("check: imports {} DLLs: {}", imports.len(), imports.join(", "));
    let runtime_imports: Vec<&String> = imports
        .iter()
        .filter(|dll| {
            let lower = dll.to_ascii_lowercase();
            C_RUNTIME_DLL_PREFIXES.iter().any(|prefix| lower.starts_with(prefix))
        })
        .collect();
    if !runtime_imports.is_empty() {
        return Err(format!(
            "the exe imports the C runtime ({runtime_imports:?}); the static CRT did not take effect, so the \
             friend would need the Visual C++ redistributable"
        )
        .into());
    }
    println!("check: no VC++ runtime or Universal CRT imports (static CRT)");

    let delay_loaded = delay_loaded_dlls(&exe)?;
    verify_windivert_is_delay_loaded(&imports, &delay_loaded)?;
    println!("check: {WINDIVERT_DLL} is delay-loaded, not a load-time import (delay-loads: {})", delay_loaded.join(", "));

    let subsystem = exe.nt_headers().optional_header().subsystem();
    if subsystem != IMAGE_SUBSYSTEM_WINDOWS_GUI {
        return Err(format!("the exe subsystem is {subsystem}, expected {IMAGE_SUBSYSTEM_WINDOWS_GUI} (GUI)").into());
    }
    println!("check: subsystem = {subsystem} (Windows GUI, no console)");

    let manifest = manifest(&exe)?;
    if !manifest.contains("requireAdministrator") {
        return Err("the embedded RT_MANIFEST does not request requireAdministrator".into());
    }
    println!("check: RT_MANIFEST embeds requireAdministrator ({} bytes)", manifest.len());
    Ok(())
}

fn imported_dlls(exe: &PeFile64) -> Result<Vec<String>> {
    let sections = exe.section_table();
    let Some(table) = exe.data_directories().import_table(exe.data(), &sections)? else {
        return Ok(Vec::new());
    };
    let mut names = Vec::new();
    let mut descriptors = table.descriptors()?;
    while let Some(descriptor) = descriptors.next()? {
        names.push(String::from_utf8_lossy(table.name(descriptor.name.get(LE))?).into_owned());
    }
    Ok(names)
}

fn delay_loaded_dlls(exe: &PeFile64) -> Result<Vec<String>> {
    let sections = exe.section_table();
    let Some(table) = exe.data_directories().delay_load_import_table(exe.data(), &sections)? else {
        return Ok(Vec::new());
    };
    let mut names = Vec::new();
    let mut descriptors = table.descriptors()?;
    while let Some(descriptor) = descriptors.next()? {
        names.push(String::from_utf8_lossy(table.name(descriptor.dll_name_rva.get(LE))?).into_owned());
    }
    Ok(names)
}

fn verify_windivert_is_delay_loaded(imports: &[String], delay_loaded: &[String]) -> Result<()> {
    let named = |names: &[String]| names.iter().any(|name| name.eq_ignore_ascii_case(WINDIVERT_DLL));
    if named(imports) {
        return Err(format!(
            "the exe imports {WINDIVERT_DLL} at load time, so Windows refuses to start it without the DLL even \
             when Npcap works; link it with /DELAYLOAD:{WINDIVERT_DLL}"
        )
        .into());
    }
    if !named(delay_loaded) {
        return Err(format!("the exe does not delay-load {WINDIVERT_DLL}; the WinDivert fallback cannot work").into());
    }
    Ok(())
}

fn manifest(exe: &PeFile64) -> Result<String> {
    let sections = exe.section_table();
    let resources = exe
        .data_directories()
        .resource_directory(exe.data(), &sections)?
        .ok_or("the exe has no resource section, so no manifest is embedded")?;
    let root = resources.root()?;
    let manifest_entry = root
        .entries
        .iter()
        .find(|entry| matches!(entry.name_or_id(), ResourceNameOrId::Id(RT_MANIFEST)))
        .ok_or("the exe has no RT_MANIFEST resource")?;
    let data_entry = match manifest_entry.data(resources)? {
        ResourceDirectoryEntryData::Table(table) => first_data_entry(resources, table)?,
        ResourceDirectoryEntryData::Data(data) => *data,
    };
    let rva = data_entry.offset_to_data.get(LE);
    let size = data_entry.size.get(LE) as usize;
    let bytes = sections
        .pe_data_at(exe.data(), rva)
        .and_then(|data| data.get(..size))
        .ok_or("the RT_MANIFEST resource points outside the file")?;
    Ok(String::from_utf8_lossy(bytes).into_owned())
}

fn first_data_entry(
    resources: ResourceDirectory,
    table: ResourceDirectoryTable,
) -> Result<object::pe::ImageResourceDataEntry> {
    let entry = table.entries.first().ok_or("the RT_MANIFEST resource is empty")?;
    match entry.data(resources)? {
        ResourceDirectoryEntryData::Table(inner) => first_data_entry(resources, inner),
        ResourceDirectoryEntryData::Data(data) => Ok(*data),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn names(list: &[&str]) -> Vec<String> {
        list.iter().map(|name| name.to_string()).collect()
    }

    #[test]
    fn windivert_must_be_delay_loaded() {
        assert!(verify_windivert_is_delay_loaded(&names(&["KERNEL32.dll"]), &names(&["WinDivert.dll"])).is_ok());
    }

    #[test]
    fn a_load_time_windivert_import_is_refused_even_when_also_delay_loaded() {
        let error = verify_windivert_is_delay_loaded(&names(&["windivert.dll"]), &names(&["WinDivert.dll"]))
            .unwrap_err()
            .to_string();
        assert!(error.contains("/DELAYLOAD"), "{error}");
    }

    #[test]
    fn an_exe_without_the_windivert_delay_load_is_refused() {
        assert!(verify_windivert_is_delay_loaded(&names(&["KERNEL32.dll"]), &[]).is_err());
    }
}
