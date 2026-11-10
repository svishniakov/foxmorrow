use std::collections::BTreeMap;
use std::path::Path;

use foxmorrow_core::catalog::Tier;
use foxmorrow_core::claims::ModelFamilies;
use foxmorrow_core::error::invalid;
use foxmorrow_core::{Error, Result};
use serde::Deserialize;

pub const PROFILE_FILE: &str = "profiles/devin.json";
pub const PROJECT_PROFILE_FILE: &str = ".foxmorrow/profile.json";

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FamilyRule {
    pub family: String,
    pub prefixes: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Profile {
    pub version: u32,
    pub brief_language: String,
    pub family_rule: bool,
    pub tiers: BTreeMap<Tier, String>,
    #[serde(default)]
    pub roles: BTreeMap<String, String>,
    pub families: Vec<FamilyRule>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProjectProfile {
    #[serde(default)]
    pub tiers: BTreeMap<Tier, String>,
    #[serde(default)]
    pub roles: BTreeMap<String, String>,
    #[serde(default)]
    pub family_rule: Option<bool>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModelChoice {
    pub model: String,
    pub project_override: bool,
}

impl Profile {
    pub fn load(plugin_root: &Path) -> Result<Self> {
        let path = plugin_root.join(PROFILE_FILE);
        let text = std::fs::read_to_string(&path)
            .map_err(|_| Error::NotFound(format!("model profile {}", path.display())))?;
        let profile: Profile = serde_json::from_str(&text)?;
        if profile.version != 1 {
            return invalid(format!(
                "unsupported model profile version {}",
                profile.version
            ));
        }
        if let Some(tier) = Tier::ALL
            .iter()
            .find(|tier| !profile.tiers.contains_key(tier))
        {
            return invalid(format!(
                "model profile has no model for tier {}",
                tier.as_str()
            ));
        }
        if let Some(model) = profile
            .tiers
            .values()
            .chain(profile.roles.values())
            .find(|model| profile.family_of(model).is_none())
        {
            return invalid(format!("model {model} has no family in the profile"));
        }
        Ok(profile)
    }

    pub fn model_for(&self, role: &str, tier: Tier, project: &ProjectProfile) -> ModelChoice {
        if let Some(model) = project.roles.get(role).or_else(|| project.tiers.get(&tier)) {
            return ModelChoice {
                model: model.clone(),
                project_override: true,
            };
        }
        let model = self
            .roles
            .get(role)
            .or_else(|| self.tiers.get(&tier))
            .cloned()
            .unwrap_or_default();
        ModelChoice {
            model,
            project_override: false,
        }
    }

    pub fn family_rule(&self, project: &ProjectProfile) -> bool {
        project.family_rule.unwrap_or(self.family_rule)
    }

    pub fn family_of(&self, model: &str) -> Option<&str> {
        self.families
            .iter()
            .find(|rule| {
                rule.prefixes
                    .iter()
                    .any(|prefix| model.starts_with(prefix.as_str()))
            })
            .map(|rule| rule.family.as_str())
    }

    pub fn families_for<'a>(&self, models: impl IntoIterator<Item = &'a String>) -> ModelFamilies {
        models
            .into_iter()
            .filter_map(|model| {
                self.family_of(model)
                    .map(|family| (model.clone(), family.to_string()))
            })
            .collect()
    }
}

impl ProjectProfile {
    pub fn load(project_root: &Path) -> Result<Self> {
        let path = project_root.join(PROJECT_PROFILE_FILE);
        if !path.exists() {
            return Ok(Self::default());
        }
        Ok(serde_json::from_str(&std::fs::read_to_string(path)?)?)
    }
}
