use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use foxmorrow_core::catalog::{self, Catalog, RoleSpec, ToolSet};
use foxmorrow_core::error::invalid;
use foxmorrow_core::{Error, Result};
use serde::Serialize;

use crate::profile::{Profile, ProjectProfile};

pub const PLUGIN_NAME: &str = "foxmorrow";
pub const PROJECT_PREFIX: &str = "fox-";
pub const MARKER: &str = "foxmorrow-role-sha256: ";
pub const VERIFY_TOOLS: [&str; 15] = [
    "read",
    "grep",
    "glob",
    "exec",
    "get_output",
    "write_to_process",
    "kill_shell",
    "webfetch",
    "web_search",
    "todo_write",
    "skill",
    "mcp_list_servers",
    "mcp_list_tools",
    "mcp_call_tool",
    "mcp_read_resource",
];

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentDefinition {
    pub role: String,
    pub profile: String,
    pub path: PathBuf,
    pub model: String,
    pub role_sha256: String,
    pub content: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize)]
pub struct SyncReport {
    pub stale: Vec<PathBuf>,
    pub written: Vec<PathBuf>,
    pub removed: Vec<PathBuf>,
}

pub fn render(
    name: &str,
    spec: &RoleSpec,
    model: &str,
    role_text: &str,
) -> Result<(String, String)> {
    let digest = catalog::role_sha256(role_text);
    let mut content = format!(
        "---\nname: {name}\ndescription: {}\nmodel: {model}\n",
        serde_json::to_string(&spec.description)?
    );
    if spec.tools == ToolSet::Verify {
        content.push_str("allowed-tools:\n");
        for tool in VERIFY_TOOLS {
            content.push_str(&format!("  - {tool}\n"));
        }
    }
    content.push_str(&format!("---\n\n{MARKER}{digest}\n\n{role_text}"));
    Ok((content, digest))
}

pub fn definition(
    plugin_root: &Path,
    project_root: &Path,
    catalog: &Catalog,
    profile: &Profile,
    project: &ProjectProfile,
    role: &str,
) -> Result<AgentDefinition> {
    let spec = catalog.role(role)?;
    let choice = profile.model_for(role, spec.tier, project);
    if profile.family_of(&choice.model).is_none() {
        return invalid(format!(
            "model {} for role {role} has no family in the profile",
            choice.model
        ));
    }
    let text = catalog::role_text(plugin_root, role)?;
    let (name, profile_name, path) = if choice.project_override {
        let name = format!("{PROJECT_PREFIX}{role}");
        let path = project_root
            .join(".devin")
            .join("agents")
            .join(format!("{name}.md"));
        (name.clone(), name, path)
    } else {
        let path = plugin_root.join("agents").join(format!("{role}.md"));
        (role.to_string(), format!("{PLUGIN_NAME}:{role}"), path)
    };
    let (content, role_sha256) = render(&name, spec, &choice.model, &text)?;
    Ok(AgentDefinition {
        role: role.to_string(),
        profile: profile_name,
        path,
        model: choice.model,
        role_sha256,
        content,
    })
}

pub fn plugin_definitions(
    plugin_root: &Path,
    catalog: &Catalog,
    profile: &Profile,
) -> Result<Vec<AgentDefinition>> {
    catalog
        .roles
        .keys()
        .map(|role| {
            definition(
                plugin_root,
                plugin_root,
                catalog,
                profile,
                &ProjectProfile::default(),
                role,
            )
        })
        .collect()
}

pub fn project_definitions(
    plugin_root: &Path,
    project_root: &Path,
    catalog: &Catalog,
    profile: &Profile,
    project: &ProjectProfile,
) -> Result<Vec<AgentDefinition>> {
    catalog
        .roles
        .iter()
        .filter(|(role, spec)| profile.model_for(role, spec.tier, project).project_override)
        .map(|(role, _)| definition(plugin_root, project_root, catalog, profile, project, role))
        .collect()
}

pub fn ensure_current(definition: &AgentDefinition) -> Result<()> {
    let current = std::fs::read_to_string(&definition.path).unwrap_or_default();
    if current == definition.content {
        return Ok(());
    }
    let command = if definition.profile.starts_with(PROJECT_PREFIX) {
        "foxmorrow sync"
    } else {
        "foxmorrow sync --plugin"
    };
    Err(Error::Conflict(format!(
        "{} is missing or stale; run {command}",
        definition.path.display()
    )))
}

pub fn sync(
    definitions: &[AgentDefinition],
    dir: &Path,
    prefix: Option<&str>,
    check: bool,
) -> Result<SyncReport> {
    let mut report = SyncReport::default();
    for definition in definitions {
        if std::fs::read_to_string(&definition.path).ok().as_deref()
            == Some(definition.content.as_str())
        {
            continue;
        }
        report.stale.push(definition.path.clone());
        if !check {
            if let Some(parent) = definition.path.parent() {
                std::fs::create_dir_all(parent)?;
            }
            std::fs::write(&definition.path, &definition.content)?;
            report.written.push(definition.path.clone());
        }
    }
    let wanted: BTreeSet<&Path> = definitions
        .iter()
        .map(|definition| definition.path.as_path())
        .collect();
    let mut entries: Vec<PathBuf> = std::fs::read_dir(dir)
        .into_iter()
        .flatten()
        .flatten()
        .map(|entry| entry.path())
        .collect();
    entries.sort();
    for path in entries {
        let name = path
            .file_name()
            .map(|name| name.to_string_lossy().into_owned())
            .unwrap_or_default();
        if !name.ends_with(".md")
            || wanted.contains(path.as_path())
            || prefix.is_some_and(|prefix| !name.starts_with(prefix))
        {
            continue;
        }
        if !std::fs::read_to_string(&path).is_ok_and(|text| text.contains(MARKER)) {
            continue;
        }
        report.stale.push(path.clone());
        if !check {
            std::fs::remove_file(&path)?;
            report.removed.push(path);
        }
    }
    Ok(report)
}
