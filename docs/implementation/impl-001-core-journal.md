# План 001. Ядро и CLI

Document status: approved
Based on: [foxmorrow v1: дизайн](../design/2026-11-10-foxmorrow-v1-design.md), [дорожная карта](roadmap-v1.md)

Задачи выполняются по порядку, шаги отмечаются чекбоксами `- [ ]`. Коммиты из шагов «Коммит» выполняются, только если пользователь разрешил коммиты в этом run.

**Цель:** Cargo workspace с крейтом `foxmorrow-core` и бинарником `foxmorrow`. Журнал задачи в SQLite, команды `init`, `assign`, `check`, `report`, `status`, правила порядка и неизменяемости, оценка утверждений C1–C6 по нормализованному наблюдению и `finalize` в библиотеке.

**Архитектура:** ядро ничего не знает о Devin. Источник наблюдений (план 002) передаёт в `claims::verify` нормализованный `ChainObservation`. Ядро сверяет его с назначением, отчётом и квитанциями и записывает результат вместе со снимком наблюдения. `finalize` перепроверяет сохранённые снимки. Неизменяемость обеспечивают триггеры SQLite. Каждая запись идёт в транзакции `BEGIN IMMEDIATE`.

**Стек:** Rust 1.99.1 (edition 2024), `rusqlite` 0.41.1 со встроенным SQLite, `serde` 1.0.229, `serde_json` 1.0.154, `sha2` 0.11.1, `clap` 4.6.7, в тестах `tempfile` 3.27.0.

**Вне плана:** чтение `sessions.db`, hooks, CLI `verify` и `finalize` (план 002); каталог ролей, брифы, вычисление профиля, моделей и хешей в `assign` (план 003); `init --doc` и `init --plan` (план 004); CI и выпуск (план 005).

## Критерии приёмки

- `cargo fmt --all --check`, `cargo clippy --workspace --all-targets -- -D warnings` и `cargo test --workspace` завершаются успешно на macOS.
- `foxmorrow init --criteria <файл>#<заголовок> --paths …` создаёт `.foxmorrow/runs/<run_id>/journal.sqlite` и `.foxmorrow/current`, записывает критерии `ac-N` из плоского списка раздела, хеш раздела, разрешённые пути и базовое состояние git (HEAD, remote refs, незакоммиченные изменения без `.foxmorrow/`).
- `assign` отклоняет пути вне разрешённых путей run и пути с `..`, требует профиль и ожидаемую модель для всех видов назначений, кроме `root`, и соблюдает порядок: `review` зависит только от принятых завершённых назначений `author`, `qa` требует принятого `review` с исходом `passed`, исправление (`--previous`) имеет тот же вид и ссылается на назначение с отчётом.
- `check` выполняет команду один раз и записывает квитанцию с cwd, argv, exit code, временем, SHA-256 и последними 16 КиБ вывода. Если записи результата нет, квитанция остаётся без него. `foxmorrow` завершается с exit code команды.
- `report` принимает один отчёт на назначение и отклоняет:
  - пустой `summary`;
  - `passed` без `completed`;
  - неизвестные критерии и критерии без `reason`;
  - QA без полного покрытия критериев;
  - ссылки на чужие или незавершённые квитанции;
  - успешные ссылки на квитанции с ненулевым exit code;
  - отчёт `completed` при квитанции без результата;
  - отчёт root об остановке без `reason`.
- `claims::verify` даёт `accepted`, только если C1–C6 подтверждены; `rejected` при любом `failed`; `unverified` при отсутствии наблюдения или данных. Повторная проверка после `accepted` отклоняется. Правило о семействах работает для `review` и `qa`, если оно включено в run.
- `finalize::finalize` принимает только принятый `qa` с `passed` либо принятый отчёт `root` с неположительным исходом, перепроверяет снимки всего замыкания `depends_on` и закрывает run. После final новые назначения, квитанции, отчёты и проверки отклоняются.
- Записи `assignment`, `report`, `verification`, `final` нельзя изменить или удалить; квитанцию можно дополнить результатом один раз.
- В коде нет поясняющих комментариев.

## Структура файлов

```
rust-toolchain.toml
Cargo.toml
.gitignore
crates/foxmorrow-core/Cargo.toml
crates/foxmorrow-core/src/lib.rs          объявления модулей
crates/foxmorrow-core/src/error.rs        Error, Result, invalid
crates/foxmorrow-core/src/util.rs         sha256_hex, now_ms, new_run_id
crates/foxmorrow-core/src/model.rs        Kind, Status, Outcome, VerificationState
crates/foxmorrow-core/src/criteria.rs     разбор раздела критериев
crates/foxmorrow-core/src/schema.rs       DDL и триггеры неизменяемости
crates/foxmorrow-core/src/journal.rs      открытие журнала, транзакции записи
crates/foxmorrow-core/src/state.rs        запросы состояния отчётов и проверок
crates/foxmorrow-core/src/baseline.rs     базовое состояние git
crates/foxmorrow-core/src/run.rs          создание и открытие run
crates/foxmorrow-core/src/assignment.rs   назначения и правила порядка
crates/foxmorrow-core/src/receipt.rs      выполнение check и квитанции
crates/foxmorrow-core/src/report.rs       отчёты и их проверка
crates/foxmorrow-core/src/claims.rs       ChainObservation, C1–C6, verify
crates/foxmorrow-core/src/finalize.rs     final
crates/foxmorrow-core/src/status.rs       сводка состояния run
crates/foxmorrow-core/tests/common/mod.rs общие помощники тестов
crates/foxmorrow-core/tests/*.rs          тесты по модулям
crates/foxmorrow/Cargo.toml
crates/foxmorrow/src/main.rs              CLI
crates/foxmorrow/src/project.rs           поиск корня проекта
crates/foxmorrow/tests/cli.rs             сквозной тест бинарника
```

## Задача 1. Workspace и toolchain

**Файлы:** создать `rust-toolchain.toml`, `Cargo.toml`, `.gitignore`, `crates/foxmorrow-core/Cargo.toml`, `crates/foxmorrow-core/src/lib.rs`, `crates/foxmorrow/Cargo.toml`, `crates/foxmorrow/src/main.rs`.

- [ ] **Шаг 1.** `rust-toolchain.toml`:

```toml
[toolchain]
channel = "1.99.1"
components = ["rustfmt", "clippy"]
```

- [ ] **Шаг 2.** Установить toolchain (нужна сеть):

```sh
cd /Users/ucnlejumper/Sites/foxmorrow && rustup toolchain install 1.99.1 --component rustfmt clippy && cargo --version
```

Ожидается: `cargo 1.99.1 …`.

- [ ] **Шаг 3.** `Cargo.toml`:

```toml
[workspace]
resolver = "3"
members = ["crates/foxmorrow-core", "crates/foxmorrow"]

[workspace.package]
version = "0.1.0"
edition = "2024"
license = "MIT"
rust-version = "1.99"

[workspace.dependencies]
rusqlite = { version = "0.41.1", features = ["bundled"] }
serde = { version = "1.0.229", features = ["derive"] }
serde_json = "1.0.154"
sha2 = "0.11.1"
clap = { version = "4.6.7", features = ["derive"] }
tempfile = "3.27.0"
foxmorrow-core = { path = "crates/foxmorrow-core" }
```

- [ ] **Шаг 4.** `.gitignore`:

```gitignore
/target
```

- [ ] **Шаг 5.** `crates/foxmorrow-core/Cargo.toml`:

```toml
[package]
name = "foxmorrow-core"
version.workspace = true
edition.workspace = true
license.workspace = true
rust-version.workspace = true

[dependencies]
rusqlite.workspace = true
serde.workspace = true
serde_json.workspace = true
sha2.workspace = true

[dev-dependencies]
tempfile.workspace = true
```

- [ ] **Шаг 6.** `crates/foxmorrow-core/src/lib.rs` (модули добавляются в следующих задачах):

```rust
pub mod error;

pub use error::{Error, Result};
```

- [ ] **Шаг 7.** `crates/foxmorrow/Cargo.toml`:

```toml
[package]
name = "foxmorrow"
version.workspace = true
edition.workspace = true
license.workspace = true
rust-version.workspace = true

[[bin]]
name = "foxmorrow"
path = "src/main.rs"

[dependencies]
foxmorrow-core.workspace = true
clap.workspace = true
serde.workspace = true
serde_json.workspace = true

[dev-dependencies]
tempfile.workspace = true
```

- [ ] **Шаг 8.** Временный `crates/foxmorrow/src/main.rs` (полностью заменяется в задаче 12):

```rust
use clap::Parser;

#[derive(Parser)]
#[command(name = "foxmorrow", version)]
struct Cli {}

fn main() {
    Cli::parse();
}
```

- [ ] **Шаг 9.** Заглушка `error.rs`, чтобы workspace собрался (полное содержание в задаче 2):

```rust
pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug)]
pub enum Error {}
```

- [ ] **Шаг 10.** Проверить:

```sh
cargo run -q -p foxmorrow -- --version
```

Ожидается: `foxmorrow 0.1.0`. Появится `Cargo.lock`; он входит в коммит.

- [ ] **Шаг 11.** Коммит:

```sh
git add rust-toolchain.toml Cargo.toml Cargo.lock .gitignore crates
git commit -m "Set up the foxmorrow Cargo workspace"
```

## Задача 2. Ошибки, утилиты, модель статусов

**Файлы:** изменить `crates/foxmorrow-core/src/error.rs`, `lib.rs`; создать `util.rs`, `model.rs`, тест `crates/foxmorrow-core/tests/model.rs`.

- [ ] **Шаг 1.** Тест `crates/foxmorrow-core/tests/model.rs`:

```rust
use foxmorrow_core::Error;
use foxmorrow_core::model::{Kind, Outcome, Status, VerificationState};
use foxmorrow_core::util::{new_run_id, sha256_hex};

#[test]
fn sha256_matches_the_standard_vector() {
    assert_eq!(
        sha256_hex(b"abc"),
        "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
    );
}

#[test]
fn run_ids_are_short_and_unique() {
    let first = new_run_id();
    let second = new_run_id();
    assert!(first.starts_with("r-"));
    assert_eq!(first.len(), 12);
    assert_ne!(first, second);
}

#[test]
fn statuses_round_trip_through_their_names() {
    for kind in [Kind::Author, Kind::Review, Kind::Qa, Kind::Root] {
        assert_eq!(Kind::parse(kind.as_str()).unwrap(), kind);
    }
    assert_eq!(Status::parse("interrupted").unwrap(), Status::Interrupted);
    assert_eq!(Outcome::parse("unverified").unwrap(), Outcome::Unverified);
    assert_eq!(VerificationState::parse("rejected").unwrap(), VerificationState::Rejected);
    assert_eq!(serde_json::to_string(&Outcome::Passed).unwrap(), "\"passed\"");
}

#[test]
fn unknown_names_are_invalid() {
    assert!(matches!(Kind::parse("orchestrator"), Err(Error::Invalid(_))));
}
```

- [ ] **Шаг 2.** Запустить и убедиться, что тест не компилируется:

```sh
cargo test -p foxmorrow-core --test model
```

Ожидается: ошибка компиляции `unresolved import foxmorrow_core::model`.

- [ ] **Шаг 3.** `crates/foxmorrow-core/src/error.rs`:

```rust
use std::fmt;

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug)]
pub enum Error {
    Sqlite(rusqlite::Error),
    Io(std::io::Error),
    Json(serde_json::Error),
    Invalid(String),
    NotFound(String),
    Conflict(String),
}

pub fn invalid<T>(message: impl Into<String>) -> Result<T> {
    Err(Error::Invalid(message.into()))
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::Sqlite(error) => write!(f, "journal storage error: {error}"),
            Error::Io(error) => write!(f, "io error: {error}"),
            Error::Json(error) => write!(f, "invalid json: {error}"),
            Error::Invalid(message) => write!(f, "invalid: {message}"),
            Error::NotFound(message) => write!(f, "not found: {message}"),
            Error::Conflict(message) => write!(f, "conflict: {message}"),
        }
    }
}

impl std::error::Error for Error {}

impl From<rusqlite::Error> for Error {
    fn from(error: rusqlite::Error) -> Self {
        Error::Sqlite(error)
    }
}

impl From<std::io::Error> for Error {
    fn from(error: std::io::Error) -> Self {
        Error::Io(error)
    }
}

impl From<serde_json::Error> for Error {
    fn from(error: serde_json::Error) -> Self {
        Error::Json(error)
    }
}
```

- [ ] **Шаг 4.** `crates/foxmorrow-core/src/util.rs`:

```rust
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

use sha2::{Digest, Sha256};

static RUN_COUNTER: AtomicU64 = AtomicU64::new(0);

pub fn sha256_hex(data: &[u8]) -> String {
    Sha256::digest(data).iter().map(|byte| format!("{byte:02x}")).collect()
}

pub fn now_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|elapsed| elapsed.as_millis() as i64)
        .unwrap_or(0)
}

pub fn new_run_id() -> String {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|elapsed| elapsed.as_nanos())
        .unwrap_or(0);
    let counter = RUN_COUNTER.fetch_add(1, Ordering::Relaxed);
    let seed = format!("{nanos}-{}-{counter}", std::process::id());
    format!("r-{}", &sha256_hex(seed.as_bytes())[..10])
}
```

- [ ] **Шаг 5.** `crates/foxmorrow-core/src/model.rs`:

```rust
use serde::{Deserialize, Serialize};

use crate::error::{Result, invalid};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Kind {
    Author,
    Review,
    Qa,
    Root,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Status {
    Completed,
    Interrupted,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Outcome {
    Passed,
    Failed,
    Unverified,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum VerificationState {
    Accepted,
    Rejected,
    Unverified,
}

fn parse_named<T: Copy>(value: &str, all: &[T], name: fn(T) -> &'static str, what: &str) -> Result<T> {
    match all.iter().copied().find(|candidate| name(*candidate) == value) {
        Some(found) => Ok(found),
        None => invalid(format!("unknown {what} '{value}'")),
    }
}

impl Kind {
    pub fn as_str(self) -> &'static str {
        match self {
            Kind::Author => "author",
            Kind::Review => "review",
            Kind::Qa => "qa",
            Kind::Root => "root",
        }
    }

    pub fn parse(value: &str) -> Result<Self> {
        parse_named(value, &[Kind::Author, Kind::Review, Kind::Qa, Kind::Root], Self::as_str, "assignment kind")
    }

    pub fn is_verification(self) -> bool {
        matches!(self, Kind::Review | Kind::Qa)
    }
}

impl Status {
    pub fn as_str(self) -> &'static str {
        match self {
            Status::Completed => "completed",
            Status::Interrupted => "interrupted",
        }
    }

    pub fn parse(value: &str) -> Result<Self> {
        parse_named(value, &[Status::Completed, Status::Interrupted], Self::as_str, "report status")
    }
}

impl Outcome {
    pub fn as_str(self) -> &'static str {
        match self {
            Outcome::Passed => "passed",
            Outcome::Failed => "failed",
            Outcome::Unverified => "unverified",
        }
    }

    pub fn parse(value: &str) -> Result<Self> {
        parse_named(value, &[Outcome::Passed, Outcome::Failed, Outcome::Unverified], Self::as_str, "outcome")
    }
}

impl VerificationState {
    pub fn as_str(self) -> &'static str {
        match self {
            VerificationState::Accepted => "accepted",
            VerificationState::Rejected => "rejected",
            VerificationState::Unverified => "unverified",
        }
    }

    pub fn parse(value: &str) -> Result<Self> {
        parse_named(
            value,
            &[VerificationState::Accepted, VerificationState::Rejected, VerificationState::Unverified],
            Self::as_str,
            "verification state",
        )
    }
}
```

- [ ] **Шаг 6.** `crates/foxmorrow-core/src/lib.rs`:

```rust
pub mod error;
pub mod model;
pub mod util;

pub use error::{Error, Result};
```

- [ ] **Шаг 7.** Запустить:

```sh
cargo test -p foxmorrow-core --test model
```

Ожидается: 4 теста пройдены.

- [ ] **Шаг 8.** Коммит:

```sh
git add crates/foxmorrow-core
git commit -m "Add core errors, hashing and status model"
```

## Задача 3. Разбор критериев приёмки

Критерии берутся из плоского списка раздела. Вложенные пункты, строки в блоках кода и подразделы не входят. Раздел заканчивается на следующем заголовке любого уровня.

**Файлы:** создать `crates/foxmorrow-core/src/criteria.rs`, тест `crates/foxmorrow-core/tests/criteria.rs`; изменить `lib.rs`.

- [ ] **Шаг 1.** Тест:

