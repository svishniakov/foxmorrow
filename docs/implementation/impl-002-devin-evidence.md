# План 002. Адаптер Devin: доказательства

Document status: approved
Based on: [foxmorrow v1: дизайн](../design/2026-11-10-foxmorrow-v1-design.md), [дорожная карта](roadmap-v1.md), [результаты плана 000](impl-000-devin-probe.md#результаты), [план 001](impl-001-core-journal.md), [план 003](impl-003-roles-profiles-briefs.md)

Задачи выполняются по порядку, шаги отмечаются чекбоксами `- [ ]`. Коммиты из шагов «Коммит» выполняются, только если пользователь разрешил коммиты в этом run.

**Цель:** превратить записи `sessions.db` Devin в нормализованные наблюдения `ChainObservation` и подключить их к ядру. Команды `foxmorrow verify` и `foxmorrow finalize`, обработчик `foxmorrow hook`, запись модели и сессии root при `init`.

**Архитектура:**
- **Карта схемы как данные.** `schema-maps/devin.json` описывает таблицы, обязательные колонки и JSON-указатели на нужные поля. Бинарник только интерпретирует карту. Изменение схемы Devin исправляется новой картой без пересборки.
- **Reader в `foxmorrow-devin`.** Открывает `sessions.db` только на чтение, проверяет наличие нужных таблиц и колонок, строит цепочку субагента или root. Все извлечения сверяются с положительными якорями. Если якорь не сошёлся, наблюдения нет, а ядро выставляет `unverified`.
- **Связь вызова с цепочкой.** Проба показала, что `tool_call_state` для `run_subagent` не хранит `agent_id`. Он есть в сообщении с ролью `tool` в цепочке root: `Subagent agent_id=<id> completed successfully`, `tool_call_id` которого совпадает с ID вызова. Через него вызов связывается с `subagent_heads`.
- **Hooks** только помогают root: фиксируют `session_id` при маркере `$fox` и возвращают состояние run после сжатия контекста. Подтверждений они не дают.

**Стек:** тот же, что в планах 001 и 003. Новых внешних зависимостей нет.

**Вне плана:** `hooks.json` и загрузчик (план 005); skill `fox` (план 004); проверка hooks в живом Devin и автообновление плагина (план 005); Devin Cloud.

## Решения плана, уточняющие дизайн

| Вопрос | Решение | Основание |
|---|---|---|
| Запись `PostToolUse` в журнал | Не реализуется | Дизайн §8.1 предполагал, что hooks дописывают наблюдения. Но проба показала, что авторство и вызовы надёжно берутся из `sessions.db` по `tool_use_id`, а подтверждать hooks ничего не могут. Лог без потребителя не нужен. Вернуть можно, когда появится потребитель |
| Выбор цепочки субагента | Вызов `run_subagent` ищется по `profile` и по первой строке брифа (`foxmorrow assignment <id> run <run> brief-sha256 <хеш>`). Ноль совпадений или больше одного дают отсутствие наблюдения | Заголовок уникален для run и назначения. Неоднозначность не должна приводить к ложному `passed` |
| Сообщения без модели | Если хотя бы у одного ответа цепочки нет `generation_model`, список моделей наблюдения пуст, и C3 получает `unverified` | Дизайн §8.2: подтверждение требует модель у всех ответов |
| Путь к `sessions.db` | Порядок: флаг `--sessions-db`, переменная `FOXMORROW_SESSIONS_DB`, `$DEVIN_SESSIONS_DIR/sessions.db`, затем `~/.local/share/devin/cli/sessions.db` | `DEVIN_SESSIONS_DIR` документирована в changelog Devin. Путь на Windows не проверялся |
| Сессия root | Порядок: `--session-id`, `.foxmorrow/session` (пишет hook), последняя по активности сессия с `working_directory`, равным каталогу проекта | Дизайн §7 |
| Модель root | При `init` и `verify` берётся модель последнего ответа в главной цепочке root, записывается один раз | Дизайн §6.1 |
| Модели для таблицы семейств | Объединение моделей из принятых наблюдений журнала, ожидаемых моделей назначений и текущего наблюдения | Ядру нужна таблица семейств для всех моделей, которые встретились в run |
| Окружение Desktop | Reader не запускает `devin`, поэтому проблема из плана 000 его не касается | `sessions.db` читается напрямую |
| Запуск через Windows | Ядро распознаёт `foxmorrow`, `foxmorrow.exe` и `foxmorrow.cmd` с любыми разделителями пути | Загрузчик плагина на Windows будет называться `foxmorrow.cmd` (дизайн §5.1) |
| Вывод `PostCompaction` | Возвращается как `additionalContext` по аналогии с `UserPromptSubmit`. Поддержка этого поля для `PostCompaction` в документации Devin не подтверждена | Проверяется вживую в плане 005; обработчик при этом безопасен |

## Критерии приёмки

- `cargo fmt --all --check`, `cargo clippy --workspace --all-targets -- -D warnings` и `cargo test --workspace` завершаются успешно на macOS.
- `schema-maps/devin.json` проходит разбор и проверку. Невалидная карта (версия не 1, указатель без `/`, нет обязательной таблицы, недопустимое имя таблицы) отклоняется.
- На образце `tests/fixtures/devin-3000.12.43` reader для каждой из трёх проб возвращает `agent_id`, профиль, единственную модель роли, маркер роли в системных сообщениях, задание, вызов `exec` с кодом выхода `0`, `3` и `0`. Для root возвращает модель `swe-2-high` и три вызова `run_subagent`.
- Reader отказывает с понятной причиной, если нет нужной колонки, если вызов не найден или найден дважды, если в результате вызова нет `agent_id`.
- `foxmorrow verify -a ID` принимает отчёт с совпадающими поручением, ролью, моделью и вызовами, отклоняет несовпадение модели, а при недоступной базе возвращает `unverified` и код выхода 1. Повторная проверка после отклонения возможна.
- `foxmorrow verify` для назначения root использует главную цепочку root.
- `foxmorrow finalize --report ID` закрывает run, перепроверяя сохранённые снимки, и записывает модель root в final.
- `foxmorrow init` записывает идентификатор сессии root и модель root, если они известны.
- `foxmorrow hook UserPromptSubmit` при `$fox` или `$foxmorrow` записывает `.foxmorrow/session` и возвращает указание запустить skill `fox`; без маркера ничего не пишет и не выводит. `foxmorrow hook PostCompaction` возвращает состояние run только для сессии root. Ошибки hook не меняют код выхода 0.
- В коде нет поясняющих комментариев.

## Структура файлов

```
schema-maps/devin.json
crates/foxmorrow-core/src/run.rs                record_root
crates/foxmorrow-core/src/claims.rs             observed_models, распознавание запуска foxmorrow
crates/foxmorrow-core/tests/run.rs
crates/foxmorrow-core/tests/claims.rs
crates/foxmorrow-devin/src/lib.rs
crates/foxmorrow-devin/src/schema_map.rs
crates/foxmorrow-devin/src/reader.rs
crates/foxmorrow-devin/src/hook.rs
crates/foxmorrow-devin/tests/schema_map.rs
crates/foxmorrow-devin/tests/reader.rs
crates/foxmorrow-devin/tests/hook.rs
crates/foxmorrow/Cargo.toml                     rusqlite в dev-dependencies
crates/foxmorrow/src/main.rs                    init, verify, finalize, hook
crates/foxmorrow/tests/cli.rs
```

## Задача 1. Ядро: корень сессии, модели, запуск на Windows

**Файлы:** изменить `crates/foxmorrow-core/src/run.rs`, `src/claims.rs`, `tests/run.rs`, `tests/claims.rs`.

- [ ] **Шаг 1.** Тесты. В `crates/foxmorrow-core/tests/run.rs` добавить:

```rust
#[test]
fn records_root_identity_once() {
    let dir = tempfile::tempdir().unwrap();
    let (_, mut journal) = run::create_run(new_run(dir.path(), vec!["src/".into()])).unwrap();
    let first = run::record_root(&mut journal, Some("s-1"), Some("claude-opus-5-5-high")).unwrap();
    assert_eq!(first.root_session_id.as_deref(), Some("s-1"));
    assert_eq!(first.root_model.as_deref(), Some("claude-opus-5-5-high"));
    let second = run::record_root(&mut journal, Some("s-2"), Some("other")).unwrap();
    assert_eq!(second.root_session_id.as_deref(), Some("s-1"));
    assert_eq!(second.root_model.as_deref(), Some("claude-opus-5-5-high"));
    assert_eq!(run::record_root(&mut journal, None, None).unwrap(), second);
}
```

В `crates/foxmorrow-core/tests/claims.rs` добавить:

```rust
#[test]
fn observed_models_collects_accepted_snapshots() {
    let dir = tempfile::tempdir().unwrap();
    let (run, mut journal) = new_run(dir.path());
    accepted_chain(&mut journal, &run);
    let models: Vec<String> = claims::observed_models(journal.conn()).unwrap().into_iter().collect();
    assert_eq!(models, vec!["gpt-6".to_string(), "opus-5".to_string(), "swe-2-high".to_string()]);
}

#[test]
fn report_call_recognizes_platform_launchers() {
    let cases = [
        ("bin/foxmorrow report -a a-01 <<'JSON'", ClaimState::Verified),
        ("C:\\plugin\\bin\\foxmorrow.cmd report -a a-01", ClaimState::Verified),
        ("\"/opt/fox/foxmorrow.exe\" report -a a-01", ClaimState::Verified),
        ("notfoxmorrow report -a a-01", ClaimState::Failed),
    ];
    for (command, expected) in cases {
        let dir = tempfile::tempdir().unwrap();
        let (run, mut journal) = new_run(dir.path());
        let author = assignment::create(&mut journal, &run, spec("backend-worker", Kind::Author, "swe-2-high", &[])).unwrap();
        report::submit(&mut journal, &run, &author.id, &passed_report(&[])).unwrap();
        let mut observed = observation(&author, "agent-1", "swe-2-high");
        observed.tool_calls = vec![ToolCall { id: "c".into(), name: "exec".into(), command: Some(command.into()), exit_code: Some(0) }];
        let verification = claims::verify(&mut journal, &run, &author.id, Some(observed), &families()).unwrap();
        assert_eq!(claim_state(&verification.claims, "C2"), expected, "{command}");
    }
}
```

- [ ] **Шаг 2.** Запустить: `cargo test -p foxmorrow-core --test run --test claims`. Ожидается ошибка компиляции: нет `record_root` и `observed_models`.

- [ ] **Шаг 3.** В `run.rs` добавить после `open_run`:

```rust
pub fn record_root(journal: &mut Journal, session_id: Option<&str>, model: Option<&str>) -> Result<Run> {
    journal.write(|tx| {
        tx.execute(
            "UPDATE run SET root_session_id = COALESCE(root_session_id, ?1), root_model = COALESCE(root_model, ?2)",
            params![session_id, model],
        )?;
        Ok(())
    })?;
    load_run(journal.conn())
}
```

- [ ] **Шаг 4.** В `claims.rs` добавить после `accepted_observation`:

```rust
pub fn observed_models(conn: &Connection) -> Result<BTreeSet<String>> {
    let mut statement = conn.prepare("SELECT observation_json FROM verification WHERE observation_json IS NOT NULL")?;
    let rows = statement.query_map([], |row| row.get::<_, String>(0))?;
    let mut models = BTreeSet::new();
    for row in rows {
        let observed: ChainObservation = serde_json::from_str(&row?)?;
        models.extend(observed.models);
    }
    Ok(models)
}
```

В функции `invokes` заменить условие `pair[0].ends_with("foxmorrow")` на `is_foxmorrow(pair[0])` и добавить функцию:

```rust
fn is_foxmorrow(word: &str) -> bool {
    let name = word
        .trim_matches(|c| c == '"' || c == '\'')
        .rsplit(['/', '\\'])
        .next()
        .unwrap_or("");
    matches!(name, "foxmorrow" | "foxmorrow.exe" | "foxmorrow.cmd")
}
```

- [ ] **Шаг 5.** Запустить: `cargo test -p foxmorrow-core`. Ожидается: все тесты крейта пройдены, включая 3 новых.

- [ ] **Шаг 6.** Коммит:

```sh
git add crates/foxmorrow-core
git commit -m "Record root identity and recognize platform launchers in claims"
```

## Задача 2. Карта схемы Devin

**Файлы:** создать `schema-maps/devin.json`, `crates/foxmorrow-devin/src/schema_map.rs`, `tests/schema_map.rs`; изменить `src/lib.rs`.

- [ ] **Шаг 1.** `schema-maps/devin.json`:

```json
{
  "map_version": 1,
  "tested_devin": ["3000.12.43"],
  "tables": {
    "sessions": ["id", "working_directory", "last_activity_at", "main_chain_id"],
    "message_nodes": ["session_id", "node_id", "parent_node_id", "chat_message"],
    "tool_call_state": ["session_id", "tool_call_id", "tool_call_json", "tool_call_update_json"],
    "subagent_heads": ["session_id", "agent_id", "chain_node_id"]
  },
  "message": {
    "role": "/role",
    "content": "/content",
    "model": "/metadata/generation_model",
    "tool_calls": "/tool_calls",
    "tool_call_id": "/tool_call_id"
  },
  "tool_call": { "id": "/id", "name": "/name", "command": "/arguments/command" },
  "request": { "profile": "/rawInput/profile", "task": "/rawInput/task" },
  "result": { "exit_code": "/_meta/terminal_exit/exit_code" },
  "subagent_result_prefix": "Subagent agent_id="
}
```

`message`, `tool_call` описывают поля в `message_nodes.chat_message`, `request` читается из `tool_call_state.tool_call_json`, `result` из `tool_call_state.tool_call_update_json`.

- [ ] **Шаг 2.** Тест `crates/foxmorrow-devin/tests/schema_map.rs`:

```rust
use std::path::PathBuf;

use foxmorrow_core::Error;
use foxmorrow_devin::schema_map::{SCHEMA_MAP_FILE, SchemaMap};

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..").canonicalize().unwrap()
}

fn mutate(change: impl FnOnce(&mut serde_json::Value)) -> foxmorrow_core::Result<SchemaMap> {
    let text = std::fs::read_to_string(root().join(SCHEMA_MAP_FILE)).unwrap();
    let mut value: serde_json::Value = serde_json::from_str(&text).unwrap();
    change(&mut value);
    SchemaMap::parse(&value.to_string())
}

#[test]
fn shipped_map_loads() {
    let map = SchemaMap::load(&root()).unwrap();
    assert_eq!(map.map_version, 1);
    assert!(map.tested_devin.contains(&"3000.12.43".to_string()));
    assert_eq!(map.message.model, "/metadata/generation_model");
}

#[test]
fn rejects_broken_maps() {
    assert!(matches!(mutate(|v| v["map_version"] = 2.into()), Err(Error::Invalid(_))));
    assert!(matches!(mutate(|v| v["message"]["role"] = "role".into()), Err(Error::Invalid(_))));
    assert!(matches!(mutate(|v| { v["tables"].as_object_mut().unwrap().remove("subagent_heads"); }), Err(Error::Invalid(_))));
    assert!(matches!(mutate(|v| { v["tables"]["x; DROP TABLE y"] = serde_json::json!(["a"]); }), Err(Error::Invalid(_))));
    assert!(matches!(mutate(|v| v["subagent_result_prefix"] = " ".into()), Err(Error::Invalid(_))));
    assert!(matches!(mutate(|v| v["extra"] = 1.into()), Err(Error::Json(_))));
    assert!(matches!(SchemaMap::load(&root().join("nowhere")), Err(Error::NotFound(_))));
}
```

- [ ] **Шаг 3.** Запустить: `cargo test -p foxmorrow-devin --test schema_map`. Ожидается ошибка компиляции.

- [ ] **Шаг 4.** `crates/foxmorrow-devin/src/schema_map.rs`:

```rust
use std::collections::BTreeMap;
use std::path::Path;

use foxmorrow_core::error::invalid;
use foxmorrow_core::{Error, Result};
use serde::Deserialize;

pub const SCHEMA_MAP_FILE: &str = "schema-maps/devin.json";
const REQUIRED_TABLES: [&str; 4] = ["sessions", "message_nodes", "tool_call_state", "subagent_heads"];

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
            return invalid(format!("unsupported schema map version {}", self.map_version));
        }
        if let Some(table) = REQUIRED_TABLES.iter().find(|table| !self.tables.contains_key(**table)) {
            return invalid(format!("schema map lacks table {table}"));
        }
        if let Some(table) = self
            .tables
            .keys()
            .find(|table| table.is_empty() || !table.chars().all(|c| c.is_ascii_alphanumeric() || c == '_'))
        {
            return invalid(format!("schema map table name '{table}' is not an identifier"));
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
            return invalid(format!("schema map pointer '{pointer}' must start with '/'"));
        }
        if self.subagent_result_prefix.trim().is_empty() {
            return invalid("schema map subagent_result_prefix must not be empty");
        }
        Ok(())
    }
}
```

- [ ] **Шаг 5.** В `crates/foxmorrow-devin/src/lib.rs` добавить `pub mod schema_map;` после `pub mod profile;`.

- [ ] **Шаг 6.** Запустить: `cargo test -p foxmorrow-devin --test schema_map`. Ожидается: 2 теста пройдены.

- [ ] **Шаг 7.** Коммит:

```sh
git add schema-maps crates/foxmorrow-devin
git commit -m "Describe the Devin sessions.db schema as data"
```

## Задача 3. Reader `sessions.db`

**Файлы:** создать `crates/foxmorrow-devin/src/reader.rs`, `tests/reader.rs`; изменить `src/lib.rs`.

- [ ] **Шаг 1.** Тест `crates/foxmorrow-devin/tests/reader.rs`. Образец копируется во временный каталог, чтобы тесты не создавали файлов в репозитории:

```rust
use std::path::{Path, PathBuf};

use foxmorrow_core::Error;
use foxmorrow_devin::reader::{ChainQuery, Reader, locate};
use foxmorrow_devin::schema_map::SchemaMap;
use rusqlite::Connection;

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..").canonicalize().unwrap()
}

fn fixture_dir() -> PathBuf {
    root().join("tests/fixtures/devin-3000.12.43")
}

fn manifest() -> serde_json::Value {
    serde_json::from_str(&std::fs::read_to_string(fixture_dir().join("manifest.json")).unwrap()).unwrap()
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
        needles: vec![format!("PROBE-TASK-{letter} {}", manifest["nonce"].as_str().unwrap())],
    }
}

#[test]
fn reads_each_probe_chain() {
    let manifest = manifest();
    let nonce = manifest["nonce"].as_str().unwrap();
    let session = manifest["session_id"].as_str().unwrap();
    let (_dir, path) = copy_db();
    let reader = open(&path);
    for (role, letter, exit_code) in [("probe-a", "A", 0), ("probe-b", "B", 3), ("fox-probe-c", "C", 0)] {
        let query = query(&manifest, role, letter);
        let observed = reader.find_subagent(session, &query).unwrap();
        assert_eq!(observed.agent_id, manifest["agent_ids"][role].as_str().unwrap());
        assert_eq!(observed.profile, query.profile);
        assert_eq!(observed.models, vec![manifest["models"][role].as_str().unwrap().to_string()]);
        let marker = format!("foxmorrow-role-sha256: {role}-{nonce}");
        assert!(observed.system_texts.iter().any(|text| text.contains(&marker)), "{role}");
        assert!(observed.task_text.contains(&query.needles[0]));
        let exec = observed.tool_calls.iter().find(|call| call.name == "exec").unwrap();
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
    assert_eq!(reader.root_model(session).unwrap().as_deref(), Some("swe-2-high"));
    let observed = reader.root_observation(session).unwrap();
    assert_eq!((observed.agent_id.as_str(), observed.profile.as_str()), ("root", ""));
    assert_eq!(observed.models, vec!["swe-2-high".to_string()]);
    assert_eq!(observed.tool_calls.iter().filter(|call| call.name == "run_subagent").count(), 3);
    assert_eq!(reader.root_model("missing-session").unwrap(), None);
    assert!(matches!(reader.root_observation("missing-session"), Err(Error::NotFound(_))));
}

#[test]
fn finds_the_latest_session_of_a_directory() {
    let (_dir, path) = copy_db();
    let reader = open(&path);
    assert_eq!(reader.latest_session_in("/fixture/project").unwrap().as_deref(), Some("dedicated-sherbet"));
    assert_eq!(reader.latest_session_in("/elsewhere").unwrap(), None);
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
        assert!(matches!(reader.find_subagent(session, &wrong), Err(Error::NotFound(_))));
        let mut other = query(&manifest, "probe-a", "A");
        other.profile = "fox-probe:unknown".into();
        assert!(matches!(reader.find_subagent(session, &other), Err(Error::NotFound(_))));
    }
    Connection::open(&path)
        .unwrap()
        .execute(
            "INSERT INTO tool_call_state (session_id, tool_call_id, tool_call_json) SELECT session_id, 'duplicate', tool_call_json FROM tool_call_state WHERE tool_call_json LIKE '%fox-probe:probe-a%'",
            [],
        )
        .unwrap();
    let reader = open(&path);
    assert!(matches!(reader.find_subagent(session, &query(&manifest, "probe-a", "A")), Err(Error::Conflict(_))));
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
    assert!(matches!(reader.find_subagent(session, &query(&manifest, "probe-b", "B")), Err(Error::NotFound(_))));
}

#[test]
fn checks_the_schema_before_reading() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("sessions.db");
    Connection::open(&path).unwrap().execute_batch("CREATE TABLE sessions (id TEXT);").unwrap();
    let error = Reader::open(&path, SchemaMap::load(&root()).unwrap()).err().unwrap();
    assert!(error.to_string().contains("lacks"), "{error}");
    assert!(matches!(
        Reader::open(&dir.path().join("none.db"), SchemaMap::load(&root()).unwrap()),
        Err(Error::NotFound(_))
    ));
}

#[test]
fn locates_the_database_by_priority() {
    let home = Path::new("/home/user");
    let explicit = Some(PathBuf::from("/a.db"));
    let env = Some(PathBuf::from("/b.db"));
    let dir = Some(PathBuf::from("/devin"));
    assert_eq!(locate(explicit.clone(), env.clone(), dir.clone(), home), PathBuf::from("/a.db"));
    assert_eq!(locate(None, env, dir.clone(), home), PathBuf::from("/b.db"));
    assert_eq!(locate(None, None, dir, home), PathBuf::from("/devin/sessions.db"));
    assert_eq!(locate(None, None, None, home), PathBuf::from("/home/user/.local/share/devin/cli/sessions.db"));
}
```

- [ ] **Шаг 2.** Запустить: `cargo test -p foxmorrow-devin --test reader`. Ожидается ошибка компиляции: модуля `reader` нет.

- [ ] **Шаг 3.** `crates/foxmorrow-devin/src/reader.rs`:

```rust
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
            return Err(Error::NotFound(format!("Devin session database {}", path.display())));
        }
        let conn = Connection::open_with_flags(path, OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_NO_MUTEX)?;
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
            .query_row("SELECT MAX(version) FROM refinery_schema_history", [], |row| row.get::<_, Option<i64>>(0))
            .ok()
            .flatten();
        Ok(Self { conn, map, migration })
    }

    pub fn source(&self) -> String {
        let migration = self.migration.map_or_else(|| "unknown".to_string(), |version| version.to_string());
        format!("devin-sessions-db map={} migration={migration}", self.map.map_version)
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
        if let Some(missing) = query.needles.iter().find(|needle| !observed.task_text.contains(needle.as_str())) {
            return invalid(format!("task of subagent {agent_id} does not contain '{missing}'"));
        }
        Ok(observed)
    }

    fn main_chain(&self, session_id: &str) -> Result<Option<i64>> {
        let head: Option<Option<i64>> = self
            .conn
            .query_row("SELECT main_chain_id FROM sessions WHERE id = ?1", [session_id], |row| row.get(0))
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
            if query.needles.iter().all(|needle| task.contains(needle.as_str())) {
                matches.push(id);
            }
        }
        match matches.len() {
            0 => Err(Error::NotFound(format!("run_subagent call with profile {} and the recorded brief", query.profile))),
            1 => Ok(matches.remove(0)),
            count => Err(Error::Conflict(format!("{count} run_subagent calls match the recorded brief"))),
        }
    }

    fn agent_id_of(&self, session_id: &str, call_id: &str) -> Result<String> {
        let mut statement = self
            .conn
            .prepare("SELECT chat_message FROM message_nodes WHERE session_id = ?1 AND instr(chat_message, ?2) > 0")?;
        let rows = statement.query_map(params![session_id, call_id], |row| row.get::<_, String>(0))?;
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
            0 => Err(Error::NotFound(format!("agent id in the result of call {call_id}"))),
            1 => Ok(found.into_iter().next().unwrap_or_default()),
            _ => Err(Error::Conflict(format!("call {call_id} reports several agent ids"))),
        }
    }

    fn observe(&self, session_id: &str, head: i64, agent_id: &str, profile: &str) -> Result<ChainObservation> {
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
        let assistants: Vec<&Message> = messages.iter().filter(|message| message.role == "assistant").collect();
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
            .and_then(|value| value.pointer(&self.map.result.exit_code).and_then(Value::as_i64)))
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
                .query_row(params![session_id, id], |row| Ok((row.get(0)?, row.get(1)?)))
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
```

- [ ] **Шаг 4.** В `crates/foxmorrow-devin/src/lib.rs` добавить `pub mod reader;` после `pub mod profile;`.

- [ ] **Шаг 5.** Запустить: `cargo test -p foxmorrow-devin --test reader`. Ожидается: 7 тестов пройдены. Если `reads_each_probe_chain` падает на `migration`, сверить версию миграции в `manifest.json` (`20`) и значение в тесте.

- [ ] **Шаг 6.** Коммит:

```sh
git add crates/foxmorrow-devin
git commit -m "Read subagent and root chains from the Devin sessions database"
```

## Задача 4. Обработчик hooks

**Файлы:** создать `crates/foxmorrow-devin/src/hook.rs`, `tests/hook.rs`; изменить `src/lib.rs`.

- [ ] **Шаг 1.** Тест `crates/foxmorrow-devin/tests/hook.rs`:

```rust
use std::path::Path;

use foxmorrow_devin::hook::{self, SessionRecord, contains_marker, read_session};

fn run_hook(event: &str, input: &str, root: &Path, state: Option<&str>) -> foxmorrow_core::Result<Option<String>> {
    hook::handle(event, input, root, |_| state.map(str::to_string))
}

fn context(output: &str) -> serde_json::Value {
    serde_json::from_str(output).unwrap()
}

#[test]
fn recognizes_the_launch_markers() {
    for prompt in ["$fox do it", "Please $fox: build", "($foxmorrow) build", "do it\n$fox", "$fox", "$foxmorrow"] {
        assert!(contains_marker(prompt), "{prompt}");
    }
    for prompt in ["$foxes", "$foxmorrow2", "a$fox", "$$fox", "fox", "$fox-trot", "use $foxy"] {
        assert!(!contains_marker(prompt), "{prompt}");
    }
}

#[test]
fn a_marked_prompt_records_the_session_and_asks_for_the_skill() {
    let dir = tempfile::tempdir().unwrap();
    let output = run_hook("UserPromptSubmit", r#"{"session_id":"s-1","prompt_id":"p-1","prompt":"$fox implement"}"#, dir.path(), None)
        .unwrap()
        .unwrap();
    let value = context(&output);
    assert_eq!(value["hookSpecificOutput"]["hookEventName"], "UserPromptSubmit");
    assert!(value["hookSpecificOutput"]["additionalContext"].as_str().unwrap().contains("skill `fox`"));
    assert_eq!(
        read_session(dir.path()),
        Some(SessionRecord { session_id: "s-1".into(), prompt_id: Some("p-1".into()) })
    );
}

#[test]
fn unmarked_prompts_leave_no_trace() {
    let dir = tempfile::tempdir().unwrap();
    let output = run_hook("UserPromptSubmit", r#"{"session_id":"s-1","prompt":"hello"}"#, dir.path(), None).unwrap();
    assert_eq!(output, None);
    assert!(!dir.path().join(".foxmorrow").exists());
}

#[test]
fn compaction_returns_state_only_for_the_known_run() {
    let dir = tempfile::tempdir().unwrap();
    assert_eq!(run_hook("PostCompaction", r#"{"session_id":"s-1"}"#, dir.path(), Some("{}")).unwrap(), None);
    std::fs::create_dir_all(dir.path().join(".foxmorrow")).unwrap();
    std::fs::write(dir.path().join(".foxmorrow/current"), "r-1\n").unwrap();
    let output = run_hook("PostCompaction", r#"{"session_id":"s-1"}"#, dir.path(), Some("{\"run_id\":\"r-1\"}"))
        .unwrap()
        .unwrap();
    let value = context(&output);
    assert_eq!(value["hookSpecificOutput"]["hookEventName"], "PostCompaction");
    assert!(value["hookSpecificOutput"]["additionalContext"].as_str().unwrap().contains("r-1"));
    assert_eq!(run_hook("PostCompaction", r#"{"session_id":"s-1"}"#, dir.path(), None).unwrap(), None);
}

#[test]
fn other_events_and_bad_input() {
    let dir = tempfile::tempdir().unwrap();
    assert_eq!(run_hook("SessionEnd", "{}", dir.path(), None).unwrap(), None);
    assert!(run_hook("UserPromptSubmit", "not json", dir.path(), None).is_err());
}
```

- [ ] **Шаг 2.** Запустить: `cargo test -p foxmorrow-devin --test hook`. Ожидается ошибка компиляции.

- [ ] **Шаг 3.** `crates/foxmorrow-devin/src/hook.rs`:

```rust
use std::path::Path;

use foxmorrow_core::Result;
use serde::{Deserialize, Serialize};

const LAUNCH_CONTEXT: &str = "foxmorrow was requested with $fox. Invoke the skill `fox` (/foxmorrow:fox) before any other action; it runs the task through foxmorrow.";

#[derive(Debug, Default, Deserialize)]
pub struct HookInput {
    #[serde(default)]
    pub session_id: Option<String>,
    #[serde(default)]
    pub prompt_id: Option<String>,
    #[serde(default)]
    pub prompt: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SessionRecord {
    pub session_id: String,
    pub prompt_id: Option<String>,
}

pub fn contains_marker(prompt: &str) -> bool {
    ["$fox", "$foxmorrow"].iter().any(|marker| {
        prompt.match_indices(marker).any(|(start, _)| {
            let before = prompt[..start].chars().next_back();
            let after = prompt[start + marker.len()..].chars().next();
            before.is_none_or(|c| !(c.is_alphanumeric() || c == '_' || c == '$'))
                && after.is_none_or(|c| !(c.is_alphanumeric() || c == '-' || c == '_'))
        })
    })
}

pub fn read_session(project_root: &Path) -> Option<SessionRecord> {
    let text = std::fs::read_to_string(project_root.join(".foxmorrow").join("session")).ok()?;
    serde_json::from_str(&text).ok()
}

pub fn handle(
    event: &str,
    input: &str,
    project_root: &Path,
    run_state: impl FnOnce(Option<&str>) -> Option<String>,
) -> Result<Option<String>> {
    let parsed: HookInput = serde_json::from_str(input)?;
    match event {
        "UserPromptSubmit" if parsed.prompt.as_deref().is_some_and(contains_marker) => {
            if let Some(session_id) = parsed.session_id {
                write_session(project_root, &SessionRecord { session_id, prompt_id: parsed.prompt_id })?;
            }
            Ok(Some(context("UserPromptSubmit", LAUNCH_CONTEXT)))
        }
        "PostCompaction" if project_root.join(".foxmorrow").join("current").is_file() => Ok(run_state(parsed.session_id.as_deref())
            .map(|state| context("PostCompaction", &format!("foxmorrow run state after compaction:\n{state}")))),
        _ => Ok(None),
    }
}

fn write_session(project_root: &Path, record: &SessionRecord) -> Result<()> {
    let dir = project_root.join(".foxmorrow");
    std::fs::create_dir_all(&dir)?;
    std::fs::write(dir.join("session"), serde_json::to_string(record)?)?;
    Ok(())
}

fn context(event: &str, text: &str) -> String {
    serde_json::json!({ "hookSpecificOutput": { "hookEventName": event, "additionalContext": text } }).to_string()
}
```

- [ ] **Шаг 4.** В `lib.rs` добавить `pub mod hook;` между `pub mod agents;` и `pub mod profile;`.

- [ ] **Шаг 5.** Запустить: `cargo test -p foxmorrow-devin --test hook`. Ожидается: 5 тестов пройдены. Обработчик возвращает состояние только при активном run (`.foxmorrow/current`), а принадлежность сессии root проверяет замыкание из CLI (задача 5).

- [ ] **Шаг 6.** Коммит:

```sh
git add crates/foxmorrow-devin
git commit -m "Handle UserPromptSubmit and PostCompaction hooks"
```

## Задача 5. CLI: `init`, `verify`, `finalize`, `hook`

**Файлы:** изменить `crates/foxmorrow/Cargo.toml`, `src/main.rs`, `tests/cli.rs`.

- [ ] **Шаг 1.** В `crates/foxmorrow/Cargo.toml` в `[dev-dependencies]` добавить `rusqlite.workspace = true`.

- [ ] **Шаг 2.** Тесты. В `crates/foxmorrow/tests/cli.rs` в начало добавить импорты `use rusqlite::{Connection, OpenFlags, params};`, изменить строку шаблона брифа в `Setup::new`:

```rust
write(plugin, "templates/brief.en.md", "Role: {{role}} Profile: {{profile}} role-sha256 {{role_sha256}}\nMissing:\n{{missing_skills}}\nGoal: {{goal}}\nfoxmorrow report -a {{assignment_id}}\n");
```

и добавить в конец файла:

```rust
struct SubChain<'a> {
    agent: &'a str,
    profile: &'a str,
    task: &'a str,
    role_hash: &'a str,
    model: &'a str,
    report_command: &'a str,
}

fn build_db(
    dir: &Path,
    session: &str,
    cwd: &str,
    root_model: &str,
    root_report: Option<&str>,
    subs: &[SubChain<'_>],
) -> PathBuf {
    let fixture = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/fixtures/devin-3000.12.43/sessions.db");
    let source = Connection::open_with_flags(&fixture, OpenFlags::SQLITE_OPEN_READ_ONLY).unwrap();
    let ddl: Vec<String> = source
        .prepare("SELECT sql FROM sqlite_master WHERE sql IS NOT NULL AND type IN ('table', 'index')")
        .unwrap()
        .query_map([], |row| row.get(0))
        .unwrap()
        .map(Result::unwrap)
        .collect();
    let path = dir.join("sessions.db");
    let db = Connection::open(&path).unwrap();
    for sql in ddl {
        db.execute_batch(&sql).unwrap();
    }
    let message = |node: i64, parent: Option<i64>, json: serde_json::Value| {
        db.execute(
            "INSERT INTO message_nodes (session_id, node_id, parent_node_id, chat_message, created_at) VALUES (?1, ?2, ?3, ?4, 0)",
            params![session, node, parent, json.to_string()],
        )
        .unwrap();
    };
    let state = |id: &str, request: Option<serde_json::Value>, exit_code: Option<i64>| {
        db.execute(
            "INSERT INTO tool_call_state (session_id, tool_call_id, tool_call_json, tool_call_update_json) VALUES (?1, ?2, ?3, ?4)",
            params![
                session,
                id,
                request.map(|value| value.to_string()),
                exit_code.map(|code| serde_json::json!({"_meta": {"terminal_exit": {"exit_code": code}}}).to_string())
            ],
        )
        .unwrap();
    };
    let mut calls: Vec<serde_json::Value> = subs
        .iter()
        .map(|sub| {
            serde_json::json!({"id": format!("rs-{}", sub.agent), "name": "run_subagent",
                "arguments": {"profile": sub.profile, "task": sub.task}})
        })
        .collect();
    if let Some(command) = root_report {
        calls.push(serde_json::json!({"id": "root-report", "name": "exec", "arguments": {"command": command}}));
        state("root-report", None, Some(0));
    }
    message(1, None, serde_json::json!({"role": "user", "content": "$fox implement"}));
    message(
        2,
        Some(1),
        serde_json::json!({"role": "assistant", "content": "", "metadata": {"generation_model": root_model}, "tool_calls": calls}),
    );
    let mut last = 2;
    for (index, sub) in subs.iter().enumerate() {
        state(
            &format!("rs-{}", sub.agent),
            Some(serde_json::json!({"rawInput": {"profile": sub.profile, "task": sub.task}})),
            None,
        );
        last = 3 + index as i64;
        message(
            last,
            Some(last - 1),
            serde_json::json!({"role": "tool", "tool_call_id": format!("rs-{}", sub.agent),
                "content": format!("Subagent agent_id={} completed successfully:\n done", sub.agent)}),
        );
        let base = 100 * (index as i64 + 1);
        message(base, None, serde_json::json!({"role": "system", "content": format!("foxmorrow-role-sha256: {}\n\nrole text", sub.role_hash)}));
        message(base + 1, Some(base), serde_json::json!({"role": "user", "content": sub.task}));
        message(
            base + 2,
            Some(base + 1),
            serde_json::json!({"role": "assistant", "content": "", "metadata": {"generation_model": sub.model},
                "tool_calls": [{"id": format!("call-{}", sub.agent), "name": "exec", "arguments": {"command": sub.report_command}}]}),
        );
        state(&format!("call-{}", sub.agent), None, Some(0));
        message(base + 3, Some(base + 2), serde_json::json!({"role": "tool", "tool_call_id": format!("call-{}", sub.agent), "content": "ok"}));
        message(
            base + 4,
            Some(base + 3),
            serde_json::json!({"role": "assistant", "content": "done", "metadata": {"generation_model": sub.model}}),
        );
        db.execute(
            "INSERT INTO subagent_heads (session_id, agent_id, chain_node_id, updated_at) VALUES (?1, ?2, ?3, 0)",
            params![session, sub.agent, base + 4],
        )
        .unwrap();
    }
    message(
        last + 1,
        Some(last),
        serde_json::json!({"role": "assistant", "content": "finished", "metadata": {"generation_model": root_model}}),
    );
    db.execute(
        "INSERT INTO sessions (id, working_directory, backend_type, model, agent_mode, created_at, last_activity_at, main_chain_id) VALUES (?1, ?2, 'windsurf', ?3, 'bypass', 0, 1, ?4)",
        params![session, cwd, root_model, last + 1],
    )
    .unwrap();
    path
}

fn db_arg(path: &Path) -> String {
    path.display().to_string()
}

fn brief_hash(brief: &str) -> String {
    brief
        .lines()
        .find(|line| line.starts_with("Role:"))
        .and_then(|line| line.split_whitespace().last())
        .unwrap()
        .to_string()
}

fn author_assignment(setup: &Setup) -> String {
    setup.run(&["init", "--criteria", "task.md#Критерии приёмки", "--paths", "src/", "--session-id", "sess-1"], None);
    text(&setup.run(&["assign", "--role", "backend-worker", "--goal", "Add GET /health", "--paths", "src/"], None))
}

const REPORT: &str = r#"{"summary":"done","status":"completed","outcome":"passed"}"#;

#[test]
fn verify_accepts_a_matching_chain_and_rejects_a_wrong_model() {
    let setup = Setup::new();
    let brief = author_assignment(&setup);
    json(&setup.run(&["report", "-a", "a-01"], Some(REPORT)));
    let hash = brief_hash(&brief);
    let sub = |model, dir: &Path| {
        build_db(
            dir,
            "sess-1",
            "/work",
            "claude-opus-5-5-high",
            None,
            &[SubChain {
                agent: "ag1",
                profile: "foxmorrow:backend-worker",
                task: &brief,
                role_hash: &hash,
                model,
                report_command: "bin/foxmorrow report -a a-01 <<'JSON'",
            }],
        )
    };

    let wrong_dir = tempfile::tempdir().unwrap();
    let wrong = sub("gpt-6-1-sol-high", wrong_dir.path());
    let rejected = setup.run(&["--sessions-db", &db_arg(&wrong), "verify", "-a", "a-01"], None);
    assert_eq!(rejected.status.code(), Some(1));
    let value: serde_json::Value = serde_json::from_slice(&rejected.stdout).unwrap();
    assert_eq!(value["state"], "rejected");
    assert!(value["claims"].as_array().unwrap().iter().any(|claim| claim["id"] == "C3" && claim["state"] == "failed"));

    let good_dir = tempfile::tempdir().unwrap();
    let good = sub("swe-2-high", good_dir.path());
    let accepted = json(&setup.run(&["--sessions-db", &db_arg(&good), "verify", "-a", "a-01"], None));
    assert_eq!(accepted["state"], "accepted");
    assert_eq!(json(&setup.run(&["status"], None))["assignments"][0]["verification"], "accepted");
}

#[test]
fn verify_without_native_evidence_is_unverified() {
    let setup = Setup::new();
    author_assignment(&setup);
    json(&setup.run(&["report", "-a", "a-01"], Some(REPORT)));
    let output = setup.run(&["--sessions-db", "/nonexistent/sessions.db", "verify", "-a", "a-01"], None);
    assert_eq!(output.status.code(), Some(1));
    let value: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(value["state"], "unverified");
    assert!(String::from_utf8_lossy(&output.stderr).contains("no native evidence"));
}

#[test]
fn root_stop_is_verified_and_finalized_with_the_root_model() {
    let setup = Setup::new();
    let dir = tempfile::tempdir().unwrap();
    let db = build_db(dir.path(), "sess-1", "/work", "claude-opus-5-5-high", Some("bin/foxmorrow report -a a-01 <<'JSON'"), &[]);
    let db = db_arg(&db);
    let init = json(&setup.run(
        &["--sessions-db", &db, "init", "--criteria", "task.md#Критерии приёмки", "--paths", "src/", "--session-id", "sess-1"],
        None,
    ));
    assert_eq!(init["root_session_id"], "sess-1");
    assert_eq!(init["root_model"], "claude-opus-5-5-high");
    assert_eq!(json(&setup.run(&["status"], None))["root_model"], "claude-opus-5-5-high");

    json(&setup.run(&["assign", "--root", "--paths", "src/"], None));
    let stop = r#"{"summary":"stopped","status":"completed","outcome":"unverified","reason":"no browser access","unverified":["ac-1"]}"#;
    json(&setup.run(&["report", "-a", "a-01"], Some(stop)));
    assert_eq!(json(&setup.run(&["--sessions-db", &db, "verify", "-a", "a-01"], None))["state"], "accepted");
    let record = json(&setup.run(&["--sessions-db", &db, "finalize", "--report", "a-01"], None));
    assert_eq!(record["outcome"], "unverified");
    assert_eq!(record["root_model"], "claude-opus-5-5-high");
    assert_eq!(json(&setup.run(&["status"], None))["final_outcome"], "unverified");
}

#[test]
fn hooks_record_the_session_and_report_state() {
    let setup = Setup::new();
    let marked = text(&setup.run(
        &["hook", "UserPromptSubmit"],
        Some(r#"{"session_id":"sess-1","prompt_id":"p-1","prompt":"$fox implement the plan"}"#),
    ));
    assert!(marked.contains("skill `fox`"), "{marked}");
    let session = std::fs::read_to_string(setup.project.path().join(".foxmorrow/session")).unwrap();
    assert!(session.contains("sess-1"));

    let quiet = setup.run(&["hook", "UserPromptSubmit"], Some(r#"{"session_id":"sess-1","prompt":"hello"}"#));
    assert!(quiet.status.success() && quiet.stdout.is_empty());

    json(&setup.run(&["init", "--criteria", "task.md#Критерии приёмки", "--paths", "src/"], None));
    let state = text(&setup.run(&["hook", "PostCompaction"], Some(r#"{"session_id":"sess-1"}"#)));
    assert!(state.contains("run_id"), "{state}");
    let foreign = setup.run(&["hook", "PostCompaction"], Some(r#"{"session_id":"other"}"#));
    assert!(foreign.status.success() && foreign.stdout.is_empty());

    let broken = setup.run(&["hook", "UserPromptSubmit"], Some("not json"));
    assert!(broken.status.success() && broken.stdout.is_empty());
    assert!(String::from_utf8_lossy(&broken.stderr).contains("hook UserPromptSubmit"));
}
```

- [ ] **Шаг 3.** Запустить: `cargo test -p foxmorrow --test cli`. Ожидается: ошибки компиляции или падения новых тестов: нет `--sessions-db`, `verify`, `finalize`, `hook`.

- [ ] **Шаг 4.** `crates/foxmorrow/src/main.rs`. Добавить импорты:

```rust
use foxmorrow_core::assignment::Assignment;
use foxmorrow_core::claims::{self, ChainObservation, ModelFamilies};
use foxmorrow_core::finalize;
use foxmorrow_core::journal::Journal;
use foxmorrow_core::model::VerificationState;
use foxmorrow_devin::hook;
use foxmorrow_devin::reader::{self, ChainQuery, Reader};
use foxmorrow_devin::schema_map::SchemaMap;
```

В структуру `Cli` после поля `plugin_root` добавить:

```rust
    #[arg(long, global = true)]
    sessions_db: Option<PathBuf>,
```

В перечисление `Command` добавить варианты:

```rust
    Verify {
        #[arg(short = 'a', long = "assignment")]
        assignment: String,
        #[arg(long)]
        run: Option<String>,
    },
    Finalize {
        #[arg(long = "report")]
        report: String,
        #[arg(long)]
        run: Option<String>,
    },
    Hook {
        event: String,
    },
```

Ветку `Command::Init { … }` заменить на:

```rust
        Command::Init { criteria: reference, paths, repos, session_id } => {
            let root = project::for_init(cli.project, &cwd);
            let plugin = Plugin::load(cli.plugin_root)?;
            let project_profile = ProjectProfile::load(&root)?;
            let found = inventory(&root);
            let skills_summary = missing_required(&plugin.catalog, &found);
            let (file, heading) = criteria::split_reference(&reference)?;
            let markdown = std::fs::read_to_string(root.join(file))?;
            let repos = if repos.is_empty() { vec![root.clone()] } else { repos };
            let mut baselines = Vec::new();
            for repo in &repos {
                baselines.push(baseline::collect(repo)?);
            }
            let evidence = open_reader(cli.sessions_db, &plugin.root);
            let directory = root.canonicalize().unwrap_or_else(|_| root.clone()).display().to_string();
            let session = session_id
                .or_else(|| hook::read_session(&root).map(|record| record.session_id))
                .or_else(|| evidence.as_ref().ok().and_then(|reader| reader.latest_session_in(&directory).ok().flatten()));
            let root_model = match (&evidence, &session) {
                (Ok(reader), Some(session)) => reader.root_model(session).ok().flatten(),
                _ => None,
            };
            let (created, mut journal) = run::create_run(NewRun {
                project_root: root,
                criteria_path: file.to_string(),
                criteria_heading: heading.to_string(),
                criteria_markdown: markdown,
                allowed_paths: paths,
                baselines,
                root_session_id: session,
                family_rule: plugin.profile.family_rule(&project_profile),
                skills_summary: skills_summary.clone(),
            })?;
            let created = if root_model.is_some() { run::record_root(&mut journal, None, root_model.as_deref())? } else { created };
            print_json(&serde_json::json!({
                "run": created.id,
                "criteria": created.criteria,
                "family_rule": created.family_rule,
                "skills_source": found.source,
                "missing_skills": skills_summary,
                "root_session_id": created.root_session_id,
                "root_model": created.root_model,
                "evidence": evidence.as_ref().map_or_else(|reason| format!("unavailable: {reason}"), Reader::source),
            }))?;
        }
```

Перед веткой `Command::Sync` добавить ветки:

```rust
        Command::Verify { assignment: id, run: run_id } => {
            let project_root = project::for_existing(cli.project, &cwd)?;
            let (current, mut journal) = run::open_run(&project_root, run_id.as_deref())?;
            let plugin = Plugin::load(cli.plugin_root)?;
            let evidence = open_reader(cli.sessions_db, &plugin.root);
            let target = assignment::get(journal.conn(), &id)?;
            let session = current
                .root_session_id
                .clone()
                .or_else(|| hook::read_session(&project_root).map(|record| record.session_id));
            let observation = match (&evidence, &session) {
                (Ok(reader), Some(session)) => {
                    if current.root_model.is_none() {
                        let model = reader.root_model(session).ok().flatten();
                        run::record_root(&mut journal, Some(session), model.as_deref())?;
                    }
                    observe(reader, session, &target, &journal)
                }
                (Err(reason), _) => {
                    eprintln!("foxmorrow: no native evidence: {reason}");
                    None
                }
                (_, None) => {
                    eprintln!("foxmorrow: no native evidence: the root session id is unknown; pass --session-id to init");
                    None
                }
            };
            let extra = observation.as_ref().map(|observed| observed.models.clone()).unwrap_or_default();
            let families = model_families(&journal, &plugin.profile, &extra)?;
            let verification = claims::verify(&mut journal, &current, &id, observation, &families)?;
            print_json(&serde_json::json!({
                "assignment": verification.assignment_id,
                "state": verification.state,
                "claims": verification.claims,
            }))?;
            if verification.state != VerificationState::Accepted {
                return Ok(ExitCode::from(1));
            }
        }
        Command::Finalize { report: id, run: run_id } => {
            let project_root = project::for_existing(cli.project, &cwd)?;
            let (current, mut journal) = run::open_run(&project_root, run_id.as_deref())?;
            let plugin = Plugin::load(cli.plugin_root)?;
            let families = model_families(&journal, &plugin.profile, &[])?;
            print_json(&finalize::finalize(&mut journal, &current, &id, &families)?)?;
        }
        Command::Hook { event } => {
            let mut input = String::new();
            let _ = std::io::stdin().read_to_string(&mut input);
            let project_root = std::env::var_os("DEVIN_PROJECT_DIR")
                .map(PathBuf::from)
                .unwrap_or_else(|| project::for_init(None, &cwd));
            match hook::handle(&event, &input, &project_root, |session| compact_status(&project_root, session)) {
                Ok(Some(output)) => println!("{output}"),
                Ok(None) => {}
                Err(error) => eprintln!("foxmorrow: hook {event}: {error}"),
            }
        }
```

Для `Serialize` в `claims::Claim` и `Verification`: `Claim` уже `Serialize`, а `Verification` — нет. В выводе `verify` используются только их поля, поэтому изменений в ядре не нужно. `VerificationState` реализует `Serialize` (см. `model.rs`).

Добавить вспомогательные функции в конец файла:

```rust
fn open_reader(explicit: Option<PathBuf>, plugin_root: &Path) -> std::result::Result<Reader, String> {
    let map = SchemaMap::load(plugin_root).map_err(|error| error.to_string())?;
    let path = reader::locate(
        explicit,
        std::env::var_os("FOXMORROW_SESSIONS_DB").map(PathBuf::from),
        std::env::var_os("DEVIN_SESSIONS_DIR").map(PathBuf::from),
        &home_dir(),
    );
    Reader::open(&path, map).map_err(|error| error.to_string())
}

fn observe(reader: &Reader, session: &str, target: &Assignment, journal: &Journal) -> Option<ChainObservation> {
    let found = if target.kind == Kind::Root {
        reader.root_observation(session)
    } else {
        let header = assignment::brief(journal.conn(), &target.id)
            .ok()
            .flatten()
            .and_then(|(_, text)| text.lines().next().map(str::to_string));
        match (header, &target.profile) {
            (Some(header), Some(profile)) => {
                reader.find_subagent(session, &ChainQuery { profile: profile.clone(), needles: vec![header] })
            }
            _ => Err(Error::NotFound("the recorded brief or profile".into())),
        }
    };
    match found {
        Ok(observed) => Some(observed),
        Err(error) => {
            eprintln!("foxmorrow: no native evidence for {}: {error}", target.id);
            None
        }
    }
}

fn model_families(journal: &Journal, profile: &Profile, extra: &[String]) -> Result<ModelFamilies> {
    let mut models = claims::observed_models(journal.conn())?;
    for entry in assignment::list(journal.conn())? {
        models.extend(entry.expected_models);
    }
    models.extend(extra.iter().cloned());
    Ok(profile.families_for(models.iter()))
}

fn compact_status(project_root: &Path, session: Option<&str>) -> Option<String> {
    let (current, journal) = run::open_run(project_root, None).ok()?;
    let known = current
        .root_session_id
        .clone()
        .or_else(|| hook::read_session(project_root).map(|record| record.session_id));
    if session.is_none() || known.as_deref() != session {
        return None;
    }
    serde_json::to_string(&status::status(journal.conn(), &current).ok()?).ok()
}
```

В ветке `Command::Check` и `Command::Report` `cli.project` используется как прежде; импорт `Error` уже есть в файле.

- [ ] **Шаг 5.** Запустить: `cargo test -p foxmorrow --test cli`. Ожидается: 10 тестов пройдены (6 прежних и 4 новых).

- [ ] **Шаг 6.** Коммит:

```sh
git add crates/foxmorrow
git commit -m "Verify and finalize runs against Devin session evidence"
```

## Задача 6. Итоговая проверка

- [ ] **Шаг 1.**

```sh
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
rg -n '^\s*//' crates --glob '*.rs'
```

Ожидается: первые три команды завершаются с кодом 0, последняя ничего не находит. Если `cargo fmt --check` нашёл расхождения, выполнить `cargo fmt --all` и проверить diff. Предупреждения `clippy` исправляются в коде.

- [ ] **Шаг 2.** Проверка на настоящей базе без изменения данных. Эта команда читает `sessions.db` пользователя только на чтение и ничего не пишет:

```sh
sqlite3 -readonly "file:$HOME/.local/share/devin/cli/sessions.db?mode=ro" "select count(*) from subagent_heads"
```

Ожидается число без ошибки. Проверка reader на реальной базе с живой цепочкой выполняется в плане 005 вместе с проверкой hooks в Devin.

- [ ] **Шаг 3.** Сверить реализацию с разделом «Критерии приёмки»: для каждого пункта назвать тест или команду, которые его подтверждают.

- [ ] **Шаг 4.** Коммит исправлений, если они были:

```sh
git add -A crates schema-maps Cargo.lock
git commit -m "Fix formatting and lint findings"
```

## Что остаётся непроверенным после плана

- Поведение hooks и вывода `additionalContext` для `PostCompaction` в живом Devin (план 005).
- Работа reader на живой базе во время работы Devin, при блокировках WAL (план 005).
- Путь `sessions.db` и поведение на Windows.
- Режим `DEVIN_SESSION_STORE=files`: база не создаётся, `verify` вернёт `unverified`.
