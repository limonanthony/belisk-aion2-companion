use std::collections::{BTreeMap, HashSet};
use std::path::PathBuf;
use std::process::Command;

use serde_json::Value;

use crate::{Result, WINDOWS_TARGET};

pub struct Metadata {
    pub version: String,
    pub target_directory: PathBuf,
    pub shipped_crates: Vec<ShippedCrate>,
}

pub struct ShippedCrate {
    pub name: String,
    pub version: String,
    pub license: Option<String>,
    pub repository: Option<String>,
    pub directory: PathBuf,
}

const APP_PACKAGE: &str = "aion-2-dps-meter";

impl Metadata {
    pub fn read(root: &std::path::Path) -> Result<Self> {
        let output = Command::new(std::env::var_os("CARGO").unwrap_or_else(|| "cargo".into()))
            .args(["metadata", "--format-version", "1", "--locked", "--offline"])
            .args(["--filter-platform", WINDOWS_TARGET])
            .current_dir(root)
            .output()
            .map_err(|error| format!("could not run cargo metadata: {error}"))?;
        if !output.status.success() {
            return Err(format!(
                "cargo metadata failed (offline, so every dependency must already be downloaded):\n{}",
                String::from_utf8_lossy(&output.stderr)
            )
            .into());
        }
        let metadata: Value = serde_json::from_slice(&output.stdout)?;
        let packages = metadata["packages"].as_array().ok_or("cargo metadata has no packages")?;
        let app_id = packages
            .iter()
            .find(|package| package["name"] == APP_PACKAGE)
            .ok_or_else(|| format!("package {APP_PACKAGE} not found in the workspace"))?["id"]
            .as_str()
            .ok_or("package id is not a string")?
            .to_owned();
        let version = package_by_id(packages, &app_id)["version"]
            .as_str()
            .ok_or("app version is not a string")?
            .to_owned();
        let workspace_members: HashSet<&str> = metadata["workspace_members"]
            .as_array()
            .ok_or("cargo metadata has no workspace_members")?
            .iter()
            .filter_map(Value::as_str)
            .collect();

        let reachable = runtime_and_build_dependencies(&metadata, &app_id)?;
        let mut shipped: BTreeMap<(String, String), ShippedCrate> = BTreeMap::new();
        for id in reachable.iter().filter(|id| !workspace_members.contains(id.as_str())) {
            let package = package_by_id(packages, id);
            let name = package["name"].as_str().unwrap_or_default().to_owned();
            let version = package["version"].as_str().unwrap_or_default().to_owned();
            let manifest = package["manifest_path"].as_str().ok_or("package without manifest_path")?;
            let directory = PathBuf::from(manifest)
                .parent()
                .ok_or("manifest_path has no parent")?
                .to_path_buf();
            shipped.insert(
                (name.clone(), version.clone()),
                ShippedCrate {
                    name,
                    version,
                    license: package["license"].as_str().map(str::to_owned),
                    repository: package["repository"].as_str().map(str::to_owned),
                    directory,
                },
            );
        }

        Ok(Self {
            version,
            target_directory: PathBuf::from(
                metadata["target_directory"].as_str().ok_or("cargo metadata has no target_directory")?,
            ),
            shipped_crates: shipped.into_values().collect(),
        })
    }

    pub fn crate_named(&self, name: &str) -> Result<&ShippedCrate> {
        self.shipped_crates
            .iter()
            .find(|candidate| candidate.name == name)
            .ok_or_else(|| format!("crate {name} is not in the dependency graph").into())
    }
}

fn package_by_id<'a>(packages: &'a [Value], id: &str) -> &'a Value {
    packages
        .iter()
        .find(|package| package["id"] == id)
        .unwrap_or(&Value::Null)
}

fn runtime_and_build_dependencies(metadata: &Value, root_id: &str) -> Result<HashSet<String>> {
    let nodes = metadata["resolve"]["nodes"].as_array().ok_or("cargo metadata has no resolve graph")?;
    let mut seen: HashSet<String> = HashSet::from([root_id.to_owned()]);
    let mut pending = vec![root_id.to_owned()];
    while let Some(id) = pending.pop() {
        let Some(node) = nodes.iter().find(|node| node["id"] == id.as_str()) else {
            continue;
        };
        for dependency in node["deps"].as_array().into_iter().flatten() {
            let is_dev_only = dependency["dep_kinds"]
                .as_array()
                .is_some_and(|kinds| !kinds.is_empty() && kinds.iter().all(|kind| kind["kind"] == "dev"));
            let dependency_id = dependency["pkg"].as_str().ok_or("dependency without pkg")?;
            if !is_dev_only && seen.insert(dependency_id.to_owned()) {
                pending.push(dependency_id.to_owned());
            }
        }
    }
    Ok(seen)
}
