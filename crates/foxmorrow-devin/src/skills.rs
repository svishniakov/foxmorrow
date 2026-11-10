use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::process::Command;

use serde::{Deserialize, Serialize};

const DESKTOP_VARIABLES: [&str; 4] = [
    "WINDSURF_IDE_TYPE",
    "WINDSURF_EXT_HOST_PID",
    "DEVIN_FD_PATH",
    "WINDSURF_CODEIUM_DIR",
];

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum InventorySource {
    Devin,
    Approximate,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Inventory {
    pub source: InventorySource,
    pub names: BTreeSet<String>,
}

#[derive(Deserialize)]
struct Listed {
    name: String,
    #[serde(default)]
    errors: Vec<serde_json::Value>,
}

impl Inventory {
    pub fn missing(&self, wanted: &[String]) -> Vec<String> {
        wanted
            .iter()
            .filter(|name| !self.names.contains(*name))
            .cloned()
            .collect()
    }
}

pub fn parse_devin_list(json: &str) -> Option<BTreeSet<String>> {
    let listed: Vec<Listed> = serde_json::from_str(json).ok()?;
    Some(
        listed
            .into_iter()
            .filter(|skill| skill.errors.is_empty())
            .map(|skill| skill.name)
            .collect(),
    )
}

pub fn default_dirs(project_root: &Path, home: &Path) -> Vec<PathBuf> {
    vec![
        project_root.join(".devin/skills"),
        project_root.join(".agents/skills"),
        project_root.join(".claude/skills"),
        home.join(".config/devin/skills"),
        home.join(".agents/skills"),
        home.join(".claude/skills"),
    ]
}

pub fn scan(dirs: &[PathBuf]) -> BTreeSet<String> {
    dirs.iter()
        .filter_map(|dir| std::fs::read_dir(dir).ok())
        .flatten()
        .flatten()
        .filter(|entry| entry.path().join("SKILL.md").is_file())
        .filter_map(|entry| entry.file_name().into_string().ok())
        .collect()
}

pub fn inventory(project_root: &Path, devin: &Path, home: &Path) -> Inventory {
    let mut command = Command::new(devin);
    command
        .args(["skills", "list", "--json"])
        .current_dir(project_root);
    for variable in DESKTOP_VARIABLES {
        command.env_remove(variable);
    }
    let listed = command
        .output()
        .ok()
        .filter(|output| output.status.success())
        .and_then(|output| parse_devin_list(&String::from_utf8_lossy(&output.stdout)));
    match listed {
        Some(names) => Inventory {
            source: InventorySource::Devin,
            names,
        },
        None => Inventory {
            source: InventorySource::Approximate,
            names: scan(&default_dirs(project_root, home)),
        },
    }
}