```rust
use foxmorrow_core::Error;
use foxmorrow_core::criteria::{parse_section, split_reference};

const TASK: &str = "# Задача\n\nОписание.\n\n## Критерии приёмки\n\n- GET /health возвращает 200.\n  - вложенный пункт не считается\n- Ответ содержит версию.\n\n```text\n- строка в блоке кода\n```\n\n### Подробности\n\n- пункт подраздела не входит\n\n## Риски\n\n- не входит\n";

#[test]
fn takes_flat_items_of_the_section_only() {
    let (_, criteria) = parse_section(TASK, "критерии приёмки").unwrap();
    let pairs: Vec<(&str, &str)> = criteria.iter().map(|c| (c.id.as_str(), c.text.as_str())).collect();
    assert_eq!(pairs, vec![("ac-1", "GET /health возвращает 200."), ("ac-2", "Ответ содержит версию.")]);
}

#[test]
fn returns_the_section_text_for_hashing() {
    let (section, _) = parse_section(TASK, "Критерии приёмки").unwrap();
    assert!(section.contains("- Ответ содержит версию."));
    assert!(!section.contains("Подробности"));
}

#[test]
fn accepts_numbered_lists() {
    let (_, criteria) = parse_section("## Критерии приёмки\n\n1. Первый\n2. Второй\n", "Критерии приёмки").unwrap();
    assert_eq!(criteria.len(), 2);
    assert_eq!(criteria[1].text, "Второй");
}

#[test]
fn reports_missing_heading_and_empty_section() {
    assert!(matches!(parse_section(TASK, "Нет такого"), Err(Error::NotFound(_))));
    assert!(matches!(parse_section("## Критерии приёмки\n\nТекст.\n", "Критерии приёмки"), Err(Error::Invalid(_))));
}

#[test]
fn splits_file_and_heading() {
    assert_eq!(split_reference("docs/task.md#Критерии приёмки").unwrap(), ("docs/task.md", "Критерии приёмки"));
    assert!(split_reference("docs/task.md").is_err());
    assert!(split_reference("#heading").is_err());
}
```

- [ ] **Шаг 2.** Запустить: `cargo test -p foxmorrow-core --test criteria`. Ожидается ошибка компиляции: модуля `criteria` нет.

- [ ] **Шаг 3.** `crates/foxmorrow-core/src/criteria.rs`:

```rust
use serde::{Deserialize, Serialize};

use crate::error::{Error, Result, invalid};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Criterion {
    pub id: String,
    pub text: String,
}

pub fn split_reference(reference: &str) -> Result<(&str, &str)> {
    match reference.split_once('#') {
        Some((path, heading)) if !path.is_empty() && !heading.is_empty() => Ok((path, heading)),
        _ => invalid(format!("criteria reference '{reference}' must look like <file>#<heading>")),
    }
}

pub fn parse_section(markdown: &str, heading: &str) -> Result<(String, Vec<Criterion>)> {
    let wanted = heading.trim().to_lowercase();
    let mut in_fence = false;
    let mut found = false;
    let mut started = false;
    let mut section = Vec::new();
    let mut texts = Vec::new();
    for line in markdown.lines() {
        if line.trim_start().starts_with("```") {
            in_fence = !in_fence;
        } else if !in_fence && let Some(title) = heading_text(line) {
            if started {
                break;
            }
            started = title.to_lowercase() == wanted;
            found |= started;
            continue;
        } else if started && !in_fence && let Some(text) = bullet_text(line) {
            texts.push(text);
        }
        if started {
            section.push(line);
        }
    }
    if !found {
        return Err(Error::NotFound(format!("heading '{heading}'")));
    }
    if texts.is_empty() {
        return invalid(format!("section '{heading}' has no top-level list items"));
    }
    let criteria = texts
        .into_iter()
        .enumerate()
        .map(|(index, text)| Criterion { id: format!("ac-{}", index + 1), text })
        .collect();
    Ok((section.join("\n"), criteria))
}

fn heading_text(line: &str) -> Option<String> {
    let hashes = line.bytes().take_while(|byte| *byte == b'#').count();
    if !(1..=6).contains(&hashes) {
        return None;
    }
    let title = line[hashes..].strip_prefix(' ')?;
    Some(title.trim().trim_end_matches('#').trim().to_string())
}

fn bullet_text(line: &str) -> Option<String> {
    let rest = line
        .strip_prefix("- ")
        .or_else(|| line.strip_prefix("* "))
        .or_else(|| numbered(line))?;
    let text = rest.trim();
    (!text.is_empty()).then(|| text.to_string())
}

fn numbered(line: &str) -> Option<&str> {
    let digits = line.bytes().take_while(u8::is_ascii_digit).count();
    if digits == 0 {
        return None;
    }
    line[digits..].strip_prefix(". ")
}
```

- [ ] **Шаг 4.** Добавить `pub mod criteria;` в `lib.rs` после `pub mod error;`.

- [ ] **Шаг 5.** Запустить: `cargo test -p foxmorrow-core --test criteria`. Ожидается: 5 тестов пройдены.

- [ ] **Шаг 6.** Коммит:

```sh
git add crates/foxmorrow-core
git commit -m "Parse acceptance criteria from a markdown section"
```

## Задача 4. Схема, журнал, запросы состояния

**Файлы:** создать `schema.rs`, `journal.rs`, `state.rs`, тест `crates/foxmorrow-core/tests/journal.rs`; изменить `lib.rs`.

- [ ] **Шаг 1.** Тест:

```rust
use foxmorrow_core::Error;
use foxmorrow_core::journal::Journal;

const SEED: &str = "INSERT INTO run VALUES ('r', 1, '/p', 'task.md', 'h', 's', '[]', '[]', '[]', NULL, NULL, 1);
INSERT INTO assignment VALUES ('a-01', 1, 1, 'backend-worker', 'author', 'p', '[]', NULL, NULL, '[]', '[]', NULL);
INSERT INTO report VALUES ('a-01', 1, 'completed', 'passed', '{}', 'x');";

#[test]
fn creates_and_reopens_a_journal() {
    let dir = tempfile::tempdir().unwrap();
    Journal::create(dir.path()).unwrap();
    let journal = Journal::open(dir.path()).unwrap();
    assert!(dir.path().join("journal.sqlite").is_file());
    assert_eq!(journal.dir(), dir.path());
}

#[test]
fn refuses_missing_duplicate_and_foreign_journals() {
    let dir = tempfile::tempdir().unwrap();
    assert!(matches!(Journal::open(dir.path()), Err(Error::NotFound(_))));
    let journal = Journal::create(dir.path()).unwrap();
    assert!(matches!(Journal::create(dir.path()), Err(Error::Conflict(_))));
    journal.conn().execute("UPDATE meta SET value = '999' WHERE key = 'contract_version'", []).unwrap();
    assert!(matches!(Journal::open(dir.path()), Err(Error::Invalid(_))));
}

#[test]
fn accepted_records_cannot_change() {
    let dir = tempfile::tempdir().unwrap();
    let journal = Journal::create(dir.path()).unwrap();
    journal.conn().execute_batch(SEED).unwrap();
    let error = journal.conn().execute("UPDATE report SET outcome = 'failed'", []).unwrap_err();
    assert!(error.to_string().contains("immutable: report"));
    let error = journal.conn().execute("DELETE FROM assignment", []).unwrap_err();
    assert!(error.to_string().contains("immutable: assignment"));
}

#[test]
fn final_closes_the_run() {
    let dir = tempfile::tempdir().unwrap();
    let journal = Journal::create(dir.path()).unwrap();
    journal.conn().execute_batch(SEED).unwrap();
    journal.conn().execute("INSERT INTO final VALUES ('r', 1, 'a-01', 'passed', '{}')", []).unwrap();
    let error = journal
        .conn()
        .execute(
            "INSERT INTO assignment VALUES ('a-02', 2, 1, 'reviewer', 'review', 'p', '[]', NULL, NULL, '[]', '[]', NULL)",
            [],
        )
        .unwrap_err();
    assert!(error.to_string().contains("run is finalized"));
}

#[test]
fn failed_write_rolls_back() {
    let dir = tempfile::tempdir().unwrap();
    let mut journal = Journal::create(dir.path()).unwrap();
    let result: foxmorrow_core::Result<()> = journal.write(|tx| {
        tx.execute("INSERT INTO meta (key, value) VALUES ('probe', 'x')", [])?;
        Err(Error::Invalid("stop".into()))
    });
    assert!(result.is_err());
    let count: i64 = journal
        .conn()
        .query_row("SELECT COUNT(*) FROM meta WHERE key = 'probe'", [], |row| row.get(0))
        .unwrap();
    assert_eq!(count, 0);
}
```

- [ ] **Шаг 2.** Запустить: `cargo test -p foxmorrow-core --test journal`. Ожидается ошибка компиляции: модуля `journal` нет.

- [ ] **Шаг 3.** `crates/foxmorrow-core/src/schema.rs`:

```rust
pub const CONTRACT_VERSION: &str = "1";

pub const DDL: &str = r#"
CREATE TABLE meta (key TEXT PRIMARY KEY, value TEXT NOT NULL);

CREATE TABLE run (
    id TEXT PRIMARY KEY,
    created_ms INTEGER NOT NULL,
    project_root TEXT NOT NULL,
    criteria_path TEXT NOT NULL,
    criteria_heading TEXT NOT NULL,
    criteria_sha256 TEXT NOT NULL,
    criteria_json TEXT NOT NULL,
    allowed_paths_json TEXT NOT NULL,
    baseline_json TEXT NOT NULL,
    root_session_id TEXT,
    root_model TEXT,
    family_rule INTEGER NOT NULL
);

CREATE TABLE assignment (
    id TEXT PRIMARY KEY,
    seq INTEGER NOT NULL UNIQUE,
    created_ms INTEGER NOT NULL,
    role TEXT NOT NULL,
    kind TEXT NOT NULL CHECK (kind IN ('author', 'review', 'qa', 'root')),
    profile TEXT,
    expected_models_json TEXT NOT NULL,
    brief_sha256 TEXT,
    role_sha256 TEXT,
    allowed_paths_json TEXT NOT NULL,
    depends_on_json TEXT NOT NULL,
    previous_id TEXT REFERENCES assignment (id)
);

CREATE TABLE receipt (
    id TEXT PRIMARY KEY,
    seq INTEGER NOT NULL UNIQUE,
    assignment_id TEXT NOT NULL REFERENCES assignment (id),
    cwd TEXT NOT NULL,
    argv_json TEXT NOT NULL,
    started_ms INTEGER NOT NULL,
    finished_ms INTEGER,
    exit_code INTEGER,
    output_sha256 TEXT,
    output_tail TEXT
);

CREATE TABLE report (
    assignment_id TEXT PRIMARY KEY REFERENCES assignment (id),
    created_ms INTEGER NOT NULL,
    status TEXT NOT NULL CHECK (status IN ('completed', 'interrupted')),
    outcome TEXT NOT NULL CHECK (outcome IN ('passed', 'failed', 'unverified')),
    body_json TEXT NOT NULL,
    body_sha256 TEXT NOT NULL
);

CREATE TABLE verification (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    assignment_id TEXT NOT NULL REFERENCES report (assignment_id),
    created_ms INTEGER NOT NULL,
    state TEXT NOT NULL CHECK (state IN ('accepted', 'rejected', 'unverified')),
    claims_json TEXT NOT NULL,
    observation_json TEXT
);

CREATE UNIQUE INDEX verification_accepted_once ON verification (assignment_id) WHERE state = 'accepted';

CREATE TABLE final (
    run_id TEXT PRIMARY KEY REFERENCES run (id),
    created_ms INTEGER NOT NULL,
    report_id TEXT NOT NULL REFERENCES report (assignment_id),
    outcome TEXT NOT NULL,
    body_json TEXT NOT NULL
);

CREATE TRIGGER run_immutable BEFORE UPDATE ON run
WHEN NEW.id IS NOT OLD.id
  OR NEW.project_root IS NOT OLD.project_root
  OR NEW.criteria_json IS NOT OLD.criteria_json
  OR NEW.criteria_sha256 IS NOT OLD.criteria_sha256
  OR NEW.allowed_paths_json IS NOT OLD.allowed_paths_json
  OR NEW.baseline_json IS NOT OLD.baseline_json
  OR NEW.family_rule IS NOT OLD.family_rule
  OR (OLD.root_session_id IS NOT NULL AND NEW.root_session_id IS NOT OLD.root_session_id)
  OR (OLD.root_model IS NOT NULL AND NEW.root_model IS NOT OLD.root_model)
BEGIN SELECT RAISE(ABORT, 'immutable: run'); END;
CREATE TRIGGER run_no_delete BEFORE DELETE ON run BEGIN SELECT RAISE(ABORT, 'immutable: run'); END;

CREATE TRIGGER assignment_no_update BEFORE UPDATE ON assignment BEGIN SELECT RAISE(ABORT, 'immutable: assignment'); END;
CREATE TRIGGER assignment_no_delete BEFORE DELETE ON assignment BEGIN SELECT RAISE(ABORT, 'immutable: assignment'); END;

CREATE TRIGGER receipt_finish_once BEFORE UPDATE ON receipt
WHEN OLD.finished_ms IS NOT NULL
  OR NEW.id IS NOT OLD.id
  OR NEW.assignment_id IS NOT OLD.assignment_id
  OR NEW.cwd IS NOT OLD.cwd
  OR NEW.argv_json IS NOT OLD.argv_json
  OR NEW.started_ms IS NOT OLD.started_ms
BEGIN SELECT RAISE(ABORT, 'immutable: receipt'); END;
CREATE TRIGGER receipt_no_delete BEFORE DELETE ON receipt BEGIN SELECT RAISE(ABORT, 'immutable: receipt'); END;

CREATE TRIGGER report_no_update BEFORE UPDATE ON report BEGIN SELECT RAISE(ABORT, 'immutable: report'); END;
CREATE TRIGGER report_no_delete BEFORE DELETE ON report BEGIN SELECT RAISE(ABORT, 'immutable: report'); END;

CREATE TRIGGER verification_no_update BEFORE UPDATE ON verification BEGIN SELECT RAISE(ABORT, 'immutable: verification'); END;
CREATE TRIGGER verification_no_delete BEFORE DELETE ON verification BEGIN SELECT RAISE(ABORT, 'immutable: verification'); END;

CREATE TRIGGER final_no_update BEFORE UPDATE ON final BEGIN SELECT RAISE(ABORT, 'immutable: final'); END;
CREATE TRIGGER final_no_delete BEFORE DELETE ON final BEGIN SELECT RAISE(ABORT, 'immutable: final'); END;

CREATE TRIGGER assignment_after_final BEFORE INSERT ON assignment WHEN EXISTS (SELECT 1 FROM final)
BEGIN SELECT RAISE(ABORT, 'run is finalized'); END;
CREATE TRIGGER receipt_after_final BEFORE INSERT ON receipt WHEN EXISTS (SELECT 1 FROM final)
BEGIN SELECT RAISE(ABORT, 'run is finalized'); END;
CREATE TRIGGER report_after_final BEFORE INSERT ON report WHEN EXISTS (SELECT 1 FROM final)
BEGIN SELECT RAISE(ABORT, 'run is finalized'); END;
CREATE TRIGGER verification_after_final BEFORE INSERT ON verification WHEN EXISTS (SELECT 1 FROM final)
BEGIN SELECT RAISE(ABORT, 'run is finalized'); END;
"#;
```

- [ ] **Шаг 4.** `crates/foxmorrow-core/src/journal.rs`:

```rust
use std::path::{Path, PathBuf};
use std::time::Duration;

use rusqlite::{Connection, OptionalExtension, Transaction, TransactionBehavior, params};

use crate::error::{Error, Result, invalid};
use crate::schema::{CONTRACT_VERSION, DDL};

pub const JOURNAL_FILE: &str = "journal.sqlite";

pub struct Journal {
    conn: Connection,
    dir: PathBuf,
}

impl Journal {
    pub fn create(dir: &Path) -> Result<Self> {
        let path = dir.join(JOURNAL_FILE);
        if path.exists() {
            return Err(Error::Conflict(format!("journal already exists at {}", path.display())));
        }
        std::fs::create_dir_all(dir)?;
        let conn = connect(&path)?;
        conn.execute_batch(DDL)?;
        conn.execute(
            "INSERT INTO meta (key, value) VALUES ('contract_version', ?1)",
            params![CONTRACT_VERSION],
        )?;
        Ok(Self { conn, dir: dir.to_path_buf() })
    }

    pub fn open(dir: &Path) -> Result<Self> {
        let path = dir.join(JOURNAL_FILE);
        if !path.exists() {
            return Err(Error::NotFound(format!("journal at {}", path.display())));
        }
        let conn = connect(&path)?;
        let version: Option<String> = conn
            .query_row("SELECT value FROM meta WHERE key = 'contract_version'", [], |row| row.get(0))
            .optional()?;
        if version.as_deref() != Some(CONTRACT_VERSION) {
            return invalid(format!(
                "unsupported journal contract {version:?}; this foxmorrow reads contract {CONTRACT_VERSION}"
            ));
        }
        Ok(Self { conn, dir: dir.to_path_buf() })
    }

    pub fn dir(&self) -> &Path {
        &self.dir
    }

    pub fn conn(&self) -> &Connection {
        &self.conn
    }

