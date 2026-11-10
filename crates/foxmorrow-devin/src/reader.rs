use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::time::Duration;

use foxmorrow_core::claims::{ChainObservation, ToolCall};
use foxmorrow_core::error::invalid;
use foxmorrow_core::{Error, Result};
use rusqlite::{Connection, OpenFlags, OptionalExtension, params};
use serde_json::Value;

use crate::schema_map::SchemaMap;

const MAX_CHAIN: usize = 200_000;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChainQuery {
    pub profile: String,
    pub needles: Vec<String>,
}

pub struct Reader {
    conn: Connection,
    map: SchemaMap,
    migration: Option<i64>,
}

struct Message {
    role: String,
    content: String,
    model: Option<String>,
    calls: Vec<RawCall>,
    tool_call_id: Option<String>,
}

struct RawCall {
    id: String,
    name: String,
    command: Option<String>,
}

pub fn locate(
    explicit: Option<PathBuf>,
    foxmorrow_env: Option<PathBuf>,
    devin_dir: Option<PathBuf>,
    home: &Path,
) -> PathBuf {
    explicit
        .or(foxmorrow_env)
        .or_else(|| devin_dir.map(|dir| dir.join("sessions.db")))
        .unwrap_or_else(|| home.join(".local/share/devin/cli/sessions.db"))
}

fn text(value: &Value, pointer: &str) -> Option<String> {
    value
        .pointer(pointer)
        .and_then(Value::as_str)
        .filter(|found| !found.is_empty())
        .map(str::to_string)
}

impl Reader {
    pub fn open(path: &Path, map: SchemaMap) -> Result<Self> {
        if !path.is_file() {
            return Err(Error::NotFound(format!(
                "Devin session database {}",
                path.display()
            )));
        }
        let conn = Connection::open_with_flags(
            path,
            OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_NO_MUTEX,
        )?;
        conn.busy_timeout(Duration::from_secs(5))?;
        for (table, columns) in &map.tables {
            let mut statement = conn.prepare(&format!("PRAGMA table_info({table})"))?;
            let present: BTreeSet<String> = statement
                .query_map([], |row| row.get::<_, String>(1))?
                .collect::<std::result::Result<_, _>>()?;
            if let Some(missing) = columns.iter().find(|column| !present.contains(*column)) {
                return invalid(format!(
                    "Devin session database lacks {table}.{missing}; schema map {} does not fit this Devin version",
                    map.map_version
                ));
            }
        }
        let migration = conn
            .query_row(
                "SELECT MAX(version) FROM refinery_schema_history",
                [],
                |row| row.get::<_, Option<i64>>(0),
            )
            .ok()
            .flatten();
        Ok(Self {
            conn,
            map,
            migration,
        })
    }

    pub fn source(&self) -> String {
        let migration = self
            .migration
            .map_or_else(|| "unknown".to_string(), |version| version.to_string());
        format!(
            "devin-sessions-db map={} migration={migration}",
            self.map.map_version
        )
    }

    pub fn latest_session_in(&self, working_dir: &str) -> Result<Option<String>> {
        Ok(self
            .conn
            .query_row(
                "SELECT id FROM sessions WHERE working_directory = ?1 ORDER BY last_activity_at DESC LIMIT 1",
                [working_dir],
                |row| row.get(0),
            )
            .optional()?)
    }

    pub fn root_model(&self, session_id: &str) -> Result<Option<String>> {
        let Some(head) = self.main_chain(session_id)? else {
            return Ok(None);
        };
        let messages = self.chain(session_id, head)?;
        Ok(messages
            .iter()
            .rev()
            .find(|message| message.role == "assistant" && message.model.is_some())
            .and_then(|message| message.model.clone()))
    }

    pub fn root_observation(&self, session_id: &str) -> Result<ChainObservation> {
        let head = self
            .main_chain(session_id)?
            .ok_or_else(|| Error::NotFound(format!("main chain of session {session_id}")))?;
        self.observe(session_id, head, "root", "")
    }

