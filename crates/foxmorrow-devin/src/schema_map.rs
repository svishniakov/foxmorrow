use std::collections::BTreeMap;
use std::path::Path;

use foxmorrow_core::error::invalid;
use foxmorrow_core::{Error, Result};
use serde::Deserialize;

pub const SCHEMA_MAP_FILE: &str = "schema-maps/devin.json";
const REQUIRED_TABLES: [&str; 4] = [
    "sessions",
    "message_nodes",
    "tool_call_state",
    "subagent_heads",
];

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MessagePaths {
    pub role: String,
    pub content: String,
    pub model: String,
    pub tool_calls: String,
    pub tool_call_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ToolCallPaths {
    pub id: String,
    pub name: String,
    pub command: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RequestPaths {
    pub profile: String,
    pub task: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ResultPaths {
    pub exit_code: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SchemaMap {
    pub map_version: u32,
    pub tested_devin: Vec<String>,
    pub tables: BTreeMap<String, Vec<String>>,
    pub message: MessagePaths,
    pub tool_call: ToolCallPaths,
    pub request: RequestPaths,
    pub result: ResultPaths,
    pub subagent_result_prefix: String,
}

impl SchemaMap {
    pub fn load(plugin_root: &Path) -> Result<Self> {
        let path = plugin_root.join(SCHEMA_MAP_FILE);
        let text = std::fs::read_to_string(&path)
            .map_err(|_| Error::NotFound(format!("schema map {}", path.display())))?;
        Self::parse(&text)
    }

    pub fn parse(text: &str) -> Result<Self> {
        let map: SchemaMap = serde_json::from_str(text)?;
        map.validate()?;
        Ok(map)
    }

    fn validate(&self) -> Result<()> {
        if self.map_version != 1 {
            return invalid(format!(
                "unsupported schema map version {}",
                self.map_version
            ));
        }
        if let Some(table) = REQUIRED_TABLES
            .iter()
            .find(|table| !self.tables.contains_key(**table))
        {
            return invalid(format!("schema map lacks table {table}"));
        }
        if let Some(table) = self.tables.keys().find(|table| {
            table.is_empty() || !table.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')
        }) {
            return invalid(format!(
                "schema map table name '{table}' is not an identifier"
            ));
        }
        let pointers = [
            &self.message.role,
            &self.message.content,
            &self.message.model,
            &self.message.tool_calls,
            &self.message.tool_call_id,
            &self.tool_call.id,
            &self.tool_call.name,
            &self.tool_call.command,
            &self.request.profile,
            &self.request.task,
            &self.result.exit_code,
        ];
        if let Some(pointer) = pointers.iter().find(|pointer| !pointer.starts_with('/')) {
            return invalid(format!(
                "schema map pointer '{pointer}' must start with '/'"
            ));
        }
        if self.subagent_result_prefix.trim().is_empty() {
            return invalid("schema map subagent_result_prefix must not be empty");
        }
        Ok(())
    }
}