    pub fn write<T>(&mut self, apply: impl FnOnce(&Transaction<'_>) -> Result<T>) -> Result<T> {
        let tx = self.conn.transaction_with_behavior(TransactionBehavior::Immediate)?;
        let value = apply(&tx)?;
        tx.commit()?;
        Ok(value)
    }
}

fn connect(path: &Path) -> Result<Connection> {
    let conn = Connection::open(path)?;
    conn.busy_timeout(Duration::from_secs(5))?;
    conn.pragma_update(None, "foreign_keys", true)?;
    Ok(conn)
}
```

- [ ] **Шаг 5.** `crates/foxmorrow-core/src/state.rs`:

```rust
use rusqlite::{Connection, OptionalExtension};

use crate::error::Result;
use crate::model::{Outcome, Status, VerificationState};

pub fn report_state(conn: &Connection, assignment_id: &str) -> Result<Option<(Status, Outcome)>> {
    let raw: Option<(String, String)> = conn
        .query_row(
            "SELECT status, outcome FROM report WHERE assignment_id = ?1",
            [assignment_id],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .optional()?;
    match raw {
        Some((status, outcome)) => Ok(Some((Status::parse(&status)?, Outcome::parse(&outcome)?))),
        None => Ok(None),
    }
}

pub fn is_accepted(conn: &Connection, assignment_id: &str) -> Result<bool> {
    Ok(conn.query_row(
        "SELECT EXISTS (SELECT 1 FROM verification WHERE assignment_id = ?1 AND state = 'accepted')",
        [assignment_id],
        |row| row.get(0),
    )?)
}

pub fn latest_verification(conn: &Connection, assignment_id: &str) -> Result<Option<VerificationState>> {
    let raw: Option<String> = conn
        .query_row(
            "SELECT state FROM verification WHERE assignment_id = ?1 ORDER BY id DESC LIMIT 1",
            [assignment_id],
            |row| row.get(0),
        )
        .optional()?;
    raw.map(|state| VerificationState::parse(&state)).transpose()
}

pub fn is_finalized(conn: &Connection) -> Result<bool> {
    Ok(conn.query_row("SELECT EXISTS (SELECT 1 FROM final)", [], |row| row.get(0))?)
}
```

- [ ] **Шаг 6.** `lib.rs`:

```rust
pub mod criteria;
pub mod error;
pub mod journal;
pub mod model;
pub mod schema;
pub mod state;
pub mod util;

pub use error::{Error, Result};
```

- [ ] **Шаг 7.** Запустить: `cargo test -p foxmorrow-core --test journal`. Ожидается: 5 тестов пройдены.

- [ ] **Шаг 8.** Коммит:

```sh
git add crates/foxmorrow-core
git commit -m "Add the journal schema with immutability triggers"
```

## Задача 5. Базовое состояние git и run

**Файлы:** создать `baseline.rs`, `run.rs`, тесты `tests/baseline.rs`, `tests/run.rs`; изменить `lib.rs`.

- [ ] **Шаг 1.** Тест `crates/foxmorrow-core/tests/baseline.rs`:

```rust
use std::path::Path;
use std::process::Command;

use foxmorrow_core::baseline;

fn git(dir: &Path, args: &[&str]) {
    let status = Command::new("git")
        .arg("-C")
        .arg(dir)
        .args(["-c", "user.name=test", "-c", "user.email=test@example.com"])
        .args(args)
        .status()
        .unwrap();
    assert!(status.success());
}

#[test]
fn records_head_and_dirty_files_without_foxmorrow_data() {
    let dir = tempfile::tempdir().unwrap();
    git(dir.path(), &["init", "-q"]);
    std::fs::write(dir.path().join("a.txt"), "one").unwrap();
    git(dir.path(), &["add", "a.txt"]);
    git(dir.path(), &["commit", "-qm", "init"]);
    std::fs::write(dir.path().join("a.txt"), "two").unwrap();
    std::fs::write(dir.path().join("b.txt"), "new").unwrap();
    std::fs::create_dir_all(dir.path().join(".foxmorrow")).unwrap();
    std::fs::write(dir.path().join(".foxmorrow/current"), "r-1").unwrap();

    let state = baseline::collect(dir.path()).unwrap();

    assert_eq!(state.head.as_deref().map(str::len), Some(40));
    assert!(state.dirty.iter().any(|line| line.ends_with("a.txt") && line.starts_with(" M")));
    assert!(state.dirty.iter().any(|line| line == "?? b.txt"));
    assert!(!state.dirty.iter().any(|line| line.contains(".foxmorrow")));
    assert!(state.remote_refs.is_empty());
}

#[test]
fn empty_repository_has_no_head() {
    let dir = tempfile::tempdir().unwrap();
    git(dir.path(), &["init", "-q"]);
    assert_eq!(baseline::collect(dir.path()).unwrap().head, None);
}

#[test]
fn directory_outside_git_is_rejected() {
    let dir = tempfile::tempdir().unwrap();
    assert!(baseline::collect(dir.path()).is_err());
}
```

- [ ] **Шаг 2.** Тест `crates/foxmorrow-core/tests/run.rs`:

```rust
use foxmorrow_core::Error;
use foxmorrow_core::run::{self, NewRun};

fn new_run(root: &std::path::Path, paths: Vec<String>) -> NewRun {
    NewRun {
        project_root: root.to_path_buf(),
        criteria_path: "task.md".into(),
        criteria_heading: "Критерии приёмки".into(),
        criteria_markdown: "## Критерии приёмки\n\n- Первый\n- Второй\n".into(),
        allowed_paths: paths,
        baselines: Vec::new(),
        root_session_id: None,
        family_rule: true,
    }
}

#[test]
fn creates_a_run_and_points_current_at_it() {
    let dir = tempfile::tempdir().unwrap();
    let (created, _) = run::create_run(new_run(dir.path(), vec!["src/".into()])).unwrap();

    assert!(dir.path().join(".foxmorrow/runs").join(&created.id).join("journal.sqlite").is_file());
    assert_eq!(run::current_run_id(dir.path()).unwrap(), created.id);
    assert_eq!(created.criteria.len(), 2);
    assert_eq!(created.criteria_sha256.len(), 64);

    let (opened, _) = run::open_run(dir.path(), None).unwrap();
    assert_eq!(opened, created);
}

#[test]
fn requires_allowed_paths_and_an_existing_run() {
    let dir = tempfile::tempdir().unwrap();
    assert!(matches!(run::create_run(new_run(dir.path(), Vec::new())), Err(Error::Invalid(_))));
    assert!(matches!(run::open_run(dir.path(), None), Err(Error::NotFound(_))));
}

#[test]
fn root_identity_is_set_once_and_criteria_never_change() {
    let dir = tempfile::tempdir().unwrap();
    let (_, journal) = run::create_run(new_run(dir.path(), vec!["src/".into()])).unwrap();
    journal.conn().execute("UPDATE run SET root_model = 'opus'", []).unwrap();
    assert!(journal.conn().execute("UPDATE run SET root_model = 'gpt'", []).is_err());
    assert!(journal.conn().execute("UPDATE run SET criteria_json = '[]'", []).is_err());
}
```

- [ ] **Шаг 3.** Запустить: `cargo test -p foxmorrow-core --test baseline --test run`. Ожидается ошибка компиляции.

- [ ] **Шаг 4.** `crates/foxmorrow-core/src/baseline.rs`:

```rust
use std::path::{Path, PathBuf};
use std::process::Command;

use serde::{Deserialize, Serialize};

use crate::error::{Result, invalid};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RepoBaseline {
    pub root: String,
    pub head: Option<String>,
    pub remote_refs: Vec<String>,
    pub dirty: Vec<String>,
}

pub fn toplevel(dir: &Path) -> Result<PathBuf> {
    git(dir, &["rev-parse", "--show-toplevel"]).map(PathBuf::from)
}

pub fn collect(dir: &Path) -> Result<RepoBaseline> {
    let root = toplevel(dir)?;
    let head = git(&root, &["rev-parse", "--verify", "--quiet", "HEAD"])
        .ok()
        .filter(|head| !head.is_empty());
    let remote_refs = lines(&git(&root, &["for-each-ref", "--format=%(refname) %(objectname)", "refs/remotes"])?);
    let dirty = lines(&git(&root, &["status", "--porcelain=v1", "--untracked-files=all"])?)
        .into_iter()
        .filter(|line| !line.get(3..).is_some_and(|path| path.starts_with(".foxmorrow/")))
        .collect();
    Ok(RepoBaseline { root: root.display().to_string(), head, remote_refs, dirty })
}

fn git(dir: &Path, args: &[&str]) -> Result<String> {
    let output = Command::new("git").arg("-C").arg(dir).args(args).output()?;
    if !output.status.success() {
        return invalid(format!(
            "git {} failed in {}: {}",
            args.join(" "),
            dir.display(),
            String::from_utf8_lossy(&output.stderr).trim()
        ));
    }
    Ok(String::from_utf8_lossy(&output.stdout).trim_end().to_string())
}

fn lines(text: &str) -> Vec<String> {
    text.lines().filter(|line| !line.is_empty()).map(str::to_string).collect()
}
```

- [ ] **Шаг 5.** `crates/foxmorrow-core/src/run.rs`:

```rust
use std::path::{Path, PathBuf};

use rusqlite::{Connection, params};
use serde::Serialize;

use crate::baseline::RepoBaseline;
use crate::criteria::{self, Criterion};
use crate::error::{Error, Result, invalid};
use crate::journal::Journal;
use crate::util::{new_run_id, now_ms, sha256_hex};

#[derive(Debug, Clone)]
pub struct NewRun {
    pub project_root: PathBuf,
    pub criteria_path: String,
    pub criteria_heading: String,
    pub criteria_markdown: String,
    pub allowed_paths: Vec<String>,
    pub baselines: Vec<RepoBaseline>,
    pub root_session_id: Option<String>,
    pub family_rule: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Run {
    pub id: String,
    pub created_ms: i64,
    pub project_root: String,
    pub criteria_path: String,
    pub criteria_heading: String,
    pub criteria_sha256: String,
    pub criteria: Vec<Criterion>,
    pub allowed_paths: Vec<String>,
    pub family_rule: bool,
    pub root_session_id: Option<String>,
    pub root_model: Option<String>,
}

pub fn foxmorrow_dir(project_root: &Path) -> PathBuf {
    project_root.join(".foxmorrow")
}

pub fn runs_dir(project_root: &Path) -> PathBuf {
    foxmorrow_dir(project_root).join("runs")
}

pub fn create_run(new: NewRun) -> Result<(Run, Journal)> {
    if new.allowed_paths.is_empty() {
        return invalid("at least one allowed path is required");
    }
    let (section, criteria) = criteria::parse_section(&new.criteria_markdown, &new.criteria_heading)?;
    let id = new_run_id();
    let mut journal = Journal::create(&runs_dir(&new.project_root).join(&id))?;
    let run = Run {
        id: id.clone(),
        created_ms: now_ms(),
        project_root: new.project_root.display().to_string(),
        criteria_path: new.criteria_path,
        criteria_heading: new.criteria_heading,
        criteria_sha256: sha256_hex(section.as_bytes()),
        criteria,
        allowed_paths: new.allowed_paths,
        family_rule: new.family_rule,
        root_session_id: new.root_session_id,
        root_model: None,
    };
    let baseline_json = serde_json::to_string(&new.baselines)?;
    journal.write(|tx| {
        tx.execute(
            "INSERT INTO run (id, created_ms, project_root, criteria_path, criteria_heading, criteria_sha256, criteria_json, allowed_paths_json, baseline_json, root_session_id, root_model, family_rule)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)",
            params![
                run.id,
                run.created_ms,
                run.project_root,
                run.criteria_path,
                run.criteria_heading,
                run.criteria_sha256,
                serde_json::to_string(&run.criteria)?,
                serde_json::to_string(&run.allowed_paths)?,
                baseline_json,
                run.root_session_id,
                run.root_model,
                run.family_rule
            ],
        )?;
        Ok(())
    })?;
    std::fs::write(foxmorrow_dir(&new.project_root).join("current"), format!("{id}\n"))?;
    Ok((run, journal))
}

pub fn current_run_id(project_root: &Path) -> Result<String> {
    let path = foxmorrow_dir(project_root).join("current");
    let missing = || Error::NotFound("no current run; start one with foxmorrow init".into());
    let text = std::fs::read_to_string(&path).map_err(|_| missing())?;
    let id = text.trim();
    if id.is_empty() {
        return Err(missing());
    }
    Ok(id.to_string())
}

pub fn open_run(project_root: &Path, run_id: Option<&str>) -> Result<(Run, Journal)> {
    let id = match run_id {
        Some(id) => id.to_string(),
        None => current_run_id(project_root)?,
    };
    let journal = Journal::open(&runs_dir(project_root).join(&id))?;
    let run = load_run(journal.conn())?;
    Ok((run, journal))
}

pub fn load_run(conn: &Connection) -> Result<Run> {
    let (run, criteria_json, paths_json) = conn.query_row(
        "SELECT id, created_ms, project_root, criteria_path, criteria_heading, criteria_sha256, criteria_json, allowed_paths_json, family_rule, root_session_id, root_model FROM run",
        [],
        |row| {
            Ok((
                Run {
                    id: row.get(0)?,
                    created_ms: row.get(1)?,
                    project_root: row.get(2)?,
                    criteria_path: row.get(3)?,
                    criteria_heading: row.get(4)?,
                    criteria_sha256: row.get(5)?,
                    criteria: Vec::new(),
                    allowed_paths: Vec::new(),
                    family_rule: row.get(8)?,
                    root_session_id: row.get(9)?,
                    root_model: row.get(10)?,
                },
                row.get::<_, String>(6)?,
                row.get::<_, String>(7)?,
            ))
        },
    )?;
    Ok(Run {
        criteria: serde_json::from_str(&criteria_json)?,
        allowed_paths: serde_json::from_str(&paths_json)?,
        ..run
    })
}
```

- [ ] **Шаг 6.** Добавить в `lib.rs` `pub mod baseline;` и `pub mod run;` в алфавитном порядке.

- [ ] **Шаг 7.** Запустить: `cargo test -p foxmorrow-core --test baseline --test run`. Ожидается: 6 тестов пройдены.

- [ ] **Шаг 8.** Коммит:

```sh
git add crates/foxmorrow-core
git commit -m "Create runs with criteria and git baseline"
```

## Задача 6. Назначения

Положительные сценарии порядка (`review` после принятого автора, `qa` после принятого `review`) проверяются в задаче 9, когда появится `verify`.

**Файлы:** создать `assignment.rs`, `tests/common/mod.rs`, `tests/assignment.rs`; изменить `lib.rs`.

- [ ] **Шаг 1.** Помощники тестов `crates/foxmorrow-core/tests/common/mod.rs`. Часть функций используют только тесты задач 8–11:

```rust
#![allow(dead_code)]

use std::collections::BTreeMap;
use std::path::Path;

use foxmorrow_core::assignment::{self, Assignment, NewAssignment};
use foxmorrow_core::claims::{self, ChainObservation, ModelFamilies, ToolCall};
use foxmorrow_core::journal::Journal;
use foxmorrow_core::model::{Kind, VerificationState};
use foxmorrow_core::report;
use foxmorrow_core::run::{self, NewRun, Run};

pub const TASK: &str = "# Задача\n\n## Критерии приёмки\n\n- Первый критерий\n- Второй критерий\n";

pub fn new_run(dir: &Path) -> (Run, Journal) {
    new_run_with(dir, true)
}

pub fn new_run_with(dir: &Path, family_rule: bool) -> (Run, Journal) {
    run::create_run(NewRun {
        project_root: dir.to_path_buf(),
        criteria_path: "task.md".into(),
        criteria_heading: "Критерии приёмки".into(),
        criteria_markdown: TASK.into(),
        allowed_paths: vec!["src/".into(), "tests/".into()],
        baselines: Vec::new(),
        root_session_id: None,
        family_rule,
    })
    .unwrap()
}

pub fn spec(role: &str, kind: Kind, model: &str, depends_on: &[&str]) -> NewAssignment {
    NewAssignment {
        role: role.into(),
        kind,
        profile: Some(format!("foxmorrow:{role}")),
        expected_models: vec![model.into()],
        brief_sha256: Some(format!("brief-{role}")),
        role_sha256: Some(format!("role-{role}")),
        allowed_paths: vec!["src/".into()],
        depends_on: depends_on.iter().map(|id| id.to_string()).collect(),
        previous_id: None,
    }
}

pub fn root_spec() -> NewAssignment {
    NewAssignment {
        role: "root".into(),
        kind: Kind::Root,
        profile: None,
        expected_models: Vec::new(),
        brief_sha256: None,
        role_sha256: None,
        allowed_paths: vec!["src/".into()],
        depends_on: Vec::new(),
        previous_id: None,
    }
}

pub fn families() -> ModelFamilies {
    BTreeMap::from([
        ("swe-2-high".to_string(), "swe".to_string()),
        ("swe-2-low".to_string(), "swe".to_string()),
        ("gpt-6".to_string(), "gpt".to_string()),
        ("opus-5".to_string(), "claude".to_string()),
    ])
}

pub fn report_call(id: &str) -> ToolCall {
    ToolCall {
        id: format!("call-report-{id}"),
        name: "exec".into(),
        command: Some(format!("foxmorrow report -a {id} <<'JSON'")),
        exit_code: Some(0),
    }
}

pub fn observation(assignment: &Assignment, agent: &str, model: &str) -> ChainObservation {
    ChainObservation {
        source: "test".into(),
        agent_id: agent.into(),
        profile: assignment.profile.clone().unwrap_or_default(),
        task_text: format!(
            "foxmorrow assignment {} brief-sha256 {}",
            assignment.id,
            assignment.brief_sha256.clone().unwrap_or_default()
        ),
        system_texts: vec![format!(
            "foxmorrow-role-sha256: {}",
            assignment.role_sha256.clone().unwrap_or_default()
        )],
        models: vec![model.into()],
        tool_calls: vec![report_call(&assignment.id)],
    }
}

pub fn passed_report(criteria: &[&str]) -> String {
    let criteria: serde_json::Map<String, serde_json::Value> = criteria
        .iter()
        .map(|id| (id.to_string(), serde_json::json!({"status": "passed", "reason": "checked"})))
        .collect();
    serde_json::json!({"summary": "done", "status": "completed", "outcome": "passed", "criteria": criteria})
        .to_string()
}

pub fn accept(
    journal: &mut Journal,
    run: &Run,
    new: NewAssignment,
    agent: &str,
    model: &str,
    report_json: &str,
) -> Assignment {
    let created = assignment::create(journal, run, new).unwrap();
    report::submit(journal, run, &created.id, report_json).unwrap();
    let verification =
        claims::verify(journal, run, &created.id, Some(observation(&created, agent, model)), &families()).unwrap();
    assert_eq!(verification.state, VerificationState::Accepted, "{:?}", verification.claims);
    created
}

pub fn accepted_chain(journal: &mut Journal, run: &Run) -> (Assignment, Assignment, Assignment) {
    let author = accept(
        journal,
        run,
        spec("backend-worker", Kind::Author, "swe-2-high", &[]),
        "agent-author",
        "swe-2-high",
        &passed_report(&[]),
    );
    let review = accept(
        journal,
        run,
        spec("reviewer", Kind::Review, "gpt-6", &[&author.id]),
        "agent-review",
        "gpt-6",
        &passed_report(&[]),
    );
    let qa = accept(
        journal,
        run,
        spec("qa-verifier", Kind::Qa, "opus-5", &[&review.id]),
        "agent-qa",
        "opus-5",
        &passed_report(&["ac-1", "ac-2"]),
    );
    (author, review, qa)
}
```

Модули `claims` и `report` появятся в задачах 8 и 9. До них тесты, использующие `common`, не компилируются, поэтому тесты задачи 6 пока не подключают `common`.

- [ ] **Шаг 2.** Тест `crates/foxmorrow-core/tests/assignment.rs`:

```rust
use foxmorrow_core::Error;
use foxmorrow_core::assignment::{self, NewAssignment, within};
use foxmorrow_core::model::Kind;
use foxmorrow_core::run::{self, NewRun};

fn setup(dir: &std::path::Path) -> (foxmorrow_core::run::Run, foxmorrow_core::journal::Journal) {
    run::create_run(NewRun {
        project_root: dir.to_path_buf(),
        criteria_path: "task.md".into(),
        criteria_heading: "Критерии приёмки".into(),
        criteria_markdown: "## Критерии приёмки\n\n- Первый\n".into(),
        allowed_paths: vec!["src/".into(), "tests/".into()],
        baselines: Vec::new(),
        root_session_id: None,
        family_rule: true,
    })
    .unwrap()
}

fn author(paths: &[&str]) -> NewAssignment {
    NewAssignment {
        role: "backend-worker".into(),
        kind: Kind::Author,
        profile: Some("foxmorrow:backend-worker".into()),
        expected_models: vec!["swe-2-high".into()],
        brief_sha256: Some("brief".into()),
        role_sha256: Some("role".into()),
        allowed_paths: paths.iter().map(|p| p.to_string()).collect(),
        depends_on: Vec::new(),
        previous_id: None,
    }
}

#[test]
fn numbers_assignments_and_reads_them_back() {
    let dir = tempfile::tempdir().unwrap();
    let (run, mut journal) = setup(dir.path());
    let first = assignment::create(&mut journal, &run, author(&["src/api"])).unwrap();
    let second = assignment::create(&mut journal, &run, author(&["./tests/"])).unwrap();
    assert_eq!((first.id.as_str(), second.id.as_str()), ("a-01", "a-02"));
    assert_eq!(assignment::get(journal.conn(), "a-01").unwrap(), first);
    assert_eq!(assignment::list(journal.conn()).unwrap(), vec![first, second]);
}

#[test]
fn keeps_paths_inside_the_run() {
    assert!(within("src/api/health.rs", &["src/".into()]));
    assert!(within("src", &["./src/".into()]));
    assert!(!within("srcx/file", &["src".into()]));
    assert!(!within("src/../secrets", &["src".into()]));
    assert!(within("anything", &[".".into()]));

    let dir = tempfile::tempdir().unwrap();
    let (run, mut journal) = setup(dir.path());
    assert!(matches!(assignment::create(&mut journal, &run, author(&["docs/"])), Err(Error::Invalid(_))));
    assert!(matches!(assignment::create(&mut journal, &run, author(&[])), Err(Error::Invalid(_))));
}

#[test]
fn delegated_assignments_need_profile_and_model() {
    let dir = tempfile::tempdir().unwrap();
    let (run, mut journal) = setup(dir.path());
    let mut missing_profile = author(&["src/"]);
    missing_profile.profile = None;
    assert!(matches!(assignment::create(&mut journal, &run, missing_profile), Err(Error::Invalid(_))));
    let mut missing_model = author(&["src/"]);
    missing_model.expected_models.clear();
    assert!(matches!(assignment::create(&mut journal, &run, missing_model), Err(Error::Invalid(_))));
    let mut root = author(&["src/"]);
    root.kind = Kind::Root;
    root.profile = None;
    root.expected_models.clear();
    assert!(assignment::create(&mut journal, &run, root).is_ok());
}

#[test]
fn enforces_dependency_links() {
    let dir = tempfile::tempdir().unwrap();
    let (run, mut journal) = setup(dir.path());
    let mut unknown = author(&["src/"]);
    unknown.depends_on = vec!["a-99".into()];
    assert!(matches!(assignment::create(&mut journal, &run, unknown), Err(Error::NotFound(_))));

    let written = assignment::create(&mut journal, &run, author(&["src/"])).unwrap();
    let mut review = author(&["src/"]);
    review.kind = Kind::Review;
    assert!(matches!(assignment::create(&mut journal, &run, review.clone()), Err(Error::Invalid(_))));
    review.depends_on = vec![written.id.clone()];
    let error = assignment::create(&mut journal, &run, review).unwrap_err();
    assert!(error.to_string().contains("has no report"), "{error}");

    let mut correction = author(&["src/"]);
    correction.previous_id = Some(written.id.clone());
    let error = assignment::create(&mut journal, &run, correction).unwrap_err();
    assert!(error.to_string().contains("has no report"), "{error}");
}
```

- [ ] **Шаг 3.** Запустить: `cargo test -p foxmorrow-core --test assignment`. Ожидается ошибка компиляции.

- [ ] **Шаг 4.** `crates/foxmorrow-core/src/assignment.rs`:

```rust
use rusqlite::{Connection, OptionalExtension, Row, params};
use serde::Serialize;

use crate::error::{Error, Result, invalid};
use crate::journal::Journal;
use crate::model::{Kind, Outcome, Status};
use crate::run::Run;
use crate::state;
use crate::util::now_ms;

#[derive(Debug, Clone)]
pub struct NewAssignment {
    pub role: String,
    pub kind: Kind,
    pub profile: Option<String>,
    pub expected_models: Vec<String>,
    pub brief_sha256: Option<String>,
    pub role_sha256: Option<String>,
    pub allowed_paths: Vec<String>,
    pub depends_on: Vec<String>,
    pub previous_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Assignment {
    pub id: String,
    pub seq: i64,
    pub created_ms: i64,
    pub role: String,
    pub kind: Kind,
    pub profile: Option<String>,
    pub expected_models: Vec<String>,
    pub brief_sha256: Option<String>,
    pub role_sha256: Option<String>,
    pub allowed_paths: Vec<String>,
    pub depends_on: Vec<String>,
    pub previous_id: Option<String>,
}

const COLUMNS: &str = "id, seq, created_ms, role, kind, profile, expected_models_json, brief_sha256, role_sha256, allowed_paths_json, depends_on_json, previous_id";

type RawAssignment = (Assignment, String, String, String, String);

pub fn create(journal: &mut Journal, run: &Run, new: NewAssignment) -> Result<Assignment> {
    validate_shape(run, &new)?;
    journal.write(|tx| {
        validate_links(tx, &new)?;
        let seq: i64 = tx.query_row("SELECT COALESCE(MAX(seq), 0) + 1 FROM assignment", [], |row| row.get(0))?;
        let created = Assignment {
            id: format!("a-{seq:02}"),
            seq,
            created_ms: now_ms(),
            role: new.role,
            kind: new.kind,
            profile: new.profile,
            expected_models: new.expected_models,
            brief_sha256: new.brief_sha256,
            role_sha256: new.role_sha256,
            allowed_paths: new.allowed_paths,
            depends_on: new.depends_on,
            previous_id: new.previous_id,
        };
        tx.execute(
            &format!("INSERT INTO assignment ({COLUMNS}) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)"),
            params![
                created.id,
                created.seq,
                created.created_ms,
                created.role,
                created.kind.as_str(),
                created.profile,
                serde_json::to_string(&created.expected_models)?,
                created.brief_sha256,
                created.role_sha256,
                serde_json::to_string(&created.allowed_paths)?,
                serde_json::to_string(&created.depends_on)?,
                created.previous_id
            ],
        )?;
        Ok(created)
    })
}

pub fn get(conn: &Connection, id: &str) -> Result<Assignment> {
    let raw = conn
        .query_row(&format!("SELECT {COLUMNS} FROM assignment WHERE id = ?1"), [id], read_row)
        .optional()?;
    into_assignment(raw.ok_or_else(|| Error::NotFound(format!("assignment {id}")))?)
}

pub fn list(conn: &Connection) -> Result<Vec<Assignment>> {
    let mut statement = conn.prepare(&format!("SELECT {COLUMNS} FROM assignment ORDER BY seq"))?;
    let rows = statement.query_map([], read_row)?;
    let mut assignments = Vec::new();
    for row in rows {
        assignments.push(into_assignment(row?)?);
    }
    Ok(assignments)
}

pub fn within(path: &str, roots: &[String]) -> bool {
    let path = normalize(path);
    if path.split('/').any(|part| part == "..") {
        return false;
    }
    roots
        .iter()
        .map(|root| normalize(root))
        .any(|root| root.is_empty() || path == root || path.starts_with(&format!("{root}/")))
}

fn normalize(path: &str) -> String {
    let trimmed = path.trim();
    let trimmed = trimmed.strip_prefix("./").unwrap_or(trimmed);
    if trimmed == "." {
        String::new()
    } else {
        trimmed.trim_end_matches('/').to_string()
    }
}

fn validate_shape(run: &Run, new: &NewAssignment) -> Result<()> {
    if new.role.trim().is_empty() {
        return invalid("role is required");
    }
    if new.allowed_paths.is_empty() {
        return invalid("at least one allowed path is required");
    }
    if let Some(outside) = new.allowed_paths.iter().find(|path| !within(path, &run.allowed_paths)) {
        return invalid(format!("path '{outside}' is outside the run allowed paths"));
    }
    if new.kind != Kind::Root {
        if new.profile.as_deref().is_none_or(|profile| profile.trim().is_empty()) {
            return invalid("profile is required");
        }
        if new.expected_models.is_empty() {
            return invalid("expected model is required");
        }
    }
    Ok(())
}

fn validate_links(conn: &Connection, new: &NewAssignment) -> Result<()> {
    if let Some(previous) = &new.previous_id {
        let corrected = get(conn, previous)?;
        if corrected.kind != new.kind {
            return invalid(format!(
                "correction of {previous} must keep kind {}, got {}",
                corrected.kind.as_str(),
                new.kind.as_str()
            ));
        }
        if state::report_state(conn, previous)?.is_none() {
            return invalid(format!("{previous} has no report to correct"));
        }
    }
    let mut dependencies = Vec::new();
    for id in &new.depends_on {
        dependencies.push(get(conn, id)?);
    }
    match new.kind {
        Kind::Root => {}
        Kind::Author => {
            for dependency in &dependencies {
                if !state::is_accepted(conn, &dependency.id)? {
                    return invalid(format!("dependency {} is not accepted", dependency.id));
                }
            }
        }
        Kind::Review => {
            if dependencies.is_empty() {
                return invalid("review requires author dependencies");
            }
            for dependency in &dependencies {
                if dependency.kind != Kind::Author {
                    return invalid(format!("review dependency {} is not an author assignment", dependency.id));
                }
                require_completed_and_accepted(conn, &dependency.id)?;
            }
        }
        Kind::Qa => {
            let mut passed_review = false;
            for dependency in &dependencies {
                match dependency.kind {
                    Kind::Review => {
                        let outcome = require_completed_and_accepted(conn, &dependency.id)?;
                        if outcome != Outcome::Passed {
                            return invalid(format!("review {} did not pass", dependency.id));
                        }
                        passed_review = true;
                    }
                    Kind::Author => {
                        require_completed_and_accepted(conn, &dependency.id)?;
                    }
                    _ => return invalid(format!("qa dependency {} must be an author or review", dependency.id)),
                }
            }
            if !passed_review {
                return invalid("qa requires a passed review dependency");
            }
        }
    }
    Ok(())
}

fn require_completed_and_accepted(conn: &Connection, id: &str) -> Result<Outcome> {
    let Some((status, outcome)) = state::report_state(conn, id)? else {
        return invalid(format!("dependency {id} has no report"));
    };
    if status != Status::Completed {
        return invalid(format!("dependency {id} is not completed"));
    }
    if !state::is_accepted(conn, id)? {
        return invalid(format!("dependency {id} is not accepted"));
    }
    Ok(outcome)
}

fn read_row(row: &Row<'_>) -> rusqlite::Result<RawAssignment> {
    Ok((
        Assignment {
            id: row.get(0)?,
            seq: row.get(1)?,
            created_ms: row.get(2)?,
            role: row.get(3)?,
            kind: Kind::Root,
            profile: row.get(5)?,
            expected_models: Vec::new(),
            brief_sha256: row.get(7)?,
            role_sha256: row.get(8)?,
            allowed_paths: Vec::new(),
            depends_on: Vec::new(),
            previous_id: row.get(11)?,
        },
        row.get(4)?,
        row.get(6)?,
        row.get(9)?,
        row.get(10)?,
    ))
}

fn into_assignment((assignment, kind, models, paths, depends): RawAssignment) -> Result<Assignment> {
    Ok(Assignment {
        kind: Kind::parse(&kind)?,
        expected_models: serde_json::from_str(&models)?,
        allowed_paths: serde_json::from_str(&paths)?,
        depends_on: serde_json::from_str(&depends)?,
        ..assignment
    })
}
```

Тест на исправление в шаге 2 ожидает фразу `has no report`, а функция возвращает `{previous} has no report to correct`. Фраза входит в это сообщение, тест корректен.

- [ ] **Шаг 5.** Добавить `pub mod assignment;` в `lib.rs`.

- [ ] **Шаг 6.** Запустить: `cargo test -p foxmorrow-core --test assignment`. Ожидается: 4 теста пройдены.

- [ ] **Шаг 7.** Коммит:

```sh
git add crates/foxmorrow-core/src crates/foxmorrow-core/tests/assignment.rs
git commit -m "Add assignments with path and ordering rules"
```

Файл `tests/common/mod.rs` добавляется в коммит задачи 9, когда он начнёт компилироваться.

## Задача 7. Квитанции `check`

**Файлы:** создать `receipt.rs`, тест `tests/receipt.rs`; изменить `lib.rs`.

- [ ] **Шаг 1.** Тест `crates/foxmorrow-core/tests/receipt.rs`:

```rust
#![cfg(unix)]

use foxmorrow_core::assignment::{self, NewAssignment};
use foxmorrow_core::model::Kind;
use foxmorrow_core::receipt::{self, OUTPUT_TAIL_BYTES};
use foxmorrow_core::run::{self, NewRun};

fn setup(dir: &std::path::Path) -> (foxmorrow_core::run::Run, foxmorrow_core::journal::Journal) {
    let (run, mut journal) = run::create_run(NewRun {
        project_root: dir.to_path_buf(),
        criteria_path: "task.md".into(),
        criteria_heading: "Критерии приёмки".into(),
        criteria_markdown: "## Критерии приёмки\n\n- Первый\n".into(),
        allowed_paths: vec!["src/".into()],
        baselines: Vec::new(),
        root_session_id: None,
        family_rule: true,
    })
    .unwrap();
    assignment::create(
        &mut journal,
        &run,
        NewAssignment {
            role: "backend-worker".into(),
            kind: Kind::Author,
            profile: Some("foxmorrow:backend-worker".into()),
            expected_models: vec!["swe-2-high".into()],
            brief_sha256: None,
            role_sha256: None,
            allowed_paths: vec!["src/".into()],
            depends_on: Vec::new(),
            previous_id: None,
        },
    )
    .unwrap();
    (run, journal)
}

fn argv(parts: &[&str]) -> Vec<String> {
    parts.iter().map(|part| part.to_string()).collect()
}

#[test]
fn records_success_and_failure() {
    let dir = tempfile::tempdir().unwrap();
    let (_, mut journal) = setup(dir.path());
    let ok = receipt::run_check(&mut journal, "a-01", dir.path(), &argv(&["sh", "-c", "echo ok"])).unwrap();
    assert_eq!(ok.id, "chk-001");
    assert_eq!(ok.exit_code, Some(0));
    assert_eq!(ok.output_tail.as_deref(), Some("ok\n"));
    assert!(ok.finished_ms.is_some());

    let failed = receipt::run_check(&mut journal, "a-01", dir.path(), &argv(&["sh", "-c", "echo bad >&2; exit 3"])).unwrap();
    assert_eq!(failed.id, "chk-002");
    assert_eq!(failed.exit_code, Some(3));
    assert_eq!(failed.output_tail.as_deref(), Some("bad\n"));
    assert_eq!(receipt::list_for(journal.conn(), "a-01").unwrap(), vec![ok, failed]);
}

#[test]
fn records_a_command_that_cannot_start() {
    let dir = tempfile::tempdir().unwrap();
    let (_, mut journal) = setup(dir.path());
    let missing = receipt::run_check(&mut journal, "a-01", dir.path(), &argv(&["foxmorrow-no-such-binary"])).unwrap();
    assert_eq!(missing.exit_code, None);
    assert!(missing.output_tail.unwrap().contains("failed to start"));
}

#[test]
fn keeps_only_the_output_tail() {
    let dir = tempfile::tempdir().unwrap();
    let (_, mut journal) = setup(dir.path());
    let script = format!("head -c {} /dev/zero | tr '\\0' 'x'", OUTPUT_TAIL_BYTES + 100);
    let long = receipt::run_check(&mut journal, "a-01", dir.path(), &argv(&["sh", "-c", &script])).unwrap();
    assert_eq!(long.output_tail.unwrap().len(), OUTPUT_TAIL_BYTES);
    assert_eq!(long.output_sha256.unwrap().len(), 64);
}

#[test]
fn finished_receipt_cannot_change() {
    let dir = tempfile::tempdir().unwrap();
    let (_, mut journal) = setup(dir.path());
    receipt::run_check(&mut journal, "a-01", dir.path(), &argv(&["true"])).unwrap();
    let error = journal.conn().execute("UPDATE receipt SET exit_code = 1", []).unwrap_err();
    assert!(error.to_string().contains("immutable: receipt"));
}
```

- [ ] **Шаг 2.** Запустить: `cargo test -p foxmorrow-core --test receipt`. Ожидается ошибка компиляции.

- [ ] **Шаг 3.** `crates/foxmorrow-core/src/receipt.rs`:

```rust
use std::path::Path;
use std::process::Command;

use rusqlite::{Connection, OptionalExtension, Row, params};
use serde::Serialize;

use crate::assignment;
use crate::error::{Error, Result, invalid};
use crate::journal::Journal;
use crate::state;
use crate::util::{now_ms, sha256_hex};

pub const OUTPUT_TAIL_BYTES: usize = 16 * 1024;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Receipt {
    pub id: String,
    pub assignment_id: String,
    pub cwd: String,
    pub argv: Vec<String>,
    pub started_ms: i64,
    pub finished_ms: Option<i64>,
    pub exit_code: Option<i32>,
    pub output_sha256: Option<String>,
    pub output_tail: Option<String>,
}

const COLUMNS: &str = "id, assignment_id, cwd, argv_json, started_ms, finished_ms, exit_code, output_sha256, output_tail";

pub fn run_check(journal: &mut Journal, assignment_id: &str, cwd: &Path, argv: &[String]) -> Result<Receipt> {
    let Some(program) = argv.first() else {
        return invalid("command is required");
    };
    let cwd_text = cwd.display().to_string();
    let argv_json = serde_json::to_string(argv)?;
    let id = journal.write(|tx| {
        assignment::get(tx, assignment_id)?;
        if state::report_state(tx, assignment_id)?.is_some() {
            return Err(Error::Conflict(format!("{assignment_id} already has a report")));
        }
        let seq: i64 = tx.query_row("SELECT COALESCE(MAX(seq), 0) + 1 FROM receipt", [], |row| row.get(0))?;
        let id = format!("chk-{seq:03}");
        tx.execute(
            "INSERT INTO receipt (id, seq, assignment_id, cwd, argv_json, started_ms) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![id, seq, assignment_id, cwd_text, argv_json, now_ms()],
        )?;
        Ok(id)
    })?;
    let (exit_code, output) = match Command::new(program).args(&argv[1..]).current_dir(cwd).output() {
        Ok(result) => {
            let mut bytes = result.stdout;
            bytes.extend_from_slice(&result.stderr);
            (result.status.code(), bytes)
        }
        Err(error) => (None, format!("failed to start {program}: {error}").into_bytes()),
    };
    let tail = tail_text(&output);
    let digest = sha256_hex(&output);
    journal.write(|tx| {
        tx.execute(
            "UPDATE receipt SET finished_ms = ?1, exit_code = ?2, output_sha256 = ?3, output_tail = ?4 WHERE id = ?5",
            params![now_ms(), exit_code, digest, tail, id],
        )?;
        Ok(())
    })?;
    get(journal.conn(), &id)
}

pub fn get(conn: &Connection, id: &str) -> Result<Receipt> {
    let raw = conn
        .query_row(&format!("SELECT {COLUMNS} FROM receipt WHERE id = ?1"), [id], read_row)
        .optional()?;
    into_receipt(raw.ok_or_else(|| Error::NotFound(format!("check {id}")))?)
}

pub fn list_for(conn: &Connection, assignment_id: &str) -> Result<Vec<Receipt>> {
    let mut statement = conn.prepare(&format!("SELECT {COLUMNS} FROM receipt WHERE assignment_id = ?1 ORDER BY seq"))?;
    let rows = statement.query_map([assignment_id], read_row)?;
    let mut receipts = Vec::new();
    for row in rows {
        receipts.push(into_receipt(row?)?);
    }
    Ok(receipts)
}

fn tail_text(bytes: &[u8]) -> String {
    let start = bytes.len().saturating_sub(OUTPUT_TAIL_BYTES);
    String::from_utf8_lossy(&bytes[start..]).into_owned()
}

fn read_row(row: &Row<'_>) -> rusqlite::Result<(Receipt, String)> {
    Ok((
        Receipt {
            id: row.get(0)?,
            assignment_id: row.get(1)?,
            cwd: row.get(2)?,
            argv: Vec::new(),
            started_ms: row.get(4)?,
            finished_ms: row.get(5)?,
            exit_code: row.get(6)?,
            output_sha256: row.get(7)?,
            output_tail: row.get(8)?,
        },
        row.get(3)?,
    ))
}

fn into_receipt((receipt, argv_json): (Receipt, String)) -> Result<Receipt> {
    Ok(Receipt { argv: serde_json::from_str(&argv_json)?, ..receipt })
}
```

- [ ] **Шаг 4.** Добавить `pub mod receipt;` в `lib.rs`.

- [ ] **Шаг 5.** Запустить: `cargo test -p foxmorrow-core --test receipt`. Ожидается: 4 теста пройдены.

- [ ] **Шаг 6.** Коммит:

```sh
git add crates/foxmorrow-core/src crates/foxmorrow-core/tests/receipt.rs
git commit -m "Execute checks once and record receipts"
```

## Задача 8. Отчёты

Проверки отчёта QA тестируются в задаче 9: для назначения `qa` нужен принятый `review`.

**Файлы:** создать `report.rs`, тест `tests/report.rs`; изменить `lib.rs`.

- [ ] **Шаг 1.** Тест `crates/foxmorrow-core/tests/report.rs`:

```rust
use foxmorrow_core::Error;
use foxmorrow_core::assignment::{self, NewAssignment};
use foxmorrow_core::model::{Kind, Outcome};
use foxmorrow_core::report;
use foxmorrow_core::run::{self, NewRun, Run};
use foxmorrow_core::journal::Journal;

fn setup(dir: &std::path::Path, kind: Kind) -> (Run, Journal) {
    let (run, mut journal) = run::create_run(NewRun {
        project_root: dir.to_path_buf(),
        criteria_path: "task.md".into(),
        criteria_heading: "Критерии приёмки".into(),
        criteria_markdown: "## Критерии приёмки\n\n- Первый\n- Второй\n".into(),
        allowed_paths: vec!["src/".into()],
        baselines: Vec::new(),
        root_session_id: None,
        family_rule: true,
    })
    .unwrap();
    let delegated = kind != Kind::Root;
    assignment::create(
        &mut journal,
        &run,
        NewAssignment {
            role: if delegated { "backend-worker".into() } else { "root".into() },
            kind,
            profile: delegated.then(|| "foxmorrow:backend-worker".to_string()),
            expected_models: if delegated { vec!["swe-2-high".into()] } else { Vec::new() },
            brief_sha256: None,
            role_sha256: None,
            allowed_paths: vec!["src/".into()],
            depends_on: Vec::new(),
            previous_id: None,
        },
    )
    .unwrap();
    (run, journal)
}

fn invalid(result: foxmorrow_core::Result<report::ReportBody>, needle: &str) {
    match result {
        Err(Error::Invalid(message)) => assert!(message.contains(needle), "{message}"),
        other => panic!("expected invalid '{needle}', got {other:?}"),
    }
}

#[test]
fn stores_one_report_per_assignment() {
    let dir = tempfile::tempdir().unwrap();
    let (run, mut journal) = setup(dir.path(), Kind::Author);
    let body = report::submit(
        &mut journal,
        &run,
        "a-01",
        r#"{"summary":"done","status":"completed","outcome":"passed","criteria":{"ac-1":{"status":"passed","reason":"tested"}}}"#,
    )
    .unwrap();
    let (stored, sha) = report::get(journal.conn(), "a-01").unwrap().unwrap();
    assert_eq!(stored, body);
    assert_eq!(sha.len(), 64);
    let again = report::submit(&mut journal, &run, "a-01", r#"{"summary":"x","status":"completed","outcome":"failed"}"#);
    assert!(matches!(again, Err(Error::Conflict(_))));
}

#[test]
fn rejects_malformed_reports() {
    let dir = tempfile::tempdir().unwrap();
    let (run, mut journal) = setup(dir.path(), Kind::Author);
    invalid(report::submit(&mut journal, &run, "a-01", r#"{"summary":" ","status":"completed","outcome":"failed"}"#), "summary");
    invalid(report::submit(&mut journal, &run, "a-01", r#"{"summary":"x","status":"interrupted","outcome":"passed"}"#), "completed");
    invalid(
        report::submit(&mut journal, &run, "a-01", r#"{"summary":"x","status":"completed","outcome":"failed","criteria":{"ac-9":{"status":"failed","reason":"r"}}}"#),
        "unknown criterion",
    );
    invalid(
        report::submit(&mut journal, &run, "a-01", r#"{"summary":"x","status":"completed","outcome":"failed","criteria":{"ac-1":{"status":"failed","reason":""}}}"#),
        "reason",
    );
    assert!(matches!(
        report::submit(&mut journal, &run, "a-01", r#"{"summary":"x","status":"completed","outcome":"passed","extra":1}"#),
        Err(Error::Json(_))
    ));
    assert!(report::get(journal.conn(), "a-01").unwrap().is_none());
}

#[test]
fn root_stop_needs_reason_and_unverified_list() {
    let dir = tempfile::tempdir().unwrap();
    let (run, mut journal) = setup(dir.path(), Kind::Root);
    invalid(report::submit(&mut journal, &run, "a-01", r#"{"summary":"stop","status":"completed","outcome":"failed"}"#), "reason");
    invalid(
        report::submit(&mut journal, &run, "a-01", r#"{"summary":"stop","status":"completed","outcome":"unverified","reason":"no access"}"#),
        "unverified list",
    );
    let body = report::submit(
        &mut journal,
        &run,
        "a-01",
        r#"{"summary":"stop","status":"completed","outcome":"unverified","reason":"no access","unverified":["ac-2"]}"#,
    )
    .unwrap();
    assert_eq!(body.outcome, Outcome::Unverified);
}

#[cfg(unix)]
mod with_checks {
    use super::*;
    use foxmorrow_core::receipt;

    fn check(journal: &mut Journal, dir: &std::path::Path, script: &str) -> String {
        receipt::run_check(journal, "a-01", dir, &["sh".into(), "-c".into(), script.into()]).unwrap().id
    }

    #[test]
    fn passing_evidence_must_have_exit_code_zero() {
        let dir = tempfile::tempdir().unwrap();
        let (run, mut journal) = setup(dir.path(), Kind::Author);
        let failed = check(&mut journal, dir.path(), "exit 1");
        let body = format!(
            r#"{{"summary":"x","status":"completed","outcome":"failed","criteria":{{"ac-1":{{"status":"passed","reason":"r","evidence":["{failed}"]}}}}}}"#
        );
        invalid(report::submit(&mut journal, &run, "a-01", &body), "did not succeed");
        let honest = format!(r#"{{"summary":"x","status":"completed","outcome":"failed","evidence":["{failed}"]}}"#);
        assert!(report::submit(&mut journal, &run, "a-01", &honest).is_ok());
    }

    #[test]
    fn evidence_must_belong_to_the_assignment() {
        let dir = tempfile::tempdir().unwrap();
        let (run, mut journal) = setup(dir.path(), Kind::Author);
        invalid(
            report::submit(&mut journal, &run, "a-01", r#"{"summary":"x","status":"completed","outcome":"failed","evidence":["chk-404"]}"#),
            "does not belong",
        );
    }

    #[test]
    fn check_without_result_forces_interrupted() {
        let dir = tempfile::tempdir().unwrap();
        let (run, mut journal) = setup(dir.path(), Kind::Author);
        journal
            .conn()
            .execute(
                "INSERT INTO receipt (id, seq, assignment_id, cwd, argv_json, started_ms) VALUES ('chk-001', 1, 'a-01', '/', '[\"make\"]', 1)",
                [],
            )
            .unwrap();
        invalid(report::submit(&mut journal, &run, "a-01", r#"{"summary":"x","status":"completed","outcome":"failed"}"#), "interrupted");
        assert!(report::submit(&mut journal, &run, "a-01", r#"{"summary":"x","status":"interrupted","outcome":"unverified"}"#).is_ok());
    }

    #[test]
    fn no_checks_after_the_report() {
        let dir = tempfile::tempdir().unwrap();
        let (run, mut journal) = setup(dir.path(), Kind::Author);
        report::submit(&mut journal, &run, "a-01", r#"{"summary":"x","status":"completed","outcome":"failed"}"#).unwrap();
        let late = receipt::run_check(&mut journal, "a-01", dir.path(), &["true".into()]);
        assert!(matches!(late, Err(Error::Conflict(_))));
    }
}
```

- [ ] **Шаг 2.** Запустить: `cargo test -p foxmorrow-core --test report`. Ожидается ошибка компиляции.

- [ ] **Шаг 3.** `crates/foxmorrow-core/src/report.rs`:

```rust
use std::collections::{BTreeMap, BTreeSet};

use rusqlite::{Connection, OptionalExtension, params};
use serde::{Deserialize, Serialize};

use crate::assignment::{self, Assignment};
use crate::error::{Error, Result, invalid};
use crate::journal::Journal;
use crate::model::{Kind, Outcome, Status};
use crate::receipt::{self, Receipt};
use crate::run::Run;
use crate::util::{now_ms, sha256_hex};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CriterionResult {
    pub status: Outcome,
    pub reason: String,
    #[serde(default)]
    pub evidence: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReportBody {
    pub summary: String,
    pub status: Status,
    pub outcome: Outcome,
    #[serde(default)]
    pub criteria: BTreeMap<String, CriterionResult>,
    #[serde(default)]
    pub evidence: Vec<String>,
    #[serde(default)]
    pub resolves: BTreeMap<String, Vec<String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub unverified: Vec<String>,
}

pub fn submit(journal: &mut Journal, run: &Run, assignment_id: &str, input: &str) -> Result<ReportBody> {
    let body: ReportBody = serde_json::from_str(input)?;
    journal.write(|tx| {
        let assignment = assignment::get(tx, assignment_id)?;
        if get(tx, assignment_id)?.is_some() {
            return Err(Error::Conflict(format!(
                "{assignment_id} already has a report; create a correction with --previous {assignment_id}"
            )));
        }
        let receipts = receipt::list_for(tx, assignment_id)?;
        validate(tx, run, &assignment, &receipts, &body)?;
        let json = serde_json::to_string(&body)?;
        tx.execute(
            "INSERT INTO report (assignment_id, created_ms, status, outcome, body_json, body_sha256) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![assignment_id, now_ms(), body.status.as_str(), body.outcome.as_str(), json, sha256_hex(json.as_bytes())],
        )?;
        Ok(())
    })?;
    Ok(body)
}

pub fn get(conn: &Connection, assignment_id: &str) -> Result<Option<(ReportBody, String)>> {
    let raw: Option<(String, String)> = conn
        .query_row(
            "SELECT body_json, body_sha256 FROM report WHERE assignment_id = ?1",
            [assignment_id],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .optional()?;
    match raw {
        Some((json, digest)) => Ok(Some((serde_json::from_str(&json)?, digest))),
        None => Ok(None),
    }
}

pub fn all_evidence(body: &ReportBody) -> Vec<&str> {
    body.evidence
        .iter()
        .chain(body.criteria.values().flat_map(|result| result.evidence.iter()))
        .map(String::as_str)
        .collect()
}

pub fn passing_evidence(body: &ReportBody) -> Vec<&str> {
    let mut references: Vec<&str> = body
        .criteria
        .values()
        .filter(|result| result.status == Outcome::Passed)
        .flat_map(|result| result.evidence.iter().map(String::as_str))
        .collect();
    if body.outcome == Outcome::Passed {
        references.extend(body.evidence.iter().map(String::as_str));
    }
    references
}

fn validate(conn: &Connection, run: &Run, assignment: &Assignment, receipts: &[Receipt], body: &ReportBody) -> Result<()> {
    if body.summary.trim().is_empty() {
        return invalid("summary must not be empty");
    }
    if body.outcome == Outcome::Passed && body.status != Status::Completed {
        return invalid("outcome passed requires status completed");
    }
    if let Some(open) = receipts.iter().find(|receipt| receipt.finished_ms.is_none())
        && body.status != Status::Interrupted
    {
        return invalid(format!(
            "check {} has no recorded result; inspect it and report status interrupted",
            open.id
        ));
    }
    let known: BTreeSet<&str> = run.criteria.iter().map(|criterion| criterion.id.as_str()).collect();
    for (id, result) in &body.criteria {
        if !known.contains(id.as_str()) {
            return invalid(format!("unknown criterion {id}"));
        }
        if result.reason.trim().is_empty() {
            return invalid(format!("criterion {id} needs a reason"));
        }
    }
    if assignment.kind == Kind::Qa {
        if let Some(missing) = run.criteria.iter().find(|criterion| !body.criteria.contains_key(&criterion.id)) {
            return invalid(format!("qa report must cover {}", missing.id));
        }
        if body.outcome == Outcome::Passed && body.criteria.values().any(|result| result.status != Outcome::Passed) {
            return invalid("qa outcome passed requires every criterion passed");
        }
    }
    let passing = passing_evidence(body);
    for reference in all_evidence(body).into_iter().filter(|reference| reference.starts_with("chk-")) {
        let Some(receipt) = receipts.iter().find(|receipt| receipt.id == reference) else {
            return invalid(format!("check {reference} does not belong to {}", assignment.id));
        };
        if receipt.finished_ms.is_none() {
            return invalid(format!("check {reference} has no recorded result"));
        }
        if passing.contains(&reference) && receipt.exit_code != Some(0) {
            return invalid(format!("check {reference} did not succeed (exit code {:?})", receipt.exit_code));
        }
    }
    for previous in body.resolves.keys() {
        match get(conn, previous)? {
            None => return invalid(format!("resolved assignment {previous} has no report")),
            Some((resolved, _)) if resolved.outcome == Outcome::Passed => {
                return invalid(format!("{previous} already passed; nothing to resolve"));
            }
            Some(_) => {}
        }
    }
    if assignment.kind == Kind::Root && body.outcome != Outcome::Passed {
        if body.reason.as_deref().is_none_or(|reason| reason.trim().is_empty()) {
            return invalid("root stop report requires a reason");
        }
        if body.outcome == Outcome::Unverified && body.unverified.is_empty() {
            return invalid("root stop report requires the unverified list");
        }
    }
    Ok(())
}
```

- [ ] **Шаг 4.** Добавить `pub mod report;` в `lib.rs`.

- [ ] **Шаг 5.** Запустить: `cargo test -p foxmorrow-core --test report`. Ожидается: 7 тестов пройдены на macOS и Linux, 3 на Windows.

- [ ] **Шаг 6.** Коммит:

```sh
git add crates/foxmorrow-core/src crates/foxmorrow-core/tests/report.rs
git commit -m "Validate and store immutable reports"
```

## Задача 9. Утверждения C1–C6 и `verify`

`ChainObservation` описывает одну цепочку субагента в виде, не зависящем от среды. Маркер роли `foxmorrow-role-sha256: <hash>` генератор ролей из плана 003 вставляет в текст каждой роли. Связь квитанции с вызовом определяется так: в цепочке есть `exec` с `foxmorrow check -a <id>`, и его команда без кавычек содержит argv квитанции.

**Файлы:** создать `claims.rs`, тест `tests/claims.rs`; изменить `lib.rs`; добавить в git `tests/common/mod.rs`.

- [ ] **Шаг 1.** Тест `crates/foxmorrow-core/tests/claims.rs`:

```rust
mod common;

use common::*;
use foxmorrow_core::Error;
use foxmorrow_core::assignment;
use foxmorrow_core::claims::{self, ClaimState, ToolCall};
use foxmorrow_core::model::{Kind, VerificationState};
use foxmorrow_core::report;
use foxmorrow_core::state;

fn claim_state(claims: &[claims::Claim], id: &str) -> ClaimState {
    claims.iter().find(|claim| claim.id == id).unwrap().state
}

#[test]
fn matching_observation_is_accepted() {
    let dir = tempfile::tempdir().unwrap();
    let (run, mut journal) = new_run(dir.path());
    let author = accept(&mut journal, &run, spec("backend-worker", Kind::Author, "swe-2-high", &[]), "agent-1", "swe-2-high", &passed_report(&[]));
    assert!(state::is_accepted(journal.conn(), &author.id).unwrap());
    assert_eq!(
        claims::accepted_observation(journal.conn(), &author.id).unwrap().unwrap().agent_id,
        "agent-1"
    );
}

#[test]
fn missing_observation_stays_unverified_and_can_be_retried() {
    let dir = tempfile::tempdir().unwrap();
    let (run, mut journal) = new_run(dir.path());
    let author = assignment::create(&mut journal, &run, spec("backend-worker", Kind::Author, "swe-2-high", &[])).unwrap();
    report::submit(&mut journal, &run, &author.id, &passed_report(&[])).unwrap();
    let first = claims::verify(&mut journal, &run, &author.id, None, &families()).unwrap();
    assert_eq!(first.state, VerificationState::Unverified);
    assert_eq!(claim_state(&first.claims, "C3"), ClaimState::Unverified);
    let second = claims::verify(&mut journal, &run, &author.id, Some(observation(&author, "agent-1", "swe-2-high")), &families()).unwrap();
    assert_eq!(second.state, VerificationState::Accepted);
    let third = claims::verify(&mut journal, &run, &author.id, Some(observation(&author, "agent-1", "swe-2-high")), &families());
    assert!(matches!(third, Err(Error::Conflict(_))));
}

#[test]
fn delivery_authorship_and_model_mismatches_are_rejected() {
    let dir = tempfile::tempdir().unwrap();
    let (run, mut journal) = new_run(dir.path());
    let author = assignment::create(&mut journal, &run, spec("backend-worker", Kind::Author, "swe-2-high", &[])).unwrap();
    report::submit(&mut journal, &run, &author.id, &passed_report(&[])).unwrap();
    let good = observation(&author, "agent-1", "swe-2-high");

    let cases: Vec<(&str, claims::ChainObservation)> = vec![
        ("C1", claims::ChainObservation { profile: "foxmorrow:reviewer".into(), ..good.clone() }),
        ("C1", claims::ChainObservation { task_text: format!("assignment {}", author.id), ..good.clone() }),
        ("C1", claims::ChainObservation { system_texts: vec!["plain role".into()], ..good.clone() }),
        ("C2", claims::ChainObservation { tool_calls: Vec::new(), ..good.clone() }),
        ("C3", claims::ChainObservation { models: vec!["gpt-6".into()], ..good.clone() }),
    ];
    for (failed, observed) in cases {
        let verification = claims::verify(&mut journal, &run, &author.id, Some(observed), &families()).unwrap();
        assert_eq!(verification.state, VerificationState::Rejected, "{failed}");
        assert_eq!(claim_state(&verification.claims, failed), ClaimState::Failed, "{failed}");
    }
}

#[test]
fn review_after_accepted_author_and_qa_after_passed_review() {
    let dir = tempfile::tempdir().unwrap();
    let (run, mut journal) = new_run(dir.path());
    let (_, _, qa) = accepted_chain(&mut journal, &run);
    assert_eq!(qa.id, "a-03");
}

#[test]
fn qa_requires_a_passed_review() {
    let dir = tempfile::tempdir().unwrap();
    let (run, mut journal) = new_run(dir.path());
    let author = accept(&mut journal, &run, spec("backend-worker", Kind::Author, "swe-2-high", &[]), "agent-1", "swe-2-high", &passed_report(&[]));
    let review = accept(
        &mut journal,
        &run,
        spec("reviewer", Kind::Review, "gpt-6", &[&author.id]),
        "agent-2",
        "gpt-6",
        r#"{"summary":"defects","status":"completed","outcome":"failed"}"#,
    );
    let error = assignment::create(&mut journal, &run, spec("qa-verifier", Kind::Qa, "opus-5", &[&review.id])).unwrap_err();
    assert!(error.to_string().contains("did not pass"), "{error}");
    let error = assignment::create(&mut journal, &run, spec("qa-verifier", Kind::Qa, "opus-5", &[&author.id])).unwrap_err();
    assert!(error.to_string().contains("passed review"), "{error}");
}

#[test]
fn qa_report_must_cover_every_criterion() {
    let dir = tempfile::tempdir().unwrap();
    let (run, mut journal) = new_run(dir.path());
    let author = accept(&mut journal, &run, spec("backend-worker", Kind::Author, "swe-2-high", &[]), "agent-1", "swe-2-high", &passed_report(&[]));
    let review = accept(&mut journal, &run, spec("reviewer", Kind::Review, "gpt-6", &[&author.id]), "agent-2", "gpt-6", &passed_report(&[]));
    let qa = assignment::create(&mut journal, &run, spec("qa-verifier", Kind::Qa, "opus-5", &[&review.id])).unwrap();
    let partial = report::submit(&mut journal, &run, &qa.id, &passed_report(&["ac-1"]));
    assert!(partial.unwrap_err().to_string().contains("must cover ac-2"));
    let mixed = r#"{"summary":"x","status":"completed","outcome":"passed","criteria":{"ac-1":{"status":"passed","reason":"r"},"ac-2":{"status":"failed","reason":"r"}}}"#;
    assert!(report::submit(&mut journal, &run, &qa.id, mixed).unwrap_err().to_string().contains("every criterion"));
}

#[test]
fn family_rule_rejects_a_reviewer_of_the_author_family() {
    let dir = tempfile::tempdir().unwrap();
    let (run, mut journal) = new_run(dir.path());
    let author = accept(&mut journal, &run, spec("backend-worker", Kind::Author, "swe-2-high", &[]), "agent-1", "swe-2-high", &passed_report(&[]));
    let review = assignment::create(&mut journal, &run, spec("reviewer", Kind::Review, "swe-2-low", &[&author.id])).unwrap();
    report::submit(&mut journal, &run, &review.id, &passed_report(&[])).unwrap();
    let verification = claims::verify(&mut journal, &run, &review.id, Some(observation(&review, "agent-2", "swe-2-low")), &families()).unwrap();
    assert_eq!(claim_state(&verification.claims, "C3"), ClaimState::Failed);

    let unknown = claims::verify(&mut journal, &run, &review.id, Some(observation(&review, "agent-2", "swe-2-low")), &Default::default()).unwrap();
    assert_eq!(claim_state(&unknown.claims, "C3"), ClaimState::Unverified);
}

#[test]
fn family_rule_can_be_disabled_for_the_run() {
    let dir = tempfile::tempdir().unwrap();
    let (run, mut journal) = new_run_with(dir.path(), false);
    let author = accept(&mut journal, &run, spec("backend-worker", Kind::Author, "swe-2-high", &[]), "agent-1", "swe-2-high", &passed_report(&[]));
    accept(&mut journal, &run, spec("reviewer", Kind::Review, "swe-2-low", &[&author.id]), "agent-2", "swe-2-low", &passed_report(&[]));
}

#[test]
fn verification_chain_must_be_independent() {
    let dir = tempfile::tempdir().unwrap();
    let (run, mut journal) = new_run(dir.path());
    let author = accept(&mut journal, &run, spec("backend-worker", Kind::Author, "swe-2-high", &[]), "agent-1", "swe-2-high", &passed_report(&[]));
    let review = assignment::create(&mut journal, &run, spec("reviewer", Kind::Review, "gpt-6", &[&author.id])).unwrap();
    report::submit(&mut journal, &run, &review.id, &passed_report(&[])).unwrap();

    let same_agent = claims::verify(&mut journal, &run, &review.id, Some(observation(&review, "agent-1", "gpt-6")), &families()).unwrap();
    assert_eq!(claim_state(&same_agent.claims, "C5"), ClaimState::Failed);

    let mut edited = observation(&review, "agent-2", "gpt-6");
    edited.tool_calls.push(ToolCall { id: "e1".into(), name: "edit".into(), command: None, exit_code: None });
    let editing = claims::verify(&mut journal, &run, &review.id, Some(edited), &families()).unwrap();
    assert_eq!(claim_state(&editing.claims, "C5"), ClaimState::Failed);
}

#[test]
fn tool_call_evidence_must_be_in_the_chain_and_succeed() {
    let dir = tempfile::tempdir().unwrap();
    let (run, mut journal) = new_run(dir.path());
    let author = assignment::create(&mut journal, &run, spec("backend-worker", Kind::Author, "swe-2-high", &[])).unwrap();
    report::submit(
        &mut journal,
        &run,
        &author.id,
        r#"{"summary":"x","status":"completed","outcome":"passed","criteria":{"ac-1":{"status":"passed","reason":"r","evidence":["exec-7"]}}}"#,
    )
    .unwrap();
    let missing = claims::verify(&mut journal, &run, &author.id, Some(observation(&author, "agent-1", "swe-2-high")), &families()).unwrap();
    assert_eq!(claim_state(&missing.claims, "C4"), ClaimState::Failed);

    let mut failing = observation(&author, "agent-1", "swe-2-high");
    failing.tool_calls.push(ToolCall { id: "exec-7".into(), name: "exec".into(), command: Some("cargo test".into()), exit_code: Some(101) });
    let failed = claims::verify(&mut journal, &run, &author.id, Some(failing), &families()).unwrap();
    assert_eq!(claim_state(&failed.claims, "C4"), ClaimState::Failed);
}

#[cfg(unix)]
#[test]
fn check_evidence_must_be_run_from_the_chain() {
    use foxmorrow_core::receipt;

    let dir = tempfile::tempdir().unwrap();
    let (run, mut journal) = new_run(dir.path());
    let author = assignment::create(&mut journal, &run, spec("backend-worker", Kind::Author, "swe-2-high", &[])).unwrap();
    let check = receipt::run_check(&mut journal, &author.id, dir.path(), &["sh".into(), "-c".into(), "echo ok".into()]).unwrap();
    report::submit(
        &mut journal,
        &run,
        &author.id,
        &format!(r#"{{"summary":"x","status":"completed","outcome":"passed","evidence":["{}"]}}"#, check.id),
    )
    .unwrap();

    let without = claims::verify(&mut journal, &run, &author.id, Some(observation(&author, "agent-1", "swe-2-high")), &families()).unwrap();
    assert_eq!(claim_state(&without.claims, "C4"), ClaimState::Failed);

    let mut with = observation(&author, "agent-1", "swe-2-high");
    with.tool_calls.push(ToolCall {
        id: "exec-1".into(),
        name: "exec".into(),
        command: Some(format!("bin/foxmorrow check -a {} -- sh -c \"echo ok\"", author.id)),
        exit_code: Some(0),
    });
    let verified = claims::verify(&mut journal, &run, &author.id, Some(with), &families()).unwrap();
    assert_eq!(verified.state, VerificationState::Accepted, "{:?}", verified.claims);
}
```

- [ ] **Шаг 2.** Запустить: `cargo test -p foxmorrow-core --test claims`. Ожидается ошибка компиляции: модуля `claims` нет.

- [ ] **Шаг 3.** `crates/foxmorrow-core/src/claims.rs`:

```rust
use std::collections::{BTreeMap, BTreeSet};

use rusqlite::{Connection, OptionalExtension, params};
use serde::{Deserialize, Serialize};

use crate::assignment::{self, Assignment};
use crate::error::{Error, Result};
use crate::journal::Journal;
use crate::model::{Kind, VerificationState};
use crate::receipt::{self, Receipt};
use crate::report::{self, ReportBody};
use crate::run::Run;
use crate::state;
use crate::util::now_ms;

pub type ModelFamilies = BTreeMap<String, String>;

const EDIT_TOOLS: [&str; 4] = ["edit", "write", "apply_patch", "notebook_edit"];

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ToolCall {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub command: Option<String>,
    #[serde(default)]
    pub exit_code: Option<i64>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChainObservation {
    pub source: String,
    pub agent_id: String,
    pub profile: String,
    pub task_text: String,
    pub system_texts: Vec<String>,
    pub models: Vec<String>,
    pub tool_calls: Vec<ToolCall>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ClaimState {
    Verified,
    Failed,
    Unverified,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Claim {
    pub id: String,
    pub state: ClaimState,
    pub detail: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AuthorFacts {
    pub assignment_id: String,
    pub agent_id: String,
    pub models: Vec<String>,
}

pub struct ClaimInput<'a> {
    pub run: &'a Run,
    pub assignment: &'a Assignment,
    pub report: &'a ReportBody,
    pub receipts: &'a [Receipt],
    pub authors: &'a [AuthorFacts],
    pub families: &'a ModelFamilies,
    pub depends_accepted: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Verification {
    pub assignment_id: String,
    pub state: VerificationState,
    pub claims: Vec<Claim>,
}

pub fn verify(
    journal: &mut Journal,
    run: &Run,
    assignment_id: &str,
    observation: Option<ChainObservation>,
    families: &ModelFamilies,
) -> Result<Verification> {
    journal.write(|tx| {
        if state::is_accepted(tx, assignment_id)? {
            return Err(Error::Conflict(format!("{assignment_id} is already accepted")));
        }
        let claims = evaluate_recorded(tx, run, assignment_id, observation.as_ref(), families)?;
        let verdict = verdict_of(&claims);
        let observation_json = observation.as_ref().map(serde_json::to_string).transpose()?;
        tx.execute(
            "INSERT INTO verification (assignment_id, created_ms, state, claims_json, observation_json) VALUES (?1, ?2, ?3, ?4, ?5)",
            params![assignment_id, now_ms(), verdict.as_str(), serde_json::to_string(&claims)?, observation_json],
        )?;
        Ok(Verification { assignment_id: assignment_id.to_string(), state: verdict, claims })
    })
}

pub fn accepted_observation(conn: &Connection, assignment_id: &str) -> Result<Option<ChainObservation>> {
    let raw: Option<Option<String>> = conn
        .query_row(
            "SELECT observation_json FROM verification WHERE assignment_id = ?1 AND state = 'accepted'",
            [assignment_id],
            |row| row.get(0),
        )
        .optional()?;
    match raw.flatten() {
        Some(json) => Ok(Some(serde_json::from_str(&json)?)),
        None => Ok(None),
    }
}

pub(crate) fn evaluate_recorded(
    conn: &Connection,
    run: &Run,
    assignment_id: &str,
    observation: Option<&ChainObservation>,
    families: &ModelFamilies,
) -> Result<Vec<Claim>> {
    let assignment = assignment::get(conn, assignment_id)?;
    let (report, _) = report::get(conn, assignment_id)?
        .ok_or_else(|| Error::NotFound(format!("report for {assignment_id}")))?;
    let receipts = receipt::list_for(conn, assignment_id)?;
    let authors = author_facts(conn, &assignment)?;
    let mut depends_accepted = true;
    for id in &assignment.depends_on {
        depends_accepted &= state::is_accepted(conn, id)?;
    }
    let input = ClaimInput {
        run,
        assignment: &assignment,
        report: &report,
        receipts: &receipts,
        authors: &authors,
        families,
        depends_accepted,
    };
    Ok(evaluate(&input, observation))
}

pub fn evaluate(input: &ClaimInput<'_>, observation: Option<&ChainObservation>) -> Vec<Claim> {
    let order = if input.depends_accepted {
        claim("C6", ClaimState::Verified, "dependencies accepted")
    } else {
        claim("C6", ClaimState::Failed, "a dependency is not accepted")
    };
    let Some(observed) = observation else {
        let mut claims: Vec<Claim> = ["C1", "C2", "C3", "C4", "C5"]
            .iter()
            .map(|id| claim(id, ClaimState::Unverified, "no native observation"))
            .collect();
        claims.push(order);
        return claims;
    };
    vec![
        delivery(input, observed),
        authorship(input, observed),
        model(input, observed),
        actions(input, observed),
        independence(input, observed),
        order,
    ]
}

fn claim(id: &str, state: ClaimState, detail: impl Into<String>) -> Claim {
    Claim { id: id.to_string(), state, detail: detail.into() }
}

fn verdict_of(claims: &[Claim]) -> VerificationState {
    if claims.iter().any(|claim| claim.state == ClaimState::Failed) {
        VerificationState::Rejected
    } else if claims.iter().any(|claim| claim.state == ClaimState::Unverified) {
        VerificationState::Unverified
    } else {
        VerificationState::Accepted
    }
}

fn delivery(input: &ClaimInput<'_>, observed: &ChainObservation) -> Claim {
    let assignment = input.assignment;
    if assignment.kind == Kind::Root {
        return claim("C1", ClaimState::Verified, "root report without delegation");
    }
    if assignment.profile.as_deref() != Some(observed.profile.as_str()) {
        return claim("C1", ClaimState::Failed, format!("profile {} does not match the assignment", observed.profile));
    }
    if !has_token(&observed.task_text, &assignment.id) {
        return claim("C1", ClaimState::Failed, "task does not contain the assignment id");
    }
    let Some(brief) = &assignment.brief_sha256 else {
        return claim("C1", ClaimState::Unverified, "brief hash was not recorded");
    };
    if !observed.task_text.contains(brief.as_str()) {
        return claim("C1", ClaimState::Failed, "task does not contain the brief hash");
    }
    let Some(role) = &assignment.role_sha256 else {
        return claim("C1", ClaimState::Unverified, "role hash was not recorded");
    };
    let marker = format!("foxmorrow-role-sha256: {role}");
    if !observed.system_texts.iter().any(|text| text.contains(&marker)) {
        return claim("C1", ClaimState::Failed, "role text with the recorded hash is missing");
    }
    claim("C1", ClaimState::Verified, "task and role delivered")
}

fn authorship(input: &ClaimInput<'_>, observed: &ChainObservation) -> Claim {
    let id = &input.assignment.id;
    let reported = observed
        .tool_calls
        .iter()
        .any(|call| call.name == "exec" && call.command.as_deref().is_some_and(|command| invokes(command, "report", id)));
    if reported {
        claim("C2", ClaimState::Verified, "report submitted from this chain")
    } else {
        claim("C2", ClaimState::Failed, format!("no foxmorrow report call for {id} in this chain"))
    }
}

fn model(input: &ClaimInput<'_>, observed: &ChainObservation) -> Claim {
    if observed.models.is_empty() {
        return claim("C3", ClaimState::Unverified, "no model recorded");
    }
    let expected = &input.assignment.expected_models;
    if !expected.is_empty()
        && let Some(unexpected) = observed.models.iter().find(|model| !expected.contains(model))
    {
        return claim(
            "C3",
            ClaimState::Failed,
            format!("model {unexpected} is not expected ({})", expected.join(", ")),
        );
    }
    if input.run.family_rule && input.assignment.kind.is_verification() {
        let Some(own) = families_of(&observed.models, input.families) else {
            return claim("C3", ClaimState::Unverified, "model family is unknown");
        };
        let author_models: Vec<String> = input.authors.iter().flat_map(|author| author.models.iter().cloned()).collect();
        if author_models.is_empty() {
            return claim("C3", ClaimState::Unverified, "author models are unknown");
        }
        let Some(theirs) = families_of(&author_models, input.families) else {
            return claim("C3", ClaimState::Unverified, "author model family is unknown");
        };
        if let Some(shared) = own.intersection(&theirs).next() {
            return claim("C3", ClaimState::Failed, format!("shares model family {shared} with the author"));
        }
    }
    claim("C3", ClaimState::Verified, "model matches the profile")
}

fn actions(input: &ClaimInput<'_>, observed: &ChainObservation) -> Claim {
    let id = &input.assignment.id;
    let references = report::all_evidence(input.report);
    if references.is_empty() {
        return claim("C4", ClaimState::Verified, "no evidence references");
    }
    let passing = report::passing_evidence(input.report);
    for reference in references {
        if reference.starts_with("chk-") {
            let Some(receipt) = input.receipts.iter().find(|receipt| receipt.id == reference) else {
                return claim("C4", ClaimState::Failed, format!("check {reference} does not belong to {id}"));
            };
            let argv = unquoted(&receipt.argv.join(" "));
            let ran_here = observed.tool_calls.iter().any(|call| {
                call.name == "exec"
                    && call
                        .command
                        .as_deref()
                        .is_some_and(|command| invokes(command, "check", id) && unquoted(command).contains(&argv))
            });
            if !ran_here {
                return claim("C4", ClaimState::Failed, format!("check {reference} was not run from this chain"));
            }
        } else {
            let Some(call) = observed.tool_calls.iter().find(|call| call.id == reference) else {
                return claim("C4", ClaimState::Failed, format!("tool call {reference} is not in this chain"));
            };
            if passing.contains(&reference)
                && let Some(code) = call.exit_code.filter(|code| *code != 0)
            {
                return claim("C4", ClaimState::Failed, format!("tool call {reference} exited with {code}"));
            }
        }
    }
    claim("C4", ClaimState::Verified, "evidence found in this chain")
}

fn independence(input: &ClaimInput<'_>, observed: &ChainObservation) -> Claim {
    if !input.assignment.kind.is_verification() {
        return claim("C5", ClaimState::Verified, "not a verification assignment");
    }
    if let Some(author) = input.authors.iter().find(|author| author.agent_id == observed.agent_id) {
        return claim(
            "C5",
            ClaimState::Failed,
            format!("same agent as author assignment {}", author.assignment_id),
        );
    }
    if let Some(call) = observed.tool_calls.iter().find(|call| EDIT_TOOLS.contains(&call.name.as_str())) {
        return claim("C5", ClaimState::Failed, format!("verification chain used {}", call.name));
    }
    claim("C5", ClaimState::Verified, "independent of the authors")
}

fn author_facts(conn: &Connection, assignment: &Assignment) -> Result<Vec<AuthorFacts>> {
    let mut pending = assignment.depends_on.clone();
    let mut seen = BTreeSet::new();
    let mut facts = Vec::new();
    while let Some(id) = pending.pop() {
        if !seen.insert(id.clone()) {
            continue;
        }
        let dependency = assignment::get(conn, &id)?;
        if dependency.kind == Kind::Author
            && let Some(observed) = accepted_observation(conn, &id)?
        {
            facts.push(AuthorFacts { assignment_id: id, agent_id: observed.agent_id, models: observed.models });
        }
        pending.extend(dependency.depends_on);
    }
    Ok(facts)
}

fn families_of(models: &[String], families: &ModelFamilies) -> Option<BTreeSet<String>> {
    models.iter().map(|model| families.get(model).cloned()).collect()
}

fn invokes(command: &str, subcommand: &str, assignment_id: &str) -> bool {
    let words: Vec<&str> = command.split_whitespace().collect();
    let calls = words.windows(2).any(|pair| pair[0].ends_with("foxmorrow") && pair[1] == subcommand);
    let names = words
        .windows(2)
        .any(|pair| matches!(pair[0], "-a" | "--assignment") && pair[1] == assignment_id);
    calls && names
}

fn has_token(text: &str, token: &str) -> bool {
    text.split(|c: char| !(c.is_ascii_alphanumeric() || c == '-' || c == '_'))
        .any(|part| part == token)
}

fn unquoted(text: &str) -> String {
    text.chars().filter(|c| *c != '"' && *c != '\'').collect()
}
```

- [ ] **Шаг 4.** Добавить `pub mod claims;` в `lib.rs`.

- [ ] **Шаг 5.** Запустить: `cargo test -p foxmorrow-core`. Ожидается: проходят все тесты крейта, включая 11 тестов `claims` (10 на Windows).

- [ ] **Шаг 6.** Коммит:

```sh
git add crates/foxmorrow-core/src crates/foxmorrow-core/tests/common crates/foxmorrow-core/tests/claims.rs
git commit -m "Evaluate claims C1-C6 against normalized chain observations"
```

## Задача 10. `finalize`

**Файлы:** создать `finalize.rs`, тест `tests/finalize.rs`; изменить `lib.rs`.

- [ ] **Шаг 1.** Тест `crates/foxmorrow-core/tests/finalize.rs`:

```rust
mod common;

use common::*;
use foxmorrow_core::Error;
use foxmorrow_core::assignment;
use foxmorrow_core::claims;
use foxmorrow_core::finalize;
use foxmorrow_core::model::{Kind, Outcome, VerificationState};
use foxmorrow_core::report;

#[test]
fn closes_an_accepted_chain() {
    let dir = tempfile::tempdir().unwrap();
    let (run, mut journal) = new_run(dir.path());
    let (author, review, qa) = accepted_chain(&mut journal, &run);
    let record = finalize::finalize(&mut journal, &run, &qa.id, &families()).unwrap();
    assert_eq!(record.outcome, Outcome::Passed);
    assert_eq!(record.reports, vec![author.id.clone(), review.id, qa.id.clone()]);
    assert!(record.family_rule);

    assert!(matches!(finalize::finalize(&mut journal, &run, &qa.id, &families()), Err(Error::Conflict(_))));
    let late = assignment::create(&mut journal, &run, spec("backend-worker", Kind::Author, "swe-2-high", &[]));
    assert!(late.unwrap_err().to_string().contains("run is finalized"));
}

#[test]
fn only_qa_or_root_reports_close_a_run() {
    let dir = tempfile::tempdir().unwrap();
    let (run, mut journal) = new_run(dir.path());
    let (author, _, _) = accepted_chain(&mut journal, &run);
    assert!(matches!(finalize::finalize(&mut journal, &run, &author.id, &families()), Err(Error::Invalid(_))));
}

#[test]
fn unaccepted_qa_cannot_close_a_run() {
    let dir = tempfile::tempdir().unwrap();
    let (run, mut journal) = new_run(dir.path());
    let author = accept(&mut journal, &run, spec("backend-worker", Kind::Author, "swe-2-high", &[]), "agent-1", "swe-2-high", &passed_report(&[]));
    let review = accept(&mut journal, &run, spec("reviewer", Kind::Review, "gpt-6", &[&author.id]), "agent-2", "gpt-6", &passed_report(&[]));
    let qa = assignment::create(&mut journal, &run, spec("qa-verifier", Kind::Qa, "opus-5", &[&review.id])).unwrap();
    report::submit(&mut journal, &run, &qa.id, &passed_report(&["ac-1", "ac-2"])).unwrap();
    let error = finalize::finalize(&mut journal, &run, &qa.id, &families()).unwrap_err();
    assert!(error.to_string().contains("not accepted"), "{error}");
}

#[test]
fn snapshots_are_rechecked() {
    let dir = tempfile::tempdir().unwrap();
    let (run, mut journal) = new_run(dir.path());
    let (_, _, qa) = accepted_chain(&mut journal, &run);
    let mut merged = families();
    merged.insert("gpt-6".into(), "swe".into());
    let error = finalize::finalize(&mut journal, &run, &qa.id, &merged).unwrap_err();
    assert!(error.to_string().contains("no longer verifies"), "{error}");
}

#[test]
fn root_can_stop_with_a_non_positive_outcome() {
    let dir = tempfile::tempdir().unwrap();
    let (run, mut journal) = new_run(dir.path());
    let root = assignment::create(&mut journal, &run, root_spec()).unwrap();
    report::submit(
        &mut journal,
        &run,
        &root.id,
        r#"{"summary":"stopped","status":"completed","outcome":"unverified","reason":"no browser access","unverified":["ac-2"]}"#,
    )
    .unwrap();
    let mut observed = observation(&root, "root", "opus-5");
    observed.profile = String::new();
    let verification = claims::verify(&mut journal, &run, &root.id, Some(observed), &families()).unwrap();
    assert_eq!(verification.state, VerificationState::Accepted, "{:?}", verification.claims);
    let record = finalize::finalize(&mut journal, &run, &root.id, &families()).unwrap();
    assert_eq!(record.outcome, Outcome::Unverified);
    assert_eq!(record.reason.as_deref(), Some("no browser access"));
    assert_eq!(record.unverified, vec!["ac-2".to_string()]);
}

#[test]
fn root_cannot_claim_a_passed_final() {
    let dir = tempfile::tempdir().unwrap();
    let (run, mut journal) = new_run(dir.path());
    let root = assignment::create(&mut journal, &run, root_spec()).unwrap();
    report::submit(&mut journal, &run, &root.id, &passed_report(&[])).unwrap();
    let mut observed = observation(&root, "root", "opus-5");
    observed.profile = String::new();
    claims::verify(&mut journal, &run, &root.id, Some(observed), &families()).unwrap();
    assert!(matches!(finalize::finalize(&mut journal, &run, &root.id, &families()), Err(Error::Invalid(_))));
}
```

- [ ] **Шаг 2.** Запустить: `cargo test -p foxmorrow-core --test finalize`. Ожидается ошибка компиляции.

- [ ] **Шаг 3.** `crates/foxmorrow-core/src/finalize.rs`:

```rust
use std::collections::BTreeSet;

use rusqlite::{Connection, params};
use serde::{Deserialize, Serialize};

use crate::assignment;
use crate::claims::{self, ClaimState, ModelFamilies};
use crate::error::{Error, Result, invalid};
use crate::journal::Journal;
use crate::model::{Kind, Outcome, Status};
use crate::report;
use crate::run::Run;
use crate::state;
use crate::util::now_ms;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FinalRecord {
    pub run_id: String,
    pub report_id: String,
    pub outcome: Outcome,
    pub family_rule: bool,
    pub root_model: Option<String>,
    pub reports: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub unverified: Vec<String>,
}

pub fn finalize(journal: &mut Journal, run: &Run, report_id: &str, families: &ModelFamilies) -> Result<FinalRecord> {
    journal.write(|tx| {
        if state::is_finalized(tx)? {
            return Err(Error::Conflict("run is already finalized".into()));
        }
        let selected = assignment::get(tx, report_id)?;
        let Some((body, _)) = report::get(tx, report_id)? else {
            return invalid(format!("{report_id} has no report"));
        };
        match selected.kind {
            Kind::Qa if body.status == Status::Completed && body.outcome == Outcome::Passed => {}
            Kind::Qa => return invalid("a non-positive final requires a root stop report"),
            Kind::Root if body.outcome != Outcome::Passed => {}
            Kind::Root => return invalid("a root report can only stop the run with a non-positive outcome"),
            _ => return invalid("final requires a qa or root report"),
        }
        let reports = dependency_closure(tx, report_id)?;
        for id in &reports {
            let Some(observed) = claims::accepted_observation(tx, id)? else {
                return invalid(format!("{id} is not accepted"));
            };
            let recheck = claims::evaluate_recorded(tx, run, id, Some(&observed), families)?;
            if let Some(failed) = recheck.iter().find(|claim| claim.state != ClaimState::Verified) {
                return invalid(format!(
                    "{id} no longer verifies from its snapshot: {} {}",
                    failed.id, failed.detail
                ));
            }
        }
        let record = FinalRecord {
            run_id: run.id.clone(),
            report_id: report_id.to_string(),
            outcome: body.outcome,
            family_rule: run.family_rule,
            root_model: run.root_model.clone(),
            reports,
            reason: body.reason.clone(),
            unverified: body.unverified.clone(),
        };
        tx.execute(
            "INSERT INTO final (run_id, created_ms, report_id, outcome, body_json) VALUES (?1, ?2, ?3, ?4, ?5)",
            params![record.run_id, now_ms(), record.report_id, record.outcome.as_str(), serde_json::to_string(&record)?],
        )?;
        Ok(record)
    })
}

fn dependency_closure(conn: &Connection, start: &str) -> Result<Vec<String>> {
    let mut pending = vec![start.to_string()];
    let mut seen = BTreeSet::new();
    let mut ordered = Vec::new();
    while let Some(id) = pending.pop() {
        if !seen.insert(id.clone()) {
            continue;
        }
        let current = assignment::get(conn, &id)?;
        ordered.push((current.seq, id));
        pending.extend(current.depends_on);
    }
    ordered.sort();
    Ok(ordered.into_iter().map(|(_, id)| id).collect())
}
```

- [ ] **Шаг 4.** Добавить `pub mod finalize;` в `lib.rs`.

- [ ] **Шаг 5.** Запустить: `cargo test -p foxmorrow-core --test finalize`. Ожидается: 6 тестов пройдены.

- [ ] **Шаг 6.** Коммит:

```sh
git add crates/foxmorrow-core/src crates/foxmorrow-core/tests/finalize.rs
git commit -m "Finalize runs from rechecked snapshots"
```

## Задача 11. Сводка `status`

**Файлы:** создать `status.rs`, тест `tests/status.rs`; изменить `lib.rs`.

- [ ] **Шаг 1.** Тест `crates/foxmorrow-core/tests/status.rs`:

```rust
mod common;

use common::*;
use foxmorrow_core::assignment;
use foxmorrow_core::finalize;
use foxmorrow_core::model::{Kind, Outcome, VerificationState};
use foxmorrow_core::status;

#[test]
fn summarizes_assignments_reports_and_final() {
    let dir = tempfile::tempdir().unwrap();
    let (run, mut journal) = new_run(dir.path());
    let fresh = status::status(journal.conn(), &run).unwrap();
    assert!(fresh.assignments.is_empty());
    assert_eq!(fresh.criteria.len(), 2);

    let (author, _, qa) = accepted_chain(&mut journal, &run);
    let open = assignment::create(&mut journal, &run, spec("python-worker", Kind::Author, "swe-2-high", &[])).unwrap();
    journal
        .conn()
        .execute(
            "INSERT INTO receipt (id, seq, assignment_id, cwd, argv_json, started_ms) VALUES ('chk-900', 900, ?1, '/', '[\"make\"]', 1)",
            [&open.id],
        )
        .unwrap();

    let view = status::status(journal.conn(), &run).unwrap();
    let first = view.assignments.iter().find(|entry| entry.id == author.id).unwrap();
    assert_eq!(first.verification, Some(VerificationState::Accepted));
    assert_eq!(first.report.as_ref().unwrap().outcome, Outcome::Passed);
    let pending = view.assignments.iter().find(|entry| entry.id == open.id).unwrap();
    assert_eq!(pending.open_checks, vec!["chk-900".to_string()]);
    assert!(pending.report.is_none());
    assert_eq!(view.final_outcome, None);

    finalize::finalize(&mut journal, &run, &qa.id, &families()).unwrap();
    assert_eq!(status::status(journal.conn(), &run).unwrap().final_outcome, Some(Outcome::Passed));
}
```

- [ ] **Шаг 2.** Запустить: `cargo test -p foxmorrow-core --test status`. Ожидается ошибка компиляции.

- [ ] **Шаг 3.** `crates/foxmorrow-core/src/status.rs`:

```rust
use rusqlite::{Connection, OptionalExtension};
use serde::Serialize;

use crate::assignment;
use crate::criteria::Criterion;
use crate::error::Result;
use crate::model::{Kind, Outcome, Status, VerificationState};
use crate::receipt;
use crate::run::Run;
use crate::state;

#[derive(Debug, Serialize)]
pub struct StatusView {
    pub run_id: String,
    pub criteria: Vec<Criterion>,
    pub family_rule: bool,
    pub root_model: Option<String>,
    pub assignments: Vec<AssignmentStatus>,
    pub final_outcome: Option<Outcome>,
}

#[derive(Debug, Serialize)]
pub struct AssignmentStatus {
    pub id: String,
    pub role: String,
    pub kind: Kind,
    pub depends_on: Vec<String>,
    pub previous_id: Option<String>,
    pub report: Option<ReportState>,
    pub verification: Option<VerificationState>,
    pub open_checks: Vec<String>,
}

#[derive(Debug, Serialize)]
pub struct ReportState {
    pub status: Status,
    pub outcome: Outcome,
}

pub fn status(conn: &Connection, run: &Run) -> Result<StatusView> {
    let mut assignments = Vec::new();
    for entry in assignment::list(conn)? {
        let report = state::report_state(conn, &entry.id)?.map(|(status, outcome)| ReportState { status, outcome });
        let verification = state::latest_verification(conn, &entry.id)?;
        let open_checks = receipt::list_for(conn, &entry.id)?
            .into_iter()
            .filter(|receipt| receipt.finished_ms.is_none())
            .map(|receipt| receipt.id)
            .collect();
        assignments.push(AssignmentStatus {
            id: entry.id,
            role: entry.role,
            kind: entry.kind,
            depends_on: entry.depends_on,
            previous_id: entry.previous_id,
            report,
            verification,
            open_checks,
        });
    }
    let final_outcome: Option<String> = conn.query_row("SELECT outcome FROM final", [], |row| row.get(0)).optional()?;
    Ok(StatusView {
        run_id: run.id.clone(),
        criteria: run.criteria.clone(),
        family_rule: run.family_rule,
        root_model: run.root_model.clone(),
        assignments,
        final_outcome: final_outcome.map(|outcome| Outcome::parse(&outcome)).transpose()?,
    })
}
```

- [ ] **Шаг 4.** Итоговый `crates/foxmorrow-core/src/lib.rs`:

```rust
pub mod assignment;
pub mod baseline;
pub mod claims;
pub mod criteria;
pub mod error;
pub mod finalize;
pub mod journal;
pub mod model;
pub mod receipt;
pub mod report;
pub mod run;
pub mod schema;
pub mod state;
pub mod status;
pub mod util;

pub use error::{Error, Result};
```

- [ ] **Шаг 5.** Запустить: `cargo test -p foxmorrow-core`. Ожидается: все тесты крейта пройдены.

- [ ] **Шаг 6.** Коммит:

```sh
git add crates/foxmorrow-core
git commit -m "Summarize run status"
```

## Задача 12. CLI

**Файлы:** заменить `crates/foxmorrow/src/main.rs`; создать `crates/foxmorrow/src/project.rs`, `crates/foxmorrow/tests/cli.rs`.

- [ ] **Шаг 1.** Тест `crates/foxmorrow/tests/cli.rs`:

```rust
#![cfg(unix)]

use std::io::Write;
use std::path::Path;
use std::process::{Command, Output, Stdio};

fn foxmorrow(dir: &Path, args: &[&str], stdin: Option<&str>) -> Output {
    let mut child = Command::new(env!("CARGO_BIN_EXE_foxmorrow"))
        .args(args)
        .current_dir(dir)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    let mut pipe = child.stdin.take().unwrap();
    if let Some(input) = stdin {
        pipe.write_all(input.as_bytes()).unwrap();
    }
    drop(pipe);
    child.wait_with_output().unwrap()
}

fn json(output: &Output) -> serde_json::Value {
    assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));
    serde_json::from_slice(&output.stdout).unwrap()
}

fn project() -> tempfile::TempDir {
    let dir = tempfile::tempdir().unwrap();
    assert!(Command::new("git").arg("-C").arg(dir.path()).args(["init", "-q"]).status().unwrap().success());
    std::fs::write(dir.path().join("task.md"), "# Task\n\n## Критерии приёмки\n\n- Health returns 200\n").unwrap();
    dir
}

const ASSIGN: &[&str] = &[
    "assign", "--role", "backend-worker", "--kind", "author", "--profile", "foxmorrow:backend-worker",
    "--expected-model", "swe-2-high", "--paths", "src/",
];

#[test]
fn runs_init_assign_check_report_status() {
    let dir = project();
    let root = dir.path();
    let init = json(&foxmorrow(root, &["init", "--criteria", "task.md#Критерии приёмки", "--paths", "src/"], None));
    assert_eq!(init["criteria"][0]["id"], "ac-1");

    let assigned = json(&foxmorrow(root, ASSIGN, None));
    assert_eq!(assigned["id"], "a-01");

    let check = foxmorrow(root, &["check", "-a", "a-01", "--", "sh", "-c", "echo hello; exit 3"], None);
    assert_eq!(check.status.code(), Some(3));
    assert_eq!(String::from_utf8_lossy(&check.stdout), "hello\n");
    assert!(String::from_utf8_lossy(&check.stderr).contains("chk-001 exit=3"));

    let reported = json(&foxmorrow(
        root,
        &["report", "-a", "a-01"],
        Some(r#"{"summary":"tried","status":"completed","outcome":"failed","evidence":["chk-001"]}"#),
    ));
    assert_eq!(reported["outcome"], "failed");

    let status = json(&foxmorrow(root, &["status"], None));
    assert_eq!(status["assignments"][0]["report"]["outcome"], "failed");
    assert_eq!(status["assignments"][0]["verification"], serde_json::Value::Null);
}

#[test]
fn errors_exit_with_code_two() {
    let dir = project();
    let root = dir.path();
    json(&foxmorrow(root, &["init", "--criteria", "task.md#Критерии приёмки", "--paths", "src/"], None));
    json(&foxmorrow(root, ASSIGN, None));
    let report = r#"{"summary":"x","status":"completed","outcome":"failed"}"#;
    json(&foxmorrow(root, &["report", "-a", "a-01"], Some(report)));
    let again = foxmorrow(root, &["report", "-a", "a-01"], Some(report));
    assert_eq!(again.status.code(), Some(2));
    assert!(String::from_utf8_lossy(&again.stderr).contains("already has a report"));
}

#[test]
fn status_outside_a_run_fails() {
    let dir = tempfile::tempdir().unwrap();
    let output = foxmorrow(dir.path(), &["status"], None);
    assert_eq!(output.status.code(), Some(2));
    assert!(String::from_utf8_lossy(&output.stderr).contains("no foxmorrow run"));
}
```

- [ ] **Шаг 2.** Запустить: `cargo test -p foxmorrow --test cli`. Ожидается: тесты падают, потому что временный `main.rs` не знает подкоманд.

- [ ] **Шаг 3.** `crates/foxmorrow/src/project.rs`:

```rust
use std::path::{Path, PathBuf};

use foxmorrow_core::{Error, Result, baseline};

pub fn for_init(explicit: Option<PathBuf>, cwd: &Path) -> PathBuf {
    explicit.unwrap_or_else(|| baseline::toplevel(cwd).unwrap_or_else(|_| cwd.to_path_buf()))
}

pub fn for_existing(explicit: Option<PathBuf>, cwd: &Path) -> Result<PathBuf> {
    if let Some(path) = explicit {
        return Ok(path);
    }
    cwd.ancestors()
        .find(|dir| dir.join(".foxmorrow").join("current").is_file())
        .map(Path::to_path_buf)
        .ok_or_else(|| Error::NotFound("no foxmorrow run in this directory or its parents; start one with foxmorrow init".into()))
}
```

- [ ] **Шаг 4.** `crates/foxmorrow/src/main.rs`:

```rust
mod project;

use std::io::Read;
use std::path::PathBuf;
use std::process::ExitCode;

use clap::{Parser, Subcommand};
use foxmorrow_core::assignment::{self, NewAssignment};
use foxmorrow_core::model::Kind;
use foxmorrow_core::run::{self, NewRun};
use foxmorrow_core::{Result, baseline, criteria, receipt, report, status};
use serde::Serialize;

#[derive(Parser)]
#[command(name = "foxmorrow", version, about = "Provable multi-agent runs")]
struct Cli {
    #[arg(long, global = true)]
    project: Option<PathBuf>,
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    Init {
        #[arg(long)]
        criteria: String,
        #[arg(long = "paths", required = true, num_args = 1..)]
        paths: Vec<String>,
        #[arg(long = "repo")]
        repos: Vec<PathBuf>,
        #[arg(long)]
        no_family_rule: bool,
        #[arg(long)]
        session_id: Option<String>,
    },
    Assign {
        #[arg(long)]
        role: String,
        #[arg(long)]
        kind: String,
        #[arg(long)]
        profile: Option<String>,
        #[arg(long = "expected-model")]
        expected_models: Vec<String>,
        #[arg(long)]
        brief_sha256: Option<String>,
        #[arg(long)]
        role_sha256: Option<String>,
        #[arg(long = "paths", required = true, num_args = 1..)]
        paths: Vec<String>,
        #[arg(long = "depends-on", num_args = 1..)]
        depends_on: Vec<String>,
        #[arg(long)]
        previous: Option<String>,
        #[arg(long)]
        run: Option<String>,
    },
    Check {
        #[arg(short = 'a', long = "assignment")]
        assignment: String,
        #[arg(long)]
        run: Option<String>,
        #[arg(last = true, required = true)]
        command: Vec<String>,
    },
    Report {
        #[arg(short = 'a', long = "assignment")]
        assignment: String,
        #[arg(long)]
        run: Option<String>,
    },
    Status {
        #[arg(long)]
        run: Option<String>,
    },
}

fn main() -> ExitCode {
    match execute(Cli::parse()) {
        Ok(code) => code,
        Err(error) => {
            eprintln!("foxmorrow: error: {error}");
            ExitCode::from(2)
        }
    }
}

fn execute(cli: Cli) -> Result<ExitCode> {
    let cwd = std::env::current_dir()?;
    match cli.command {
        Command::Init { criteria: reference, paths, repos, no_family_rule, session_id } => {
            let root = project::for_init(cli.project, &cwd);
            let (file, heading) = criteria::split_reference(&reference)?;
            let markdown = std::fs::read_to_string(root.join(file))?;
            let repos = if repos.is_empty() { vec![root.clone()] } else { repos };
            let mut baselines = Vec::new();
            for repo in &repos {
                baselines.push(baseline::collect(repo)?);
            }
            let (created, _) = run::create_run(NewRun {
                project_root: root,
                criteria_path: file.to_string(),
                criteria_heading: heading.to_string(),
                criteria_markdown: markdown,
                allowed_paths: paths,
                baselines,
                root_session_id: session_id,
                family_rule: !no_family_rule,
            })?;
            print_json(&serde_json::json!({ "run": created.id, "criteria": created.criteria }))?;
        }
        Command::Assign {
            role,
            kind,
            profile,
            expected_models,
            brief_sha256,
            role_sha256,
            paths,
            depends_on,
            previous,
            run: run_id,
        } => {
            let root = project::for_existing(cli.project, &cwd)?;
            let (current, mut journal) = run::open_run(&root, run_id.as_deref())?;
            let created = assignment::create(
                &mut journal,
                &current,
                NewAssignment {
                    role,
                    kind: Kind::parse(&kind)?,
                    profile,
                    expected_models,
                    brief_sha256,
                    role_sha256,
                    allowed_paths: paths,
                    depends_on,
                    previous_id: previous,
                },
            )?;
            print_json(&created)?;
        }
        Command::Check { assignment: id, run: run_id, command } => {
            let root = project::for_existing(cli.project, &cwd)?;
            let (_, mut journal) = run::open_run(&root, run_id.as_deref())?;
            let recorded = receipt::run_check(&mut journal, &id, &cwd, &command)?;
            print!("{}", recorded.output_tail.as_deref().unwrap_or(""));
            let shown = recorded.exit_code.map_or_else(|| "none".to_string(), |code| code.to_string());
            eprintln!("foxmorrow: {} exit={shown}", recorded.id);
            let code = recorded.exit_code.and_then(|code| u8::try_from(code).ok()).unwrap_or(1);
            return Ok(ExitCode::from(code));
        }
        Command::Report { assignment: id, run: run_id } => {
            let root = project::for_existing(cli.project, &cwd)?;
            let (current, mut journal) = run::open_run(&root, run_id.as_deref())?;
            let mut input = String::new();
            std::io::stdin().read_to_string(&mut input)?;
            let body = report::submit(&mut journal, &current, &id, &input)?;
            print_json(&serde_json::json!({ "assignment": id, "status": body.status, "outcome": body.outcome }))?;
        }
        Command::Status { run: run_id } => {
            let root = project::for_existing(cli.project, &cwd)?;
            let (current, journal) = run::open_run(&root, run_id.as_deref())?;
            print_json(&status::status(journal.conn(), &current)?)?;
        }
    }
    Ok(ExitCode::SUCCESS)
}

fn print_json(value: &impl Serialize) -> Result<()> {
    println!("{}", serde_json::to_string_pretty(value)?);
    Ok(())
}
```

- [ ] **Шаг 5.** Запустить: `cargo test -p foxmorrow --test cli`. Ожидается: 3 теста пройдены.

- [ ] **Шаг 6.** Коммит:

```sh
git add crates/foxmorrow
git commit -m "Expose init, assign, check, report and status in the CLI"
```

## Задача 13. Итоговая проверка

- [ ] **Шаг 1.** Форматирование, линтер и все тесты:

```sh
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

Ожидается: все три команды завершаются с кодом 0. Если `cargo fmt --check` нашёл расхождения, выполнить `cargo fmt --all`, проверить diff и повторить. Предупреждения `clippy` исправляются в коде, без `allow`. Исключение одно: `#![allow(dead_code)]` в общем модуле тестов `tests/common/mod.rs`, потому что каждый тестовый файл использует его частично.

- [ ] **Шаг 2.** Сверить реализацию с разделом «Критерии приёмки» этого плана: для каждого пункта назвать тест или команду, которые его подтверждают.

- [ ] **Шаг 3.** Убедиться, что в коде нет поясняющих комментариев:

```sh
rg -n '^\s*//' crates --glob '*.rs'
```

Ожидается пустой вывод.

- [ ] **Шаг 4.** Коммит исправлений, если они были:

```sh
git add -A crates Cargo.lock
git commit -m "Fix formatting and lint findings"
```
