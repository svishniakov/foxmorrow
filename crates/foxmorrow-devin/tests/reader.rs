use std::path::{Path, PathBuf};

use foxmorrow_core::Error;
use foxmorrow_devin::reader::{ChainQuery, Reader, locate};
use foxmorrow_devin::schema_map::SchemaMap;
use rusqlite::Connection;

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .unwrap()
}

fn fixture_dir() -> PathBuf {
    root().join("tests/fixtures/devin-3000.12.43")
}

fn manifest() -> serde_json::Value {
    serde_json::from_str(&std::fs::read_to_string(fixture_dir().join("manifest.json")).unwrap())
        .unwrap()
}

fn copy_db() -> (tempfile::TempDir, PathBuf) {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("sessions.db");
    std::fs::copy(fixture_dir().join("sessions.db"), &path).unwrap();
    (dir, path)
}

fn open(path: &Path) -> Reader {
    Reader::open(path, SchemaMap::load(&root()).unwrap()).unwrap()
}

fn query(manifest: &serde_json::Value, role: &str, letter: &str) -> ChainQuery {
    ChainQuery {
        profile: manifest["profiles"][role].as_str().unwrap().to_string(),
        needles: vec![format!(
            "PROBE-TASK-{letter} {}",
            manifest["nonce"].as_str().unwrap()
        )],
    }
}

#[test]
fn reads_each_probe_chain() {
    let manifest = manifest();
    let nonce = manifest["nonce"].as_str().unwrap();
    let session = manifest["session_id"].as_str().unwrap();
    let (_dir, path) = copy_db();
    let reader = open(&path);
    for (role, letter, exit_code) in [
        ("probe-a", "A", 0),
        ("probe-b", "B", 3),
        ("fox-probe-c", "C", 0),
    ] {
        let query = query(&manifest, role, letter);
        let observed = reader.find_subagent(session, &query).unwrap();
        assert_eq!(
            observed.agent_id,
            manifest["agent_ids"][role].as_str().unwrap()
        );
        assert_eq!(observed.profile, query.profile);
        assert_eq!(
            observed.models,
            vec![manifest["models"][role].as_str().unwrap().to_string()]
        );
        let marker = format!("foxmorrow-role-sha256: {role}-{nonce}");
        assert!(
            observed
                .system_texts
                .iter()
                .any(|text| text.contains(&marker)),
            "{role}"
        );
        assert!(observed.task_text.contains(&query.needles[0]));
        let exec = observed
            .tool_calls
            .iter()
            .find(|call| call.name == "exec")
            .unwrap();
        assert_eq!(exec.exit_code, Some(exit_code), "{role}");
        assert!(exec.command.as_deref().unwrap().contains(nonce));
        assert_eq!(observed.source, "devin-sessions-db map=1 migration=20");
    }
}

#[test]
fn reads_the_root_chain() {
    let manifest = manifest();
    let session = manifest["session_id"].as_str().unwrap();
    let (_dir, path) = copy_db();
    let reader = open(&path);
    assert_eq!(
        reader.root_model(session).unwrap().as_deref(),
        Some("swe-2-high")
    );
    let observed = reader.root_observation(session).unwrap();
    assert_eq!(
        (observed.agent_id.as_str(), observed.profile.as_str()),
        ("root", "")
    );
    assert_eq!(observed.models, vec!["swe-2-high".to_string()]);
    assert_eq!(
        observed
            .tool_calls
            .iter()
            .filter(|call| call.name == "run_subagent")
            .count(),
        3
    );
    assert_eq!(reader.root_model("missing-session").unwrap(), None);
    assert!(matches!(
        reader.root_observation("missing-session"),
        Err(Error::NotFound(_))
    ));
}

#[test]
fn finds_the_latest_session_of_a_directory() {
    let (_dir, path) = copy_db();
    let reader = open(&path);
    assert_eq!(
        reader
            .latest_session_in("/fixture/project")
            .unwrap()
            .as_deref(),
        Some("dedicated-sherbet")
    );
    assert_eq!(reader.latest_session_in("/elsewhere").unwrap(), None);
    assert_eq!(
        reader
            .latest_session_in("/fixture/project/polydesk-saas")
            .unwrap()
            .as_deref(),
        Some("dedicated-sherbet")
    );
    assert_eq!(reader.latest_session_in("/fixture/projectx").unwrap(), None);
}

#[test]
fn refuses_unmatched_and_ambiguous_calls() {
    let manifest = manifest();
    let session = manifest["session_id"].as_str().unwrap();
    let (_dir, path) = copy_db();
    {
        let reader = open(&path);
        let mut wrong = query(&manifest, "probe-a", "A");
        wrong.needles = vec!["PROBE-TASK-B".into()];
        assert!(matches!(
            reader.find_subagent(session, &wrong),
            Err(Error::NotFound(_))
        ));
        let mut other = query(&manifest, "probe-a", "A");
        other.profile = "fox-probe:unknown".into();
        assert!(matches!(
            reader.find_subagent(session, &other),
            Err(Error::NotFound(_))
        ));
    }
    Connection::open(&path)
        .unwrap()
        .execute(
            "INSERT INTO tool_call_state (session_id, tool_call_id, tool_call_json) SELECT session_id, 'duplicate', tool_call_json FROM tool_call_state WHERE tool_call_json LIKE '%fox-probe:probe-a%'",
            [],
        )
        .unwrap();
    let reader = open(&path);
    assert!(matches!(
        reader.find_subagent(session, &query(&manifest, "probe-a", "A")),
        Err(Error::Conflict(_))
    ));
}

#[test]
fn refuses_a_missing_agent_id_or_chain() {
    let manifest = manifest();
    let session = manifest["session_id"].as_str().unwrap();
    let (_dir, path) = copy_db();
    let connection = Connection::open(&path).unwrap();
    connection
        .execute(
            "UPDATE message_nodes SET chat_message = replace(chat_message, 'Subagent agent_id=', 'Subagent id ') WHERE chat_message LIKE '%Subagent agent_id=%'",
            [],
        )
        .unwrap();
    drop(connection);
    let reader = open(&path);
    assert!(matches!(
        reader.find_subagent(session, &query(&manifest, "probe-b", "B")),
        Err(Error::NotFound(_))
    ));
}

#[test]
fn checks_the_schema_before_reading() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("sessions.db");
    Connection::open(&path)
        .unwrap()
        .execute_batch("CREATE TABLE sessions (id TEXT);")
        .unwrap();
    let error = Reader::open(&path, SchemaMap::load(&root()).unwrap())
        .err()
        .unwrap();
    assert!(error.to_string().contains("lacks"), "{error}");
    assert!(matches!(
        Reader::open(
            &dir.path().join("none.db"),
            SchemaMap::load(&root()).unwrap()
        ),
        Err(Error::NotFound(_))
    ));
}

#[test]
fn locates_the_database_by_priority() {
    let home = Path::new("/home/user");
    let explicit = Some(PathBuf::from("/a.db"));
    let env = Some(PathBuf::from("/b.db"));
    let dir = Some(PathBuf::from("/devin"));
    assert_eq!(
        locate(explicit.clone(), env.clone(), dir.clone(), home),
        PathBuf::from("/a.db")
    );
    assert_eq!(locate(None, env, dir.clone(), home), PathBuf::from("/b.db"));
    assert_eq!(
        locate(None, None, dir, home),
        PathBuf::from("/devin/sessions.db")
    );
    assert_eq!(
        locate(None, None, None, home),
        PathBuf::from("/home/user/.local/share/devin/cli/sessions.db")
    );
}