    pub fn find_subagent(&self, session_id: &str, query: &ChainQuery) -> Result<ChainObservation> {
        let call_id = self.matching_call(session_id, query)?;
        let agent_id = self.agent_id_of(session_id, &call_id)?;
        let head: Option<i64> = self
            .conn
            .query_row(
                "SELECT chain_node_id FROM subagent_heads WHERE session_id = ?1 AND agent_id = ?2",
                params![session_id, agent_id],
                |row| row.get(0),
            )
            .optional()?;
        let head = head.ok_or_else(|| Error::NotFound(format!("chain of subagent {agent_id}")))?;
        let observed = self.observe(session_id, head, &agent_id, &query.profile)?;
        if let Some(missing) = query
            .needles
            .iter()
            .find(|needle| !observed.task_text.contains(needle.as_str()))
        {
            return invalid(format!(
                "task of subagent {agent_id} does not contain '{missing}'"
            ));
        }
        Ok(observed)
    }

    fn main_chain(&self, session_id: &str) -> Result<Option<i64>> {
        let head: Option<Option<i64>> = self
            .conn
            .query_row(
                "SELECT main_chain_id FROM sessions WHERE id = ?1",
                [session_id],
                |row| row.get(0),
            )
            .optional()?;
        Ok(head.flatten())
    }

