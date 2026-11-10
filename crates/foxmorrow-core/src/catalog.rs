use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::error::{Error, Result, invalid};
use crate::model::Kind;
use crate::util::sha256_hex;

pub const CATALOG_FILE: &str = "roles/roles.json";
pub const COMMON_FILE: &str = "roles/_common.md";

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Tier {
    Worker,
    Analyst,
    Verifier,
    Escalation,
}

impl Tier {
    pub const ALL: [Tier; 4] = [
        Tier::Worker,
        Tier::Analyst,
        Tier::Verifier,
        Tier::Escalation,
    ];

    pub fn as_str(self) -> &'static str {
        match self {
            Tier::Worker => "worker",
            Tier::Analyst => "analyst",
            Tier::Verifier => "verifier",
            Tier::Escalation => "escalation",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ToolSet {
    Full,
    Verify,
}

#[derive(Debug, Clone, PartialEq, Eq, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Skills {
    #[serde(default)]
    pub required: Vec<String>,
    #[serde(default)]
    pub optional: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RoleSpec {
    pub description: String,
    pub tier: Tier,
    pub kind: Kind,
    pub tools: ToolSet,
    #[serde(default)]
    pub skills: Skills,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Source {
    pub repository: String,
    pub commit: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Catalog {
    pub version: u32,
    pub source: Source,
    pub roles: BTreeMap<String, RoleSpec>,
}

impl Catalog {
    pub fn load(root: &Path) -> Result<Self> {
        let path = root.join(CATALOG_FILE);
        let text = std::fs::read_to_string(&path)
            .map_err(|_| Error::NotFound(format!("role catalog {}", path.display())))?;
        let catalog: Catalog = serde_json::from_str(&text)?;
        if catalog.version != 1 {
            return invalid(format!(
                "unsupported role catalog version {}",
                catalog.version
            ));
        }
        let common = root.join(COMMON_FILE);
        if !common.is_file() {
            return Err(Error::NotFound(format!(
                "common role text {}",
                common.display()
            )));
        }
        for (name, spec) in &catalog.roles {
            if name.is_empty()
                || !name
                    .chars()
                    .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
            {
                return invalid(format!(
                    "role name '{name}' must use lowercase letters, digits and hyphens"
                ));
            }
            if spec.kind == Kind::Root {
                return invalid(format!("role {name} cannot have kind root"));
            }
            if spec.kind.is_verification() && spec.tools != ToolSet::Verify {
                return invalid(format!(
                    "verification role {name} must use the verify tool set"
                ));
            }
            if spec.description.trim().is_empty() {
                return invalid(format!("role {name} needs a description"));
            }
            let file = role_file(root, name);
            if !file.is_file() {
                return Err(Error::NotFound(format!("role text {}", file.display())));
            }
        }
        Ok(catalog)
    }

    pub fn role(&self, name: &str) -> Result<&RoleSpec> {
        self.roles
            .get(name)
            .ok_or_else(|| Error::NotFound(format!("role {name} in the catalog")))
    }
}

pub fn role_file(root: &Path, role: &str) -> PathBuf {
    root.join("roles").join(format!("{role}.md"))
}

pub fn role_text(root: &Path, role: &str) -> Result<String> {
    let own = std::fs::read_to_string(role_file(root, role))?;
    let common = std::fs::read_to_string(root.join(COMMON_FILE))?;
    Ok(format!("{}\n\n{}\n", own.trim_end(), common.trim_end()))
}

pub fn role_sha256(text: &str) -> String {
    sha256_hex(text.as_bytes())
}