    fn matching_call(&self, session_id: &str, query: &ChainQuery) -> Result<String> {
        let mut statement = self.conn.prepare(
            "SELECT tool_call_id, tool_call_json FROM tool_call_state WHERE session_id = ?1 AND tool_call_json IS NOT NULL AND instr(tool_call_json, ?2) > 0",
        )?;
        let rows = statement.query_map(params![session_id, query.profile], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
        })?;
        let mut matches = Vec::new();
        for row in rows {
            let (id, json) = row?;
            let Ok(value) = serde_json::from_str::<Value>(&json) else {
                continue;
            };
            if text(&value, &self.map.request.profile).as_deref() != Some(query.profile.as_str()) {
                continue;
            }
            let task = text(&value, &self.map.request.task).unwrap_or_default();
            if query
                .needles
                .iter()
                .all(|needle| task.contains(needle.as_str()))
            {
                matches.push(id);
            }
        }
        match matches.len() {
            0 => Err(Error::NotFound(format!(
                "run_subagent call with profile {} and the recorded brief",
                query.profile
            ))),
            1 => Ok(matches.remove(0)),
            count => Err(Error::Conflict(format!(
                "{count} run_subagent calls match the recorded brief"
            ))),
        }
    }

    fn agent_id_of(&self, session_id: &str, call_id: &str) -> Result<String> {
        let mut statement = self
            .conn
            .prepare("SELECT chat_message FROM message_nodes WHERE session_id = ?1 AND instr(chat_message, ?2) > 0")?;
        let rows =
            statement.query_map(params![session_id, call_id], |row| row.get::<_, String>(0))?;
        let mut found = BTreeSet::new();
        for row in rows {
            let message = self.parse_message(&row?)?;
            if message.role != "tool" || message.tool_call_id.as_deref() != Some(call_id) {
                continue;
            }
            if let Some((_, rest)) = message.content.split_once(&self.map.subagent_result_prefix) {
                let id: String = rest
                    .chars()
                    .take_while(|c| c.is_ascii_alphanumeric() || *c == '-' || *c == '_')
                    .collect();
                if !id.is_empty() {
                    found.insert(id);
                }
            }
        }
        match found.len() {
            0 => Err(Error::NotFound(format!(
                "agent id in the result of call {call_id}"
            ))),
            1 => Ok(found.into_iter().next().unwrap_or_default()),
            _ => Err(Error::Conflict(format!(
                "call {call_id} reports several agent ids"
            ))),
        }
    }

    fn observe(
        &self,
        session_id: &str,
        head: i64,
        agent_id: &str,
        profile: &str,
    ) -> Result<ChainObservation> {
        let messages = self.chain(session_id, head)?;
        let task_text = messages
            .iter()
            .find(|message| message.role == "user")
            .map(|message| message.content.clone())
            .ok_or_else(|| Error::NotFound("task message in the chain".into()))?;
        let system_texts = messages
            .iter()
            .filter(|message| message.role == "system")
            .map(|message| message.content.clone())
            .collect();
        let assistants: Vec<&Message> = messages
            .iter()
            .filter(|message| message.role == "assistant")
            .collect();
        let models: Vec<String> = if assistants.iter().all(|message| message.model.is_some()) {
            assistants
                .iter()
                .filter_map(|message| message.model.clone())
                .collect::<BTreeSet<_>>()
                .into_iter()
                .collect()
        } else {
            Vec::new()
        };
        let mut tool_calls = Vec::new();
        for call in assistants.iter().flat_map(|message| message.calls.iter()) {
            tool_calls.push(ToolCall {
                id: call.id.clone(),
                name: call.name.clone(),
                command: call.command.clone(),
                exit_code: self.exit_code(session_id, &call.id)?,
            });
        }
        Ok(ChainObservation {
            source: self.source(),
            agent_id: agent_id.to_string(),
            profile: profile.to_string(),
            task_text,
            system_texts,
            models,
            tool_calls,
        })
    }

    fn exit_code(&self, session_id: &str, call_id: &str) -> Result<Option<i64>> {
        let json: Option<Option<String>> = self
            .conn
            .query_row(
                "SELECT tool_call_update_json FROM tool_call_state WHERE session_id = ?1 AND tool_call_id = ?2",
                params![session_id, call_id],
                |row| row.get(0),
            )
            .optional()?;
        Ok(json
            .flatten()
            .and_then(|text| serde_json::from_str::<Value>(&text).ok())
            .and_then(|value| {
                value
                    .pointer(&self.map.result.exit_code)
                    .and_then(Value::as_i64)
            }))
    }

    fn chain(&self, session_id: &str, head: i64) -> Result<Vec<Message>> {
        let mut statement = self
            .conn
            .prepare("SELECT parent_node_id, chat_message FROM message_nodes WHERE session_id = ?1 AND node_id = ?2")?;
        let mut messages = Vec::new();
        let mut seen = BTreeSet::new();
        let mut next = Some(head);
        while let Some(id) = next {
            if !seen.insert(id) || messages.len() >= MAX_CHAIN {
                return invalid("message chain is cyclic or too long");
            }
            let row: Option<(Option<i64>, String)> = statement
                .query_row(params![session_id, id], |row| {
                    Ok((row.get(0)?, row.get(1)?))
                })
                .optional()?;
            let Some((parent, json)) = row else {
                return Err(Error::NotFound(format!("message node {id}")));
            };
            messages.push(self.parse_message(&json)?);
            next = parent;
        }
        messages.reverse();
        Ok(messages)
    }

    fn parse_message(&self, json: &str) -> Result<Message> {
        let value: Value = serde_json::from_str(json)?;
        let paths = &self.map.message;
        let calls = value
            .pointer(&paths.tool_calls)
            .and_then(Value::as_array)
            .map(|items| {
                items
                    .iter()
                    .filter_map(|item| {
                        Some(RawCall {
                            id: text(item, &self.map.tool_call.id)?,
                            name: text(item, &self.map.tool_call.name).unwrap_or_default(),
                            command: text(item, &self.map.tool_call.command),
                        })
                    })
                    .collect()
            })
            .unwrap_or_default();
        Ok(Message {
            role: text(&value, &paths.role).unwrap_or_default(),
            content: text(&value, &paths.content).unwrap_or_default(),
            model: text(&value, &paths.model),
            calls,
            tool_call_id: text(&value, &paths.tool_call_id),
        })
    }
}
