# План 003. Роли, профили, брифы

Document status: approved
Based on: [foxmorrow v1: дизайн](../design/2026-11-10-foxmorrow-v1-design.md), [дорожная карта](roadmap-v1.md), [результаты плана 000](impl-000-devin-probe.md#результаты), [план 001](impl-001-core-journal.md)

Задачи выполняются по порядку, шаги отмечаются чекбоксами `- [ ]`. Коммиты из шагов «Коммит» выполняются, только если пользователь разрешил коммиты в этом run.

**Цель:** каталог из 21 роли, профиль моделей Devin, генерация ролей Devin с маркером `foxmorrow-role-sha256`, проектные переопределения моделей, проверка skills, шаблоны брифа `en` и `ru`. Команда `assign` сама вычисляет профиль, модель, хеш роли и бриф, а журнал хранит бриф вместе с назначением.

**Архитектура:**
- **Ядро.** Каталог ролей (`catalog`) и отрисовка брифа (`brief`) не зависят от среды и живут в `foxmorrow-core`. Ядро также хранит текст брифа в новой таблице `brief` и вычисляет его хеш при создании назначения в той же транзакции.
- **`foxmorrow-devin`.** Новый крейт отвечает за то, что относится к Devin: профиль моделей и семейства, формат `agents/*.md`, синхронизацию плагина и проекта, список skills через `devin skills list --json`. План 002 добавит в этот крейт reader `sessions.db` и hooks.
- **CLI.** Связывает всё вместе.

**Стек:** тот же, что в плане 001. Новых внешних зависимостей нет.

**Вне плана:**
- CLI `verify`, `finalize`, hooks, reader `sessions.db` (план 002);
- skill `fox`, файлы `skills/fox/references/*.md`, `init --doc` и `init --plan` (план 004);
- манифест плагина, загрузчик, CI (план 005).

## Решения плана, уточняющие дизайн

| Вопрос | Решение | Основание |
|---|---|---|
| Модели по уровням | `worker` → `swe-2-high`, `analyst` → `claude-opus-5-5-high`, `verifier` → `gpt-6-1-sol-high`, `escalation` → `gpt-6-astra-xhigh`. Для `supervising-architect` отдельно задана модель `claude-opus-5-5-max` | Вывод `devin models` из плана 000. С этим выбором каждая проверяющая роль работает на семействе, отличном от любого автора: авторы на SWE и Claude, проверяющие на GPT. `supervising-architect` пишет решения, которые проверяет reviewer на GPT, поэтому сам он на Claude. Это свойство проверяется тестом |
| Семейство модели | Задаётся префиксом ID в `profiles/devin.json` (`swe-`, `claude-`, `gpt-` и другие) | В ID Devin уровень reasoning входит в суффикс, семейство определяется префиксом |
| Обязательные и необязательные skills | В `roles/roles.json` у роли два списка, `required` и `optional`. `assign` без `--allow-missing-skills` отказывает, только если не хватает `required`. Недостающие `optional` в бриф не попадают | В Agent Flow у ролей до 19 skills, многие из них установлены не у всех. Если блокировать назначение на каждом, без флага не прошло бы почти ни одно назначение. Строгая проверка остаётся для skills, без которых роль теряет смысл |
| Skills Codex | Skills с префиксом плагина Codex (`build-web-apps:…`, `github:…`, `codex-reviewer:…`, `build-ios-apps:…`, `hugging-face:…`, `game-studio:…`), а также `openai-docs`, `design-taste-frontend`, `frontend-design` и `impeccable` в каталог не переносятся | Skills плагинов Codex и `openai-docs` Devin не видит; остальные относятся к дизайну. `qa-requirement-reviewer` после переноса skills в `~/.agents/skills` доступен Devin и остаётся у `qa-verifier` |
| Расположение skills | Канонический каталог skills пользователя: `~/.agents/skills`. Каталоги других сред (`~/.claude/skills`, `~/.codex/skills`, `~/.config/devin/skills` и другие) содержат ссылки на него. Новые skills ставятся в `~/.agents/skills` | Так устроено окружение владельца с 10 ноября 2026 года. Devin читает `~/.agents/skills`, `~/.claude/skills` и `~/.config/devin/skills`; `~/.codex/skills` он не читает, поэтому skill только в Codex для foxmorrow недоступен |
| Низкоуровневые флаги `assign` | `--kind`, `--profile`, `--expected-model`, `--brief-sha256`, `--role-sha256` удаляются. Делегированное назначение: `assign --role R --goal "…" --paths …`. Назначение root: `assign --root --paths …` | Дорожная карта: план 003 вычисляет эти значения сам |
| Правило о семействах в run | Берётся из профиля плагина и `.foxmorrow/profile.json` проекта. Флаг `init --no-family-rule` удаляется | Дизайн §2: правило отключается в профиле |
| Сводка skills в run | Новая неизменяемая колонка `run.skills_summary_json` | Дизайн §7: `init` записывает сводку недостающих skills |
| Схема журнала | Таблицы и колонки добавляются без смены `CONTRACT_VERSION = "1"` | v1 ещё не выпущен, журналов у пользователей нет |
| Язык брифа по умолчанию | `en` | Для распространяемого плагина. Владелец меняет `brief_language` в своём профиле плагина |
| Ссылки в текстах ролей | На `references/doc-flow.md#document-approval`, `#routing`, `#brd`, `#prd`, `#adr`, `#plan` и `references/contract.md`. Каталог `references` бриф указывает абсолютным путём | Файлы и якоря создаёт план 004 |
| `allowed-tools` проверяющих | `read`, `grep`, `glob`, `exec`, `get_output`, `write_to_process`, `kill_shell`, `webfetch`, `web_search`, `todo_write`, `skill`, `mcp_list_servers`, `mcp_list_tools`, `mcp_call_tool`, `mcp_read_resource` | Без `edit` и `write` (дизайн §6.3). MCP нужен QA для браузерных проверок |

## Критерии приёмки

- `cargo fmt --all --check`, `cargo clippy --workspace --all-targets -- -D warnings` и `cargo test --workspace` завершаются успешно на macOS.
- `roles/roles.json` содержит ровно 21 роль из §6.1 дизайна; дизайнерских ролей и `orchestrator` нет. Каждая роль имеет `roles/<role>.md`.
- Тексты ролей перенесены из Agent Flow `882b794` по процедуре задачи 8. В них нет терминов Codex и Agent Flow из списка запрещённых. Ссылки ведут только на `references/doc-flow.md` и `references/contract.md`.
- `agents/<role>.md` сгенерированы командой `foxmorrow sync --plugin`. `foxmorrow sync --plugin --check` не находит расхождений. Каждый файл содержит `name`, `description`, `model`, `allowed-tools` для проверяющих ролей и строку `foxmorrow-role-sha256: <sha256 текста роли>`.
- Каждая роль вида `review` или `qa` работает на семействе модели, отличном от семейства любой роли вида `author` (проверяется тестом на реальном профиле).
- `assign --role R --goal … --paths …`:
  - выбирает проектный профиль `fox-R`, если модель R переопределена в `.foxmorrow/profile.json`, иначе `foxmorrow:R`;
  - отказывает, если файл роли отсутствует или устарел;
  - отказывает, если не хватает `required` skills, пока не передан `--allow-missing-skills`;
  - печатает бриф, первая строка которого `foxmorrow assignment <id> run <run> brief-sha256 <sha256 тела>`;
  - сохраняет бриф в журнале; `foxmorrow brief -a <id>` печатает его без изменений.
- Бриф `en` и `ru` даёт одинаковые команды `foxmorrow` и одинаковые ключи отчёта. Неизвестный язык и неизвестный заполнитель шаблона отклоняются.
- `init` записывает правило о семействах из профиля и сводку недостающих `required` skills.
- `foxmorrow sync` создаёт и удаляет только файлы `.devin/agents/fox-*.md` с маркером foxmorrow; другие файлы в `.devin/agents/` не трогает.
- `foxmorrow doctor` печатает источник списка skills (`devin` или `approximate`), недостающие `required` skills по ролям и устаревшие файлы ролей.
- В коде нет поясняющих комментариев.

## Структура файлов

```
Cargo.toml                                    добавить крейт foxmorrow-devin
crates/foxmorrow-core/src/schema.rs           таблица brief, колонка run.skills_summary_json
crates/foxmorrow-core/src/run.rs              поле skills_summary
crates/foxmorrow-core/src/assignment.rs       create_with_brief, brief
crates/foxmorrow-core/src/catalog.rs          каталог ролей
crates/foxmorrow-core/src/brief.rs            шаблоны и отрисовка брифа
crates/foxmorrow-core/tests/brief_storage.rs
crates/foxmorrow-core/tests/catalog.rs
crates/foxmorrow-core/tests/brief.rs
crates/foxmorrow-devin/Cargo.toml
crates/foxmorrow-devin/src/lib.rs
crates/foxmorrow-devin/src/profile.rs         профиль моделей и семейства
crates/foxmorrow-devin/src/agents.rs          генерация и синхронизация ролей Devin
crates/foxmorrow-devin/src/skills.rs          список доступных skills
crates/foxmorrow-devin/tests/common/mod.rs
crates/foxmorrow-devin/tests/profile.rs
crates/foxmorrow-devin/tests/agents.rs
crates/foxmorrow-devin/tests/skills.rs
crates/foxmorrow-devin/tests/repository.rs    проверки реального содержимого плагина
crates/foxmorrow/Cargo.toml                   зависимость от foxmorrow-devin
crates/foxmorrow/src/main.rs                  init, assign, brief, sync, doctor
crates/foxmorrow/src/project.rs               поиск корня плагина
crates/foxmorrow/tests/cli.rs                 сквозные тесты с временным плагином
roles/roles.json
roles/_common.md
roles/<role>.md                               21 файл
profiles/devin.json
templates/brief.en.md
templates/brief.ru.md
agents/<role>.md                              21 сгенерированный файл
```

## Задача 1. Хранение брифа и сводки skills в журнале

**Файлы:** изменить `schema.rs`, `run.rs`, `assignment.rs`; изменить литералы `NewRun` в тестах; создать `tests/brief_storage.rs`.

- [ ] **Шаг 1.** Тест `crates/foxmorrow-core/tests/brief_storage.rs`:

```rust
mod common;

use std::collections::BTreeMap;

use common::*;
use foxmorrow_core::Error;
use foxmorrow_core::assignment::{self, BriefDraft};
use foxmorrow_core::model::Kind;
use foxmorrow_core::run::{self, NewRun};
use foxmorrow_core::util::sha256_hex;

#[test]
fn stores_the_rendered_brief_with_its_hash() {
    let dir = tempfile::tempdir().unwrap();
    let (run, mut journal) = new_run(dir.path());
    let (created, text) = assignment::create_with_brief(
        &mut journal,
        &run,
        spec("backend-worker", Kind::Author, "swe-2-high", &[]),
        |draft| Ok(BriefDraft { language: "en".into(), body: format!("Role: {}\n", draft.role) }),
    )
    .unwrap();
    let digest = created.brief_sha256.clone().unwrap();
    assert_eq!(digest, sha256_hex(b"Role: backend-worker\n"));
    assert_eq!(
        text,
        format!("foxmorrow assignment a-01 run {} brief-sha256 {digest}\n\nRole: backend-worker\n", run.id)
    );
    assert_eq!(assignment::brief(journal.conn(), "a-01").unwrap(), Some(("en".to_string(), text)));
    assert_eq!(assignment::get(journal.conn(), "a-01").unwrap().brief_sha256, Some(digest));
    assert!(journal.conn().execute("UPDATE brief SET text = 'x'", []).is_err());
}

#[test]
fn failed_rendering_creates_nothing() {
    let dir = tempfile::tempdir().unwrap();
    let (run, mut journal) = new_run(dir.path());
    let result = assignment::create_with_brief(
        &mut journal,
        &run,
        spec("backend-worker", Kind::Author, "swe-2-high", &[]),
        |_| Err(Error::Invalid("template broken".into())),
    );
    assert!(matches!(result, Err(Error::Invalid(_))));
    assert!(assignment::list(journal.conn()).unwrap().is_empty());
}

#[test]
fn run_keeps_the_skills_summary() {
    let dir = tempfile::tempdir().unwrap();
    let summary = BTreeMap::from([("architect".to_string(), vec!["arch-simplicity-guard".to_string()])]);
    let (created, journal) = run::create_run(NewRun {
        project_root: dir.path().to_path_buf(),
        criteria_path: "task.md".into(),
        criteria_heading: "Критерии приёмки".into(),
        criteria_markdown: TASK.into(),
        allowed_paths: vec!["src/".into()],
        baselines: Vec::new(),
        root_session_id: None,
        family_rule: true,
        skills_summary: summary.clone(),
    })
    .unwrap();
    assert_eq!(created.skills_summary, summary);
    assert_eq!(run::load_run(journal.conn()).unwrap().skills_summary, summary);
    assert!(journal.conn().execute("UPDATE run SET skills_summary_json = '{}'", []).is_err());
}
```

- [ ] **Шаг 2.** Запустить: `cargo test -p foxmorrow-core --test brief_storage`. Ожидается ошибка компиляции: нет `BriefDraft`, `create_with_brief`, `skills_summary`.

- [ ] **Шаг 3.** `schema.rs`. В таблице `run` после `family_rule INTEGER NOT NULL` добавить колонку:

```sql
    family_rule INTEGER NOT NULL,
    skills_summary_json TEXT NOT NULL
```

В триггер `run_immutable` после строки `OR NEW.family_rule IS NOT OLD.family_rule` добавить:

```sql
  OR NEW.skills_summary_json IS NOT OLD.skills_summary_json
```

После таблицы `assignment` добавить:

```sql
CREATE TABLE brief (
    assignment_id TEXT PRIMARY KEY REFERENCES assignment (id),
    language TEXT NOT NULL,
    text TEXT NOT NULL
);
```

К триггерам добавить:

```sql
CREATE TRIGGER brief_no_update BEFORE UPDATE ON brief BEGIN SELECT RAISE(ABORT, 'immutable: brief'); END;
CREATE TRIGGER brief_no_delete BEFORE DELETE ON brief BEGIN SELECT RAISE(ABORT, 'immutable: brief'); END;
CREATE TRIGGER brief_after_final BEFORE INSERT ON brief WHEN EXISTS (SELECT 1 FROM final)
BEGIN SELECT RAISE(ABORT, 'run is finalized'); END;
```

- [ ] **Шаг 4.** `run.rs`. Импорт `use std::collections::BTreeMap;`. В `NewRun` и `Run` после `family_rule` добавить поле:

```rust
    pub skills_summary: BTreeMap<String, Vec<String>>,
```

В `create_run` при построении `Run` после `family_rule: new.family_rule,` добавить `skills_summary: new.skills_summary,`. Запрос `INSERT` заменить на:

```rust
        tx.execute(
            "INSERT INTO run (id, created_ms, project_root, criteria_path, criteria_heading, criteria_sha256, criteria_json, allowed_paths_json, baseline_json, root_session_id, root_model, family_rule, skills_summary_json)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13)",
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
                run.family_rule,
                serde_json::to_string(&run.skills_summary)?
            ],
        )?;
```

`load_run` заменить на:

```rust
pub fn load_run(conn: &Connection) -> Result<Run> {
    let (run, criteria_json, paths_json, skills_json) = conn.query_row(
        "SELECT id, created_ms, project_root, criteria_path, criteria_heading, criteria_sha256, criteria_json, allowed_paths_json, family_rule, root_session_id, root_model, skills_summary_json FROM run",
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
                    skills_summary: BTreeMap::new(),
                    root_session_id: row.get(9)?,
                    root_model: row.get(10)?,
                },
                row.get::<_, String>(6)?,
                row.get::<_, String>(7)?,
                row.get::<_, String>(11)?,
            ))
        },
    )?;
    Ok(Run {
        criteria: serde_json::from_str(&criteria_json)?,
        allowed_paths: serde_json::from_str(&paths_json)?,
        skills_summary: serde_json::from_str(&skills_json)?,
        ..run
    })
}
```

- [ ] **Шаг 5.** `assignment.rs`. В импорт `util` добавить `sha256_hex`: `use crate::util::{now_ms, sha256_hex};`. Функцию `create` заменить на:

```rust
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BriefDraft {
    pub language: String,
    pub body: String,
}

pub fn create(journal: &mut Journal, run: &Run, new: NewAssignment) -> Result<Assignment> {
    Ok(insert(journal, run, new, |_| Ok(None))?.0)
}

pub fn create_with_brief(
    journal: &mut Journal,
    run: &Run,
    new: NewAssignment,
    render: impl FnOnce(&Assignment) -> Result<BriefDraft>,
) -> Result<(Assignment, String)> {
    let (created, text) = insert(journal, run, new, |draft| render(draft).map(Some))?;
    let text = text.ok_or_else(|| Error::Invalid("brief was not rendered".into()))?;
    Ok((created, text))
}

pub fn brief(conn: &Connection, id: &str) -> Result<Option<(String, String)>> {
    Ok(conn
        .query_row("SELECT language, text FROM brief WHERE assignment_id = ?1", [id], |row| {
            Ok((row.get(0)?, row.get(1)?))
        })
        .optional()?)
}

fn insert(
    journal: &mut Journal,
    run: &Run,
    new: NewAssignment,
    render: impl FnOnce(&Assignment) -> Result<Option<BriefDraft>>,
) -> Result<(Assignment, Option<String>)> {
    validate_shape(run, &new)?;
    journal.write(|tx| {
        validate_links(tx, &new)?;
        let seq: i64 = tx.query_row("SELECT COALESCE(MAX(seq), 0) + 1 FROM assignment", [], |row| row.get(0))?;
        let mut created = Assignment {
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
        let brief = match render(&created)? {
            Some(draft) => {
                let digest = sha256_hex(draft.body.as_bytes());
                let text = format!(
                    "foxmorrow assignment {} run {} brief-sha256 {digest}\n\n{}",
                    created.id, run.id, draft.body
                );
                created.brief_sha256 = Some(digest);
                Some((draft.language, text))
            }
            None => None,
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
        if let Some((language, text)) = &brief {
            tx.execute(
                "INSERT INTO brief (assignment_id, language, text) VALUES (?1, ?2, ?3)",
                params![created.id, language, text],
            )?;
        }
        Ok((created, brief.map(|(_, text)| text)))
    })
}
```

- [ ] **Шаг 6.** Обновить существующие тесты. В каждом литерале `NewRun { … }` после строки `family_rule: …,` добавить `skills_summary: Default::default(),`. Файлы: `crates/foxmorrow-core/tests/common/mod.rs`, `tests/run.rs`, `tests/assignment.rs`, `tests/receipt.rs`, `tests/report.rs`; в `crates/foxmorrow/src/main.rs` то же в литерале `NewRun` (файл полностью заменяется в задаче 9). В `tests/journal.rs` в константе `SEED` строку вставки в `run` заменить на:

```rust
const SEED: &str = "INSERT INTO run VALUES ('r', 1, '/p', 'task.md', 'h', 's', '[]', '[]', '[]', NULL, NULL, 1, '{}');
INSERT INTO assignment VALUES ('a-01', 1, 1, 'backend-worker', 'author', 'p', '[]', NULL, NULL, '[]', '[]', NULL);
INSERT INTO report VALUES ('a-01', 1, 'completed', 'passed', '{}', 'x');";
```

- [ ] **Шаг 7.** Запустить: `cargo test --workspace`. Ожидается: все тесты пройдены, включая 3 новых.

- [ ] **Шаг 8.** Коммит:

```sh
git add crates
git commit -m "Store briefs and the skills summary in the journal"
```

## Задача 2. Каталог ролей

**Файлы:** создать `crates/foxmorrow-core/src/catalog.rs`, `tests/catalog.rs`; изменить `lib.rs`.

- [ ] **Шаг 1.** Тест `crates/foxmorrow-core/tests/catalog.rs`:

```rust
use std::path::Path;

use foxmorrow_core::Error;
use foxmorrow_core::catalog::{self, Catalog, Tier, ToolSet};
use foxmorrow_core::model::Kind;
use foxmorrow_core::util::sha256_hex;

fn write(root: &Path, relative: &str, text: &str) {
    let path = root.join(relative);
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(path, text).unwrap();
}

fn plugin(root: &Path, roles: &str) {
    write(
        root,
        "roles/roles.json",
        &format!(r#"{{"version": 1, "source": {{"repository": "test/agent-flow", "commit": "0000000"}}, "roles": {{{roles}}}}}"#),
    );
    write(root, "roles/_common.md", "## Common\n\nShared rules.\n");
    write(root, "roles/backend-worker.md", "# backend-worker\n\nBuild backends.\n\n");
    write(root, "roles/reviewer.md", "# reviewer\n\nReview.\n");
}

const BACKEND: &str = r#""backend-worker": {"description": "Backend", "tier": "worker", "kind": "author", "tools": "full", "skills": {"optional": ["sql-queries"]}}"#;
const REVIEWER: &str = r#""reviewer": {"description": "Reviewer", "tier": "verifier", "kind": "review", "tools": "verify", "skills": {"required": ["arch-simplicity-guard"]}}"#;

#[test]
fn loads_roles_and_joins_role_text_with_common_rules() {
    let dir = tempfile::tempdir().unwrap();
    plugin(dir.path(), &format!("{BACKEND}, {REVIEWER}"));
    let catalog = Catalog::load(dir.path()).unwrap();
    let reviewer = catalog.role("reviewer").unwrap();
    assert_eq!((reviewer.tier, reviewer.kind, reviewer.tools), (Tier::Verifier, Kind::Review, ToolSet::Verify));
    assert_eq!(reviewer.skills.required, vec!["arch-simplicity-guard".to_string()]);
    assert!(catalog.role("backend-worker").unwrap().skills.required.is_empty());
    let text = catalog::role_text(dir.path(), "backend-worker").unwrap();
    assert_eq!(text, "# backend-worker\n\nBuild backends.\n\n## Common\n\nShared rules.\n");
    assert_eq!(catalog::role_sha256(&text), sha256_hex(text.as_bytes()));
    assert!(matches!(catalog.role("designer"), Err(Error::NotFound(_))));
    assert_eq!(Tier::Escalation.as_str(), "escalation");
}

#[test]
fn verification_roles_cannot_edit() {
    let dir = tempfile::tempdir().unwrap();
    plugin(dir.path(), &REVIEWER.replace("\"verify\"", "\"full\""));
    assert!(matches!(Catalog::load(dir.path()), Err(Error::Invalid(_))));
}

#[test]
fn rejects_missing_texts_bad_names_and_unknown_fields() {
    let dir = tempfile::tempdir().unwrap();
    plugin(dir.path(), &BACKEND.replace("backend-worker", "python-worker"));
    assert!(matches!(Catalog::load(dir.path()), Err(Error::NotFound(_))));

    let dir = tempfile::tempdir().unwrap();
    plugin(dir.path(), &BACKEND.replace("backend-worker", "../escape"));
    assert!(matches!(Catalog::load(dir.path()), Err(Error::Invalid(_))));

    let dir = tempfile::tempdir().unwrap();
    plugin(dir.path(), &BACKEND.replace("\"tools\"", "\"model\": \"x\", \"tools\""));
    assert!(matches!(Catalog::load(dir.path()), Err(Error::Json(_))));

    let dir = tempfile::tempdir().unwrap();
    plugin(dir.path(), &BACKEND.replace("\"author\"", "\"root\""));
    assert!(matches!(Catalog::load(dir.path()), Err(Error::Invalid(_))));
}
```

- [ ] **Шаг 2.** Запустить: `cargo test -p foxmorrow-core --test catalog`. Ожидается ошибка компиляции.

- [ ] **Шаг 3.** `crates/foxmorrow-core/src/catalog.rs`:

```rust
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
    pub const ALL: [Tier; 4] = [Tier::Worker, Tier::Analyst, Tier::Verifier, Tier::Escalation];

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
            return invalid(format!("unsupported role catalog version {}", catalog.version));
        }
        let common = root.join(COMMON_FILE);
        if !common.is_file() {
            return Err(Error::NotFound(format!("common role text {}", common.display())));
        }
        for (name, spec) in &catalog.roles {
            if name.is_empty() || !name.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-') {
                return invalid(format!("role name '{name}' must use lowercase letters, digits and hyphens"));
            }
            if spec.kind == Kind::Root {
                return invalid(format!("role {name} cannot have kind root"));
            }
            if spec.kind.is_verification() && spec.tools != ToolSet::Verify {
                return invalid(format!("verification role {name} must use the verify tool set"));
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
```

- [ ] **Шаг 4.** Добавить `pub mod catalog;` в `lib.rs` в алфавитном порядке.

- [ ] **Шаг 5.** Запустить: `cargo test -p foxmorrow-core --test catalog`. Ожидается: 3 теста пройдены.

- [ ] **Шаг 6.** Коммит:

```sh
git add crates/foxmorrow-core
git commit -m "Load the runtime-neutral role catalog"
```

## Задача 3. Отрисовка брифа

Шаблон разбирается за один проход. Значения подставляются как есть, поэтому `{{…}}` внутри цели не считается заполнителем. Пустой список отрисовывается как `-`.

**Файлы:** создать `crates/foxmorrow-core/src/brief.rs`, `tests/brief.rs`; изменить `lib.rs`.

- [ ] **Шаг 1.** Тест `crates/foxmorrow-core/tests/brief.rs`:

```rust
mod common;

use common::*;
use foxmorrow_core::Error;
use foxmorrow_core::assignment;
use foxmorrow_core::brief::{self, BriefInput, ContextItem};
use foxmorrow_core::model::{Kind, Outcome};

const TEMPLATE: &str = "Role: {{role}} Profile: {{profile}} role-sha256 {{role_sha256}}\nModel: {{expected_models}}\nReferences: {{references_dir}}\nSkills:\n{{skills}}\nMissing:\n{{missing_skills}}\nGoal: {{goal}}\nCriteria ({{criteria_source}}):\n{{criteria}}\nPaths:\n{{allowed_paths}}\nContext:\n{{context}}\nfoxmorrow report -a {{assignment_id}}\n";

fn input_parts(dir: &std::path::Path) -> (foxmorrow_core::run::Run, foxmorrow_core::assignment::Assignment) {
    let (run, mut journal) = new_run(dir);
    let created = assignment::create(&mut journal, &run, spec("backend-worker", Kind::Author, "swe-2-high", &[])).unwrap();
    (run, created)
}

#[test]
fn fills_every_placeholder() {
    let dir = tempfile::tempdir().unwrap();
    let (run, created) = input_parts(dir.path());
    let context = vec![ContextItem {
        assignment_id: "a-00".into(),
        role: "architect".into(),
        outcome: Outcome::Passed,
        summary: "Plan agreed".into(),
    }];
    let skills = vec!["sql-queries".to_string()];
    let text = brief::render(
        TEMPLATE,
        &BriefInput {
            run: &run,
            assignment: &created,
            references_dir: "/plugin/skills/fox/references",
            skills: &skills,
            missing_skills: &[],
            goal: "  Add GET /health {{not a placeholder}}  ",
            context: &context,
        },
    )
    .unwrap();
    assert_eq!(
        text,
        "Role: backend-worker Profile: foxmorrow:backend-worker role-sha256 role-backend-worker\n\
Model: swe-2-high\n\
References: /plugin/skills/fox/references\n\
Skills:\n- sql-queries\n\
Missing:\n-\n\
Goal: Add GET /health {{not a placeholder}}\n\
Criteria (task.md#Критерии приёмки):\n- ac-1: Первый критерий\n- ac-2: Второй критерий\n\
Paths:\n- src/\n\
Context:\n- a-00 architect passed: Plan agreed\n\
foxmorrow report -a a-01\n"
    );
}

#[test]
fn rejects_broken_templates_and_empty_goals() {
    let dir = tempfile::tempdir().unwrap();
    let (run, created) = input_parts(dir.path());
    let render = |template: &str, goal: &str| {
        brief::render(
            template,
            &BriefInput {
                run: &run,
                assignment: &created,
                references_dir: "/r",
                skills: &[],
                missing_skills: &[],
                goal,
                context: &[],
            },
        )
    };
    assert!(matches!(render("{{assignment_id}} {{unknown}}", "g"), Err(Error::Invalid(_))));
    assert!(matches!(render("{{assignment_id}} {{role", "g"), Err(Error::Invalid(_))));
    assert!(matches!(render("no id here", "g"), Err(Error::Invalid(_))));
    assert!(matches!(render("{{assignment_id}}", "  "), Err(Error::Invalid(_))));
}

#[test]
fn loads_templates_by_language() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(dir.path().join("templates")).unwrap();
    std::fs::write(dir.path().join("templates/brief.en.md"), "EN {{assignment_id}}").unwrap();
    std::fs::write(dir.path().join("templates/brief.ru.md"), "RU {{assignment_id}}").unwrap();
    assert_eq!(brief::load_template(dir.path(), "ru").unwrap(), "RU {{assignment_id}}");
    let error = brief::load_template(dir.path(), "de").unwrap_err();
    assert!(error.to_string().contains("available: en, ru"), "{error}");
    assert!(matches!(brief::load_template(dir.path(), "../en"), Err(Error::Invalid(_))));
}
```

- [ ] **Шаг 2.** Запустить: `cargo test -p foxmorrow-core --test brief`. Ожидается ошибка компиляции.

- [ ] **Шаг 3.** `crates/foxmorrow-core/src/brief.rs`:

```rust
use std::path::Path;

use crate::assignment::Assignment;
use crate::error::{Result, invalid};
use crate::model::Outcome;
use crate::run::Run;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ContextItem {
    pub assignment_id: String,
    pub role: String,
    pub outcome: Outcome,
    pub summary: String,
}

pub struct BriefInput<'a> {
    pub run: &'a Run,
    pub assignment: &'a Assignment,
    pub references_dir: &'a str,
    pub skills: &'a [String],
    pub missing_skills: &'a [String],
    pub goal: &'a str,
    pub context: &'a [ContextItem],
}

pub fn render(template: &str, input: &BriefInput<'_>) -> Result<String> {
    if !template.contains("{{assignment_id}}") {
        return invalid("brief template must contain {{assignment_id}}");
    }
    let goal = input.goal.trim();
    if goal.is_empty() {
        return invalid("brief goal must not be empty");
    }
    let assignment = input.assignment;
    let values: Vec<(&str, String)> = vec![
        ("assignment_id", assignment.id.clone()),
        ("role", assignment.role.clone()),
        ("profile", assignment.profile.clone().unwrap_or_default()),
        ("role_sha256", assignment.role_sha256.clone().unwrap_or_default()),
        ("expected_models", assignment.expected_models.join(", ")),
        ("references_dir", input.references_dir.to_string()),
        ("skills", list(input.skills.iter().cloned())),
        ("missing_skills", list(input.missing_skills.iter().cloned())),
        ("goal", goal.to_string()),
        ("criteria_source", format!("{}#{}", input.run.criteria_path, input.run.criteria_heading)),
        (
            "criteria",
            list(input.run.criteria.iter().map(|criterion| format!("{}: {}", criterion.id, criterion.text))),
        ),
        ("allowed_paths", list(assignment.allowed_paths.iter().cloned())),
        (
            "context",
            list(input.context.iter().map(|item| {
                format!("{} {} {}: {}", item.assignment_id, item.role, item.outcome.as_str(), item.summary)
            })),
        ),
    ];
    let mut output = String::new();
    let mut rest = template;
    while let Some(start) = rest.find("{{") {
        output.push_str(&rest[..start]);
        let after = &rest[start + 2..];
        let Some(end) = after.find("}}") else {
            return invalid("unclosed brief placeholder");
        };
        let name = &after[..end];
        let Some((_, value)) = values.iter().find(|(known, _)| *known == name) else {
            return invalid(format!("unknown brief placeholder {{{{{name}}}}}"));
        };
        output.push_str(value);
        rest = &after[end + 2..];
    }
    output.push_str(rest);
    Ok(output)
}

pub fn load_template(plugin_root: &Path, language: &str) -> Result<String> {
    if language.is_empty() || !language.chars().all(|c| c.is_ascii_alphanumeric() || c == '-') {
        return invalid(format!("brief language '{language}' must use letters, digits and hyphens"));
    }
    let templates = plugin_root.join("templates");
    if let Ok(text) = std::fs::read_to_string(templates.join(format!("brief.{language}.md"))) {
        return Ok(text);
    }
    let mut available: Vec<String> = std::fs::read_dir(&templates)
        .into_iter()
        .flatten()
        .flatten()
        .filter_map(|entry| entry.file_name().into_string().ok())
        .filter_map(|name| {
            name.strip_prefix("brief.")
                .and_then(|rest| rest.strip_suffix(".md"))
                .map(str::to_string)
        })
        .collect();
    available.sort();
    invalid(format!("no brief template for language '{language}'; available: {}", available.join(", ")))
}

fn list(items: impl Iterator<Item = String>) -> String {
    let lines: Vec<String> = items.map(|item| format!("- {item}")).collect();
    if lines.is_empty() { "-".to_string() } else { lines.join("\n") }
}
```

- [ ] **Шаг 4.** Добавить `pub mod brief;` в `lib.rs`.

- [ ] **Шаг 5.** Запустить: `cargo test -p foxmorrow-core --test brief`. Ожидается: 3 теста пройдены.

- [ ] **Шаг 6.** Коммит:

```sh
git add crates/foxmorrow-core
git commit -m "Render briefs from language templates"
```

## Задача 4. Крейт `foxmorrow-devin` и профиль моделей

**Файлы:** изменить корневой `Cargo.toml`; создать `crates/foxmorrow-devin/Cargo.toml`, `src/lib.rs`, `src/profile.rs`, `tests/common/mod.rs`, `tests/profile.rs`.

- [ ] **Шаг 1.** В корневом `Cargo.toml` в `members` добавить `"crates/foxmorrow-devin"`, в `[workspace.dependencies]` строку `foxmorrow-devin = { path = "crates/foxmorrow-devin" }`.

- [ ] **Шаг 2.** `crates/foxmorrow-devin/Cargo.toml`:

```toml
[package]
name = "foxmorrow-devin"
version.workspace = true
edition.workspace = true
license.workspace = true
rust-version.workspace = true

[dependencies]
foxmorrow-core.workspace = true
serde.workspace = true
serde_json.workspace = true

[dev-dependencies]
tempfile.workspace = true
```

- [ ] **Шаг 3.** `crates/foxmorrow-devin/src/lib.rs`:

```rust
pub mod agents;
pub mod profile;
pub mod skills;
```

До задач 5 и 6 временно оставить в `lib.rs` только `pub mod profile;`.

- [ ] **Шаг 4.** Помощники тестов `crates/foxmorrow-devin/tests/common/mod.rs`:

```rust
#![allow(dead_code)]

use std::path::Path;

pub const CATALOG: &str = r#"{
  "version": 1,
  "source": {"repository": "test/agent-flow", "commit": "0000000"},
  "roles": {
    "backend-worker": {"description": "Backend \"worker\"", "tier": "worker", "kind": "author", "tools": "full", "skills": {"optional": ["sql-queries"]}},
    "reviewer": {"description": "Reviewer", "tier": "verifier", "kind": "review", "tools": "verify", "skills": {"required": ["arch-simplicity-guard"]}}
  }
}"#;

pub const PROFILE: &str = r#"{
  "version": 1,
  "brief_language": "en",
  "family_rule": true,
  "tiers": {"worker": "swe-2-high", "analyst": "claude-opus-5-5-high", "verifier": "gpt-6-1-sol-high", "escalation": "gpt-6-astra-xhigh"},
  "roles": {"reviewer": "gpt-6-astra-high"},
  "families": [
    {"family": "swe", "prefixes": ["swe-"]},
    {"family": "claude", "prefixes": ["claude-"]},
    {"family": "gpt", "prefixes": ["gpt-"]}
  ]
}"#;

pub fn write(root: &Path, relative: &str, text: &str) {
    let path = root.join(relative);
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(path, text).unwrap();
}

pub fn plugin(root: &Path) {
    write(root, "roles/roles.json", CATALOG);
    write(root, "roles/_common.md", "## Common\n\nShared rules.\n");
    write(root, "roles/backend-worker.md", "# backend-worker\n\nBuild backends.\n");
    write(root, "roles/reviewer.md", "# reviewer\n\nReview.\n");
    write(root, "profiles/devin.json", PROFILE);
}
```

- [ ] **Шаг 5.** Тест `crates/foxmorrow-devin/tests/profile.rs`:

```rust
mod common;

use common::*;
use foxmorrow_core::Error;
use foxmorrow_core::catalog::Tier;
use foxmorrow_devin::profile::{ModelChoice, Profile, ProjectProfile};

#[test]
fn resolves_models_with_project_overrides_first() {
    let dir = tempfile::tempdir().unwrap();
    plugin(dir.path());
    let profile = Profile::load(dir.path()).unwrap();
    let none = ProjectProfile::default();
    assert_eq!(
        profile.model_for("backend-worker", Tier::Worker, &none),
        ModelChoice { model: "swe-2-high".into(), project_override: false }
    );
    assert_eq!(profile.model_for("reviewer", Tier::Verifier, &none).model, "gpt-6-astra-high");

    write(dir.path(), ".foxmorrow/profile.json", r#"{"tiers": {"worker": "claude-sonnet-5-5-high"}, "roles": {"reviewer": "gpt-6-1-sol-low"}, "family_rule": false}"#);
    let project = ProjectProfile::load(dir.path()).unwrap();
    assert_eq!(
        profile.model_for("backend-worker", Tier::Worker, &project),
        ModelChoice { model: "claude-sonnet-5-5-high".into(), project_override: true }
    );
    assert_eq!(profile.model_for("reviewer", Tier::Verifier, &project).model, "gpt-6-1-sol-low");
    assert!(profile.family_rule(&none));
    assert!(!profile.family_rule(&project));
}

#[test]
fn maps_models_to_families_by_prefix() {
    let dir = tempfile::tempdir().unwrap();
    plugin(dir.path());
    let profile = Profile::load(dir.path()).unwrap();
    assert_eq!(profile.family_of("claude-haiku-5-5-low"), Some("claude"));
    assert_eq!(profile.family_of("kimi-k3"), None);
    let models = vec!["swe-2-medium".to_string(), "kimi-k3".to_string()];
    let families = profile.families_for(&models);
    assert_eq!(families.get("swe-2-medium").map(String::as_str), Some("swe"));
    assert!(!families.contains_key("kimi-k3"));
}

#[test]
fn rejects_incomplete_profiles() {
    let dir = tempfile::tempdir().unwrap();
    plugin(dir.path());
    write(dir.path(), "profiles/devin.json", &PROFILE.replace(r#""escalation": "gpt-6-astra-xhigh""#, r#""escalation_x": "gpt-6-astra-xhigh""#));
    assert!(Profile::load(dir.path()).is_err());

    write(dir.path(), "profiles/devin.json", &PROFILE.replace(r#", "escalation": "gpt-6-astra-xhigh""#, ""));
    assert!(matches!(Profile::load(dir.path()), Err(Error::Invalid(_))));

    write(dir.path(), "profiles/devin.json", &PROFILE.replace("gpt-6-astra-high", "kimi-k3"));
    assert!(matches!(Profile::load(dir.path()), Err(Error::Invalid(_))));

    assert_eq!(ProjectProfile::load(tempfile::tempdir().unwrap().path()).unwrap(), ProjectProfile::default());
}
```

- [ ] **Шаг 6.** Запустить: `cargo test -p foxmorrow-devin --test profile`. Ожидается ошибка компиляции.

- [ ] **Шаг 7.** `crates/foxmorrow-devin/src/profile.rs`:

```rust
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
            return invalid(format!("unsupported model profile version {}", profile.version));
        }
        if let Some(tier) = Tier::ALL.iter().find(|tier| !profile.tiers.contains_key(tier)) {
            return invalid(format!("model profile has no model for tier {}", tier.as_str()));
        }
        if let Some(model) = profile.tiers.values().chain(profile.roles.values()).find(|model| profile.family_of(model).is_none()) {
            return invalid(format!("model {model} has no family in the profile"));
        }
        Ok(profile)
    }

    pub fn model_for(&self, role: &str, tier: Tier, project: &ProjectProfile) -> ModelChoice {
        if let Some(model) = project.roles.get(role).or_else(|| project.tiers.get(&tier)) {
            return ModelChoice { model: model.clone(), project_override: true };
        }
        let model = self.roles.get(role).or_else(|| self.tiers.get(&tier)).cloned().unwrap_or_default();
        ModelChoice { model, project_override: false }
    }

    pub fn family_rule(&self, project: &ProjectProfile) -> bool {
        project.family_rule.unwrap_or(self.family_rule)
    }

    pub fn family_of(&self, model: &str) -> Option<&str> {
        self.families
            .iter()
            .find(|rule| rule.prefixes.iter().any(|prefix| model.starts_with(prefix.as_str())))
            .map(|rule| rule.family.as_str())
    }

    pub fn families_for<'a>(&self, models: impl IntoIterator<Item = &'a String>) -> ModelFamilies {
        models
            .into_iter()
            .filter_map(|model| self.family_of(model).map(|family| (model.clone(), family.to_string())))
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
```

- [ ] **Шаг 8.** Запустить: `cargo test -p foxmorrow-devin --test profile`. Ожидается: 3 теста пройдены.

- [ ] **Шаг 9.** Коммит:

```sh
git add Cargo.toml Cargo.lock crates/foxmorrow-devin
git commit -m "Add the Devin model profile with model families"
```

## Задача 5. Генерация и синхронизация ролей Devin

Файл роли управляется foxmorrow, если в нём есть строка с маркером `foxmorrow-role-sha256: `. Синхронизация создаёт, перезаписывает и удаляет только такие файлы. В проекте она дополнительно ограничена префиксом `fox-`.

**Файлы:** создать `crates/foxmorrow-devin/src/agents.rs`, `tests/agents.rs`; в `lib.rs` добавить `pub mod agents;`.

- [ ] **Шаг 1.** Тест `crates/foxmorrow-devin/tests/agents.rs`:

```rust
mod common;

use common::*;
use foxmorrow_core::Error;
use foxmorrow_core::catalog::{self, Catalog};
use foxmorrow_devin::agents::{self, MARKER, PROJECT_PREFIX, VERIFY_TOOLS};
use foxmorrow_devin::profile::{Profile, ProjectProfile};

#[test]
fn renders_devin_agent_files() {
    let dir = tempfile::tempdir().unwrap();
    plugin(dir.path());
    let catalog = Catalog::load(dir.path()).unwrap();
    let text = catalog::role_text(dir.path(), "reviewer").unwrap();
    let (content, sha) = agents::render("reviewer", catalog.role("reviewer").unwrap(), "gpt-6-1-sol-high", &text).unwrap();
    let tools: String = VERIFY_TOOLS.iter().map(|tool| format!("  - {tool}\n")).collect();
    assert_eq!(
        content,
        format!("---\nname: reviewer\ndescription: \"Reviewer\"\nmodel: gpt-6-1-sol-high\nallowed-tools:\n{tools}---\n\n{MARKER}{sha}\n\n{text}")
    );
    assert_eq!(sha, catalog::role_sha256(&text));

    let backend = catalog::role_text(dir.path(), "backend-worker").unwrap();
    let (content, _) = agents::render("backend-worker", catalog.role("backend-worker").unwrap(), "swe-2-high", &backend).unwrap();
    assert!(content.contains("description: \"Backend \\\"worker\\\"\"\n"));
    assert!(!content.contains("allowed-tools"));
}

#[test]
fn chooses_plugin_or_project_definitions() {
    let dir = tempfile::tempdir().unwrap();
    plugin(dir.path());
    let project = tempfile::tempdir().unwrap();
    let catalog = Catalog::load(dir.path()).unwrap();
    let profile = Profile::load(dir.path()).unwrap();

    let plugin_definition = agents::definition(dir.path(), project.path(), &catalog, &profile, &ProjectProfile::default(), "backend-worker").unwrap();
    assert_eq!(plugin_definition.profile, "foxmorrow:backend-worker");
    assert_eq!(plugin_definition.path, dir.path().join("agents/backend-worker.md"));
    assert_eq!(plugin_definition.model, "swe-2-high");

    write(project.path(), ".foxmorrow/profile.json", r#"{"roles": {"backend-worker": "claude-sonnet-5-5-high"}}"#);
    let overrides = ProjectProfile::load(project.path()).unwrap();
    let project_definition = agents::definition(dir.path(), project.path(), &catalog, &profile, &overrides, "backend-worker").unwrap();
    assert_eq!(project_definition.profile, format!("{PROJECT_PREFIX}backend-worker"));
    assert_eq!(project_definition.path, project.path().join(".devin/agents/fox-backend-worker.md"));
    assert!(project_definition.content.contains("name: fox-backend-worker\n"));
    assert_eq!(project_definition.role_sha256, plugin_definition.role_sha256);

    write(project.path(), ".foxmorrow/profile.json", r#"{"roles": {"backend-worker": "kimi-k3"}}"#);
    let unknown = ProjectProfile::load(project.path()).unwrap();
    assert!(matches!(
        agents::definition(dir.path(), project.path(), &catalog, &profile, &unknown, "backend-worker"),
        Err(Error::Invalid(_))
    ));
}

#[test]
fn syncs_only_managed_files() {
    let dir = tempfile::tempdir().unwrap();
    plugin(dir.path());
    let catalog = Catalog::load(dir.path()).unwrap();
    let profile = Profile::load(dir.path()).unwrap();
    let definitions = agents::plugin_definitions(dir.path(), &catalog, &profile).unwrap();
    let agents_dir = dir.path().join("agents");

    let check = agents::sync(&definitions, &agents_dir, None, true).unwrap();
    assert_eq!(check.stale.len(), 2);
    assert!(check.written.is_empty());
    assert!(matches!(agents::ensure_current(&definitions[0]), Err(Error::Conflict(_))));

    let written = agents::sync(&definitions, &agents_dir, None, false).unwrap();
    assert_eq!(written.written.len(), 2);
    assert!(agents::ensure_current(&definitions[0]).is_ok());
    assert!(agents::sync(&definitions, &agents_dir, None, true).unwrap().stale.is_empty());

    write(dir.path(), "agents/old-role.md", &format!("---\nname: old-role\n---\n\n{MARKER}abc\n"));
    write(dir.path(), "agents/handwritten.md", "---\nname: handwritten\n---\n");
    let cleaned = agents::sync(&definitions, &agents_dir, None, false).unwrap();
    assert_eq!(cleaned.removed, vec![agents_dir.join("old-role.md")]);
    assert!(agents_dir.join("handwritten.md").is_file());

    let project = tempfile::tempdir().unwrap();
    write(project.path(), ".devin/agents/fox-gone.md", &format!("{MARKER}abc\n"));
    write(project.path(), ".devin/agents/team-role.md", &format!("{MARKER}abc\n"));
    let project_dir = project.path().join(".devin/agents");
    let none = agents::project_definitions(dir.path(), project.path(), &catalog, &profile, &ProjectProfile::default()).unwrap();
    assert!(none.is_empty());
    let report = agents::sync(&none, &project_dir, Some(PROJECT_PREFIX), false).unwrap();
    assert_eq!(report.removed, vec![project_dir.join("fox-gone.md")]);
    assert!(project_dir.join("team-role.md").is_file());
}
```

- [ ] **Шаг 2.** Запустить: `cargo test -p foxmorrow-devin --test agents`. Ожидается ошибка компиляции.

- [ ] **Шаг 3.** `crates/foxmorrow-devin/src/agents.rs`:

```rust
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

pub fn render(name: &str, spec: &RoleSpec, model: &str, role_text: &str) -> Result<(String, String)> {
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
        return invalid(format!("model {} for role {role} has no family in the profile", choice.model));
    }
    let text = catalog::role_text(plugin_root, role)?;
    let (name, profile_name, path) = if choice.project_override {
        let name = format!("{PROJECT_PREFIX}{role}");
        let path = project_root.join(".devin").join("agents").join(format!("{name}.md"));
        (name.clone(), name, path)
    } else {
        let path = plugin_root.join("agents").join(format!("{role}.md"));
        (role.to_string(), format!("{PLUGIN_NAME}:{role}"), path)
    };
    let (content, role_sha256) = render(&name, spec, &choice.model, &text)?;
    Ok(AgentDefinition { role: role.to_string(), profile: profile_name, path, model: choice.model, role_sha256, content })
}

pub fn plugin_definitions(plugin_root: &Path, catalog: &Catalog, profile: &Profile) -> Result<Vec<AgentDefinition>> {
    catalog
        .roles
        .keys()
        .map(|role| definition(plugin_root, plugin_root, catalog, profile, &ProjectProfile::default(), role))
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
    let command = if definition.profile.starts_with(PROJECT_PREFIX) { "foxmorrow sync" } else { "foxmorrow sync --plugin" };
    Err(Error::Conflict(format!("{} is missing or stale; run {command}", definition.path.display())))
}

pub fn sync(definitions: &[AgentDefinition], dir: &Path, prefix: Option<&str>, check: bool) -> Result<SyncReport> {
    let mut report = SyncReport::default();
    for definition in definitions {
        if std::fs::read_to_string(&definition.path).ok().as_deref() == Some(definition.content.as_str()) {
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
    let wanted: BTreeSet<&Path> = definitions.iter().map(|definition| definition.path.as_path()).collect();
    let mut entries: Vec<PathBuf> = std::fs::read_dir(dir)
        .into_iter()
        .flatten()
        .flatten()
        .map(|entry| entry.path())
        .collect();
    entries.sort();
    for path in entries {
        let name = path.file_name().map(|name| name.to_string_lossy().into_owned()).unwrap_or_default();
        if !name.ends_with(".md") || wanted.contains(path.as_path()) || prefix.is_some_and(|prefix| !name.starts_with(prefix)) {
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
```

- [ ] **Шаг 4.** Запустить: `cargo test -p foxmorrow-devin --test agents`. Ожидается: 3 теста пройдены.

- [ ] **Шаг 5.** Коммит:

```sh
git add crates/foxmorrow-devin
git commit -m "Generate and sync Devin role files"
```

## Задача 6. Список доступных skills

План 000 показал: `devin skills list --json` возвращает массив объектов с `name` и `errors`, а переменные Devin Desktop мешают CLI найти учётные данные. Поэтому при вызове из foxmorrow они удаляются из окружения дочернего процесса. Если Devin недоступен, список собирается по известным каталогам skills, и источник помечается `approximate`.

**Файлы:** создать `crates/foxmorrow-devin/src/skills.rs`, `tests/skills.rs`; в `lib.rs` добавить `pub mod skills;`.

- [ ] **Шаг 1.** Тест `crates/foxmorrow-devin/tests/skills.rs`:

```rust
mod common;

use common::*;
use foxmorrow_devin::skills::{self, InventorySource};

#[test]
fn parses_devin_output_and_skips_broken_skills() {
    let names = skills::parse_devin_list(
        r#"[{"name": "astro", "errors": []}, {"name": "broken", "errors": ["bad frontmatter"]}, {"name": "bun"}]"#,
    )
    .unwrap();
    assert_eq!(names.into_iter().collect::<Vec<_>>(), vec!["astro".to_string(), "bun".to_string()]);
    assert!(skills::parse_devin_list("not json").is_none());
}

#[test]
fn falls_back_to_skill_directories() {
    let project = tempfile::tempdir().unwrap();
    let home = tempfile::tempdir().unwrap();
    write(project.path(), ".devin/skills/local-skill/SKILL.md", "x");
    write(home.path(), ".agents/skills/global-skill/SKILL.md", "x");
    write(home.path(), ".agents/skills/not-a-skill/README.md", "x");
    let found = skills::inventory(project.path(), std::path::Path::new("/nonexistent/devin"), home.path());
    assert_eq!(found.source, InventorySource::Approximate);
    assert!(found.names.contains("local-skill") && found.names.contains("global-skill"));
    assert!(!found.names.contains("not-a-skill"));
    assert_eq!(found.missing(&["global-skill".into(), "astro".into()]), vec!["astro".to_string()]);
}

#[cfg(unix)]
#[test]
fn asks_devin_when_available() {
    use std::os::unix::fs::PermissionsExt;

    let bin = tempfile::tempdir().unwrap();
    let script = bin.path().join("devin");
    std::fs::write(&script, "#!/bin/sh\n[ \"$1 $2 $3\" = \"skills list --json\" ] || exit 1\necho '[{\"name\": \"astro\", \"errors\": []}]'\n").unwrap();
    std::fs::set_permissions(&script, std::fs::Permissions::from_mode(0o755)).unwrap();
    let project = tempfile::tempdir().unwrap();
    let found = skills::inventory(project.path(), &script, project.path());
    assert_eq!(found.source, InventorySource::Devin);
    assert_eq!(found.names.into_iter().collect::<Vec<_>>(), vec!["astro".to_string()]);
}
```

- [ ] **Шаг 2.** Запустить: `cargo test -p foxmorrow-devin --test skills`. Ожидается ошибка компиляции.

- [ ] **Шаг 3.** `crates/foxmorrow-devin/src/skills.rs`:

```rust
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::process::Command;

use serde::{Deserialize, Serialize};

const DESKTOP_VARIABLES: [&str; 4] = ["WINDSURF_IDE_TYPE", "WINDSURF_EXT_HOST_PID", "DEVIN_FD_PATH", "WINDSURF_CODEIUM_DIR"];

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
        wanted.iter().filter(|name| !self.names.contains(*name)).cloned().collect()
    }
}

pub fn parse_devin_list(json: &str) -> Option<BTreeSet<String>> {
    let listed: Vec<Listed> = serde_json::from_str(json).ok()?;
    Some(listed.into_iter().filter(|skill| skill.errors.is_empty()).map(|skill| skill.name).collect())
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
    command.args(["skills", "list", "--json"]).current_dir(project_root);
    for variable in DESKTOP_VARIABLES {
        command.env_remove(variable);
    }
    let listed = command
        .output()
        .ok()
        .filter(|output| output.status.success())
        .and_then(|output| parse_devin_list(&String::from_utf8_lossy(&output.stdout)));
    match listed {
        Some(names) => Inventory { source: InventorySource::Devin, names },
        None => Inventory { source: InventorySource::Approximate, names: scan(&default_dirs(project_root, home)) },
    }
}
```

- [ ] **Шаг 4.** Запустить: `cargo test -p foxmorrow-devin`. Ожидается: проходят тесты `profile`, `agents`, `skills`.

- [ ] **Шаг 5.** Коммит:

```sh
git add crates/foxmorrow-devin
git commit -m "List available skills through Devin with a directory fallback"
```

## Задача 7. Каталог, общие правила ролей, профиль и шаблоны

**Файлы:** создать `roles/roles.json`, `roles/_common.md`, `profiles/devin.json`, `templates/brief.en.md`, `templates/brief.ru.md`.

- [ ] **Шаг 1.** `roles/roles.json`. Описания ролей дословно взяты из frontmatter Agent Flow `882b794`. Списки skills составлены по правилам из раздела «Решения плана»:

```json
{
  "version": 1,
  "source": { "repository": "svishniakov/agent-flow", "commit": "882b794" },
  "roles": {
    "product-manager": {
      "description": "Product subagent owning PRD creation and substantive revision, coherent product scope, user outcomes, constraints, and acceptance criteria.",
      "tier": "analyst", "kind": "author", "tools": "full",
      "skills": { "required": ["product-manager"], "optional": ["create-prd", "one-pager-prd", "humanizer-ru", "product-management", "job-stories", "opportunity-solution-tree", "prioritization-frameworks", "product-strategy", "product-vision", "user-stories", "value-proposition", "prioritize-assumptions", "outcome-roadmap", "north-star-metric"] }
    },
    "architect": {
      "description": "Architecture subagent owning complete ADR and Implementation Plan creation and revision, technical decisions, boundaries, contracts, risks, and verification criteria.",
      "tier": "analyst", "kind": "author", "tools": "full",
      "skills": { "required": ["arch-simplicity-guard"], "optional": ["chief-architect", "golang-architect", "elixir-essentials", "otp-essentials", "ecto-essentials", "phoenix-liveview-essentials", "astro-best-practices", "system-design-doc", "pre-mortem", "sql-queries", "queue-job-processor", "kafka-development", "kafka-producer-consumer", "improve-codebase-architecture", "architecture-decision-records", "humanizer-ru"] }
    },
    "supervising-architect": {
      "description": "Senior architecture subagent resolving disputed ADR and Implementation Plan decisions, recording the chosen approach, and reassessing unresolved architecture problems.",
      "tier": "escalation", "kind": "author", "tools": "full",
      "skills": { "required": ["arch-simplicity-guard"], "optional": ["chief-architect", "ai-agents-architect", "improve-codebase-architecture", "architecture-decision-records"] }
    },
    "researcher": {
      "description": "Research subagent for documentation, APIs, SDKs, external sources, local examples, constraints, comparisons, and source-backed findings.",
      "tier": "worker", "kind": "author", "tools": "verify",
      "skills": { "required": [], "optional": ["ajtbd-research", "competitor-analysis", "browser-use", "browser-debugging", "market-sizing", "seo-audit"] }
    },
    "documenter": {
      "description": "Documentation subagent for technical specifications, task records, review sections, lessons, release notes, README work, and project documentation outside PRD, ADR and Implementation Plan authorship.",
      "tier": "analyst", "kind": "author", "tools": "full",
      "skills": { "required": [], "optional": ["markdown-documentation", "copy-editing", "grammar-check", "technical-writer", "readme-standards", "release-notes", "humanizer-ru", "software-localisation", "system-design-doc"] }
    },
    "ai-slops-hunter": {
      "description": "AI slop cleanup subagent for text, code, UI, docs, tests, and generated artifacts. Bans hyphens and dashes only in user facing prose and preserves technical syntax.",
      "tier": "worker", "kind": "author", "tools": "full",
      "skills": { "required": ["ai-slops-hunter"], "optional": ["humanize-ts", "english-humanizer", "humanize-text", "copy-editing", "grammar-check", "code-review-excellence", "accessibility", "agent-governance", "ai-agents-architect"] }
    },
    "marketing-growth-strategist": {
      "description": "Marketing and growth strategy subagent for business ideas, GTM, positioning, launch, campaigns, SEO, paid ads, growth loops, sales enablement, and promotion strategy.",
      "tier": "analyst", "kind": "author", "tools": "full",
      "skills": { "required": [], "optional": ["gtm-strategy", "gtm-motions", "launch-strategy", "business-model", "growth-loops", "content-strategy", "email-sequence", "cold-email", "ai-seo", "brand-identity", "brand-guidelines", "ideal-customer-profile", "competitive-battlecard", "cohort-analysis", "ab-test-analysis", "customer-journey-map", "kpi-dashboard-design"] }
    },
    "rag-retrieval-engineer": {
      "description": "Retrieval-first LLM/RAG engineer for semantic search, chunking, embeddings, reranking, vector stores, graph databases, GraphRAG, and retrieval quality evaluation.",
      "tier": "analyst", "kind": "author", "tools": "full",
      "skills": { "required": [], "optional": ["rag-implementation", "rag-retrieval", "evaluate-rag", "chunking-strategy", "embedding-strategies", "hybrid-search-implementation", "aliyun-qwen-rerank", "knowledge-graph-builder", "knowledge-graph", "graphrag-patterns", "sql-queries", "system-design-doc", "test-scenarios"] }
    },
    "astro-worker": {
      "description": "Astro implementation subagent for scoped HTML pages, layouts, components, styling, routing, content collections, assets, selective hydration, and build checks from an approved plan and applicable design.",
      "tier": "worker", "kind": "author", "tools": "full",
      "skills": { "required": ["astro", "astro-best-practices"], "optional": ["application-quality-assurance", "test-scenarios"] }
    },
    "backend-worker": {
      "description": "Backend execution subagent for scoped server, API, database, queue, integration, auth, or service changes from an approved plan.",
      "tier": "worker", "kind": "author", "tools": "full",
      "skills": { "required": [], "optional": ["bullmq-specialist", "sql-queries", "queue-job-processor", "rag-implementation", "application-quality-assurance", "test-scenarios"] }
    },
    "bun-worker": {
      "description": "Bun runtime subagent for Bun-based JavaScript/TypeScript projects, scripts, tests, dev servers, package management, lockfiles, and Node/npm-to-Bun migration.",
      "tier": "worker", "kind": "author", "tools": "full",
      "skills": { "required": [], "optional": ["bun", "bun-dev", "test-scenarios", "application-quality-assurance"] }
    },
    "devops-worker": {
      "description": "DevOps implementation subagent for scoped CI/CD pipelines, web application builds and deployments, environment configuration, release automation, rollback, and deployment verification from an approved plan.",
      "tier": "worker", "kind": "author", "tools": "full",
      "skills": { "required": [], "optional": ["github-actions-templates", "deployment-pipeline-design", "gha-security-review", "docker-expert", "application-quality-assurance", "test-scenarios"] }
    },
    "elixir-phoenix-worker": {
      "description": "Elixir and Phoenix implementation subagent for scoped domain logic, APIs, LiveView, Ecto persistence and migrations, OTP processes, Mix tooling, and tests from an approved plan.",
      "tier": "worker", "kind": "author", "tools": "full",
      "skills": { "required": ["elixir-essentials"], "optional": ["otp-essentials", "ecto-essentials", "phoenix-liveview-essentials", "application-quality-assurance", "test-scenarios"] }
    },
    "frontend-worker": {
      "description": "Frontend execution subagent for scoped UI, React, styling, responsive behavior, and client-state changes from an approved plan.",
      "tier": "worker", "kind": "author", "tools": "full",
      "skills": { "required": [], "optional": ["frontend-responsive-ui", "frontend-engineer", "frontend-ui-ux-engineer", "webapp-testing"] }
    },
    "golang-worker": {
      "description": "Go implementation subagent for scoped Go services, Kafka producers/consumers, CLIs, packages, concurrency, tests, modules, and idiomatic refactors from an approved plan.",
      "tier": "worker", "kind": "author", "tools": "full",
      "skills": { "required": [], "optional": ["application-quality-assurance", "golang-code-style", "golang-lint", "golang-modernize", "kafka-development", "kafka-producer-consumer", "test-scenarios"] }
    },
    "ios-worker": {
      "description": "iOS execution subagent for scoped SwiftUI, App Intents, simulator, HIG, and Apple-platform implementation tasks from an approved plan.",
      "tier": "worker", "kind": "author", "tools": "full",
      "skills": { "required": [], "optional": ["apple-hig-designer", "apple-ui-designer"] }
    },
    "python-worker": {
      "description": "Python implementation subagent for scoped backend, CLI, automation, data processing, QA scripts, tests, PDF/RAG utilities, and dependency hygiene from an approved plan.",
      "tier": "worker", "kind": "author", "tools": "full",
      "skills": { "required": [], "optional": ["application-quality-assurance", "pdf-extraction", "pdf-ocr-skill", "pdf-generator", "rag-implementation", "chunking-strategy", "browser-use", "test-scenarios", "dummy-dataset", "sql-queries", "python-packaging", "python-testing-patterns"] }
    },
    "typescript-worker": {
      "description": "TypeScript/JavaScript implementation subagent for typed app code, Node/Bun/React modules, API clients, contracts, tests, and scoped refactors from an approved plan.",
      "tier": "worker", "kind": "author", "tools": "full",
      "skills": { "required": [], "optional": ["react-components", "react-patterns", "bullmq-specialist", "playwright-e2e-testing", "humanize-ts", "test-scenarios"] }
    },
    "reviewer": {
      "description": "Independent reviewer for PRD, ADR and Implementation Plan content, architecture before implementation, and code before QA for correctness, evidence, simplicity, security, and readiness.",
      "tier": "verifier", "kind": "review", "tools": "verify",
      "skills": { "required": ["arch-simplicity-guard"], "optional": ["application-quality-assurance", "code-review-excellence", "pre-mortem", "test-scenarios"] }
    },
    "qa-verifier": {
      "description": "QA verification subagent for requirements, PRD, ADR and Implementation Plan readiness, tests, logs, reproduction, browser or simulator checks, and evidence of completed work.",
      "tier": "verifier", "kind": "qa", "tools": "verify",
      "skills": { "required": [], "optional": ["qa-requirement-reviewer", "application-quality-assurance", "playwright-e2e-testing", "browser-debugging", "test-scenarios", "webapp-testing", "e2e-testing-patterns"] }
    },
    "senior-qa-verifier": {
      "description": "Senior QA subagent for blocked resolution recovery, acceptance criteria review, test design, edge cases, negative cases, and QA adequacy checks.",
      "tier": "escalation", "kind": "qa", "tools": "verify",
      "skills": { "required": [], "optional": ["application-quality-assurance", "test-scenarios", "e2e-testing-patterns"] }
    }
  }
}
```

- [ ] **Шаг 2.** `roles/_common.md`. Добавляется в конец текста каждой роли:

```markdown
## Working in a foxmorrow run

Root starts you with a brief. The brief is your only source of the goal, acceptance criteria, allowed paths, dependency context, report format and the `References` directory with shared procedures. You do not see root's conversation or other agents' chains.

- Work only inside the allowed paths. Preserve changes made by others. Do not commit, push, publish or deploy unless the brief explicitly allows it.
- Prove each criterion you claim with `foxmorrow check -a <assignment> -- <command>`. Run a check once; if it was interrupted, inspect its effect before repeating it.
- Finish with exactly one `foxmorrow report -a <assignment>` call. A submitted report cannot be changed; root creates a new assignment for corrections.
- Report only what you observed. Reference evidence by check IDs (`chk-…`) or by the IDs of your own tool calls. A statement without evidence is not proof.
- Keep `status` (`completed` or `interrupted`) separate from `outcome` (`passed`, `failed` or `unverified`). Completed work may fail; interrupted work cannot pass.
- If a missing decision materially changes scope or correctness, stop and report `unverified` with the question in `summary`. Root asks the user.
- Choose the simplest solution that keeps the agreed architecture whole. Do not add abstractions, dependencies, services or scope that the brief does not require.
- Return findings to root. Do not hand work to other agents yourself.
- If the brief lists missing required skills, work without them and state in `summary` which decisions you made without their rules.
```

- [ ] **Шаг 3.** `profiles/devin.json`:

```json
{
  "version": 1,
  "brief_language": "en",
  "family_rule": true,
  "tiers": {
    "worker": "swe-2-high",
    "analyst": "claude-opus-5-5-high",
    "verifier": "gpt-6-1-sol-high",
    "escalation": "gpt-6-astra-xhigh"
  },
  "roles": {
    "supervising-architect": "claude-opus-5-5-max"
  },
  "families": [
    { "family": "swe", "prefixes": ["swe-"] },
    { "family": "claude", "prefixes": ["claude-"] },
    { "family": "gpt", "prefixes": ["gpt-"] },
    { "family": "gemini", "prefixes": ["gemini-"] },
    { "family": "kimi", "prefixes": ["kimi-"] },
    { "family": "glm", "prefixes": ["glm-"] },
    { "family": "grok", "prefixes": ["grok-"] },
    { "family": "deepseek", "prefixes": ["deepseek-"] },
    { "family": "nemotron", "prefixes": ["nemotron-"] },
    { "family": "muse", "prefixes": ["muse-"] }
  ]
}
```

- [ ] **Шаг 4.** `templates/brief.en.md`:

````markdown
Role: {{role}}   Profile: {{profile}}   role-sha256 {{role_sha256}}
Expected model: {{expected_models}}
References: {{references_dir}}

Skills to apply:
{{skills}}

Missing required skills:
{{missing_skills}}

## Goal

{{goal}}

## Acceptance criteria ({{criteria_source}})

{{criteria}}

## Allowed paths

{{allowed_paths}}

## Context from dependencies

{{context}}

## Rules

- Change only the allowed paths. Do not commit, push, publish or deploy.
- Run every check through:
  foxmorrow check -a {{assignment_id}} -- <command>
- Finish with exactly one report:

```sh
foxmorrow report -a {{assignment_id}} <<'JSON'
{"summary": "...", "status": "completed", "outcome": "passed",
 "criteria": {"ac-1": {"status": "passed", "reason": "...", "evidence": ["chk-001"]}},
 "evidence": ["chk-001"]}
JSON
```

`status` is `completed` or `interrupted`; `outcome` and criterion `status` are `passed`, `failed` or `unverified`.
````

- [ ] **Шаг 5.** `templates/brief.ru.md`:

````markdown
Роль: {{role}}   Профиль: {{profile}}   role-sha256 {{role_sha256}}
Ожидаемая модель: {{expected_models}}
Справочники: {{references_dir}}

Применяемые skills:
{{skills}}

Недостающие обязательные skills:
{{missing_skills}}

## Цель

{{goal}}

## Критерии приёмки ({{criteria_source}})

{{criteria}}

## Разрешённые пути

{{allowed_paths}}

## Контекст зависимостей

{{context}}

## Правила

- Меняй только разрешённые пути. Не делай commit, push, публикацию и deploy.
- Каждую проверку запускай через:
  foxmorrow check -a {{assignment_id}} -- <command>
- Заверши работу ровно одним отчётом:

```sh
foxmorrow report -a {{assignment_id}} <<'JSON'
{"summary": "...", "status": "completed", "outcome": "passed",
 "criteria": {"ac-1": {"status": "passed", "reason": "...", "evidence": ["chk-001"]}},
 "evidence": ["chk-001"]}
JSON
```

`status` принимает значения `completed` или `interrupted`; `outcome` и `status` критерия принимают значения `passed`, `failed` или `unverified`.
````

- [ ] **Шаг 6.** Коммит выполняется вместе с текстами ролей в задаче 8.

## Задача 8. Перенос текстов 21 роли

Тексты ролей предназначены для агентов, поэтому `humanizer-ru` и `ai-slops-hunter` к ним не применяются. Перенос выполняет исполнитель вручную, по правилам ниже. Соответствие правилам проверяет тест задачи 10.

**Источник:** `git -C /Users/ucnlejumper/Sites/agent-flow show 882b794:skills/agent-flow/agents/<role>.md`.

**Порядок для каждой роли:**

1. Взять файл из источника.
2. Удалить frontmatter между первыми двумя строками `---`: описание и skills уже перенесены в `roles/roles.json`, модель задаёт профиль.
3. Удалить раздел `## Execution guidance` целиком, до следующего заголовка `## `. Его заменяет `roles/_common.md`.
4. Сохранить заголовок `# <role>` и остальные разделы в исходном порядке.
5. Применить замены:

| Agent Flow | foxmorrow |
|---|---|
| `delegation packet` | `brief` |
| `the orchestrator`, `orchestrator`, `the root coordinator`, `root coordinator` | `root` |
| `[…](../references/flows.md#утверждение-документов)` | `[…](references/doc-flow.md#document-approval)` |
| `[…](../references/flows.md#маршрутизация-doc-flow)` | `[…](references/doc-flow.md#routing)` |
| `[…](../references/flows.md#подготовка-prd)` | `[…](references/doc-flow.md#prd)` |
| `[…](../references/flows.md#подготовка-adr)` | `[…](references/doc-flow.md#adr)` |
| `[…](../references/implementation-plan-authoring.md)` | `[…](references/doc-flow.md#plan)` |
| `references/traceable-runs.md` | `references/contract.md` |
| `journal-backed work` | `work in a foxmorrow run` |
| `the shared JSON report`, `shared JSON report` | `the report submitted with foxmorrow report` |
| `native response`, `native evidence`, `native actions` | `your own tool calls` |
| `the check writer`, `check writer` | `foxmorrow check` |

6. Удалить предложения о документах только в чате (содержат `restricted to chat`, `chat delivery procedure` или `chat document`): в foxmorrow v1 документы ведутся файлами.
7. В `reviewer` удалить пункт об аттестации временного автора как `verification-only`: этого механизма в журнале foxmorrow нет.
8. В `qa-verifier` фразу `Read the declared authored_paths and criterion IDs through the journal.` заменить на `Read the allowed paths and criterion IDs from the brief and foxmorrow status.`
9. Добавить пункты BRD:
   - `product-manager`, раздел `## Workflow`, первым пунктом: `- For BRD work, follow [the BRD procedure](references/doc-flow.md#brd). Request market input from marketing-growth-strategist through root when the BRD needs it.`
   - `marketing-growth-strategist`, раздел `## Use When`, новым пунктом: `- Root assigns market input for a BRD authored by product-manager.`
   - `documenter`, раздел `## Do Not Use When`, перед пунктом о PRD: `- A BRD must be created or substantively revised; use product-manager under [the BRD procedure](references/doc-flow.md#brd).`
10. Сохранить файл как `roles/<role>.md` с одним переводом строки в конце.

- [ ] **Шаг 1.** Перенести 16 ролей без особых пунктов: `supervising-architect`, `researcher`, `ai-slops-hunter`, `rag-retrieval-engineer`, `astro-worker`, `backend-worker`, `bun-worker`, `devops-worker`, `elixir-phoenix-worker`, `frontend-worker`, `golang-worker`, `ios-worker`, `python-worker`, `typescript-worker`, `senior-qa-verifier`, `architect`.

- [ ] **Шаг 2.** Перенести роли с особыми пунктами 7–9: `product-manager`, `marketing-growth-strategist`, `documenter`, `reviewer`, `qa-verifier`.

- [ ] **Шаг 3.** Для каждой роли сверить результат с источником и убедиться, что изменения ограничены пунктами 2–9:

```sh
for r in $(ls roles | sed -n 's/\.md$//p' | grep -v '^_common$'); do
  echo "== $r"
  git -C /Users/ucnlejumper/Sites/agent-flow show 882b794:skills/agent-flow/agents/$r.md | diff - roles/$r.md | head -60
done
```

- [ ] **Шаг 4.** Проверить отсутствие запрещённых терминов (тот же список проверяет тест задачи 10):

```sh
rg -n -i 'codex|traceable-runs|agent_config|native response|native evidence|native action|delegation packet|orchestrator|journal-backed|verification-only|chat delivery|restricted to chat|spawn_agent|\.\./references/|flows\.md|implementation-plan-authoring' roles
```

Ожидается пустой вывод.

- [ ] **Шаг 5.** Коммит:

```sh
git add roles profiles templates
git commit -m "Port 21 Agent Flow roles with the Devin model profile and brief templates"
```

## Задача 9. CLI: `init`, `assign`, `brief`, `sync`, `doctor`

**Файлы:** изменить `crates/foxmorrow/Cargo.toml`, `src/project.rs`; заменить `src/main.rs` и `tests/cli.rs`.

- [ ] **Шаг 1.** В `crates/foxmorrow/Cargo.toml` в `[dependencies]` добавить `foxmorrow-devin.workspace = true`.

- [ ] **Шаг 2.** Тест `crates/foxmorrow/tests/cli.rs` (заменяет файл плана 001):

```rust
#![cfg(unix)]

use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};

struct Setup {
    plugin: tempfile::TempDir,
    project: tempfile::TempDir,
    home: tempfile::TempDir,
}

impl Setup {
    fn new() -> Self {
        let setup = Setup {
            plugin: tempfile::tempdir().unwrap(),
            project: tempfile::tempdir().unwrap(),
            home: tempfile::tempdir().unwrap(),
        };
        let plugin = setup.plugin.path();
        write(plugin, "roles/roles.json", CATALOG);
        write(plugin, "roles/_common.md", "## Common\n\nShared rules.\n");
        for role in ["backend-worker", "skilled-worker", "reviewer"] {
            write(plugin, &format!("roles/{role}.md"), &format!("# {role}\n"));
        }
        write(plugin, "profiles/devin.json", PROFILE);
        write(plugin, "templates/brief.en.md", "Role: {{role}} Profile: {{profile}}\nMissing:\n{{missing_skills}}\nGoal: {{goal}}\nfoxmorrow report -a {{assignment_id}}\n");
        assert!(Command::new("git").arg("-C").arg(setup.project.path()).args(["init", "-q"]).status().unwrap().success());
        write(setup.project.path(), "task.md", "# Task\n\n## Критерии приёмки\n\n- Health returns 200\n");
        assert!(setup.run(&["sync", "--plugin"], None).status.success());
        setup
    }

    fn run(&self, args: &[&str], stdin: Option<&str>) -> Output {
        let mut child = Command::new(env!("CARGO_BIN_EXE_foxmorrow"))
            .args(args)
            .current_dir(self.project.path())
            .env("FOXMORROW_PLUGIN_ROOT", self.plugin.path())
            .env("FOXMORROW_DEVIN_BIN", "/nonexistent/devin")
            .env("HOME", self.home.path())
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

    fn init(&self) -> serde_json::Value {
        json(&self.run(&["init", "--criteria", "task.md#Критерии приёмки", "--paths", "src/"], None))
    }

    fn agent(&self, relative: &str) -> PathBuf {
        self.plugin.path().join(relative)
    }
}

const CATALOG: &str = r#"{
  "version": 1,
  "source": {"repository": "test/agent-flow", "commit": "0000000"},
  "roles": {
    "backend-worker": {"description": "Backend", "tier": "worker", "kind": "author", "tools": "full"},
    "skilled-worker": {"description": "Skilled", "tier": "worker", "kind": "author", "tools": "full", "skills": {"required": ["special-skill"]}},
    "reviewer": {"description": "Reviewer", "tier": "verifier", "kind": "review", "tools": "verify"}
  }
}"#;

const PROFILE: &str = r#"{
  "version": 1, "brief_language": "en", "family_rule": true,
  "tiers": {"worker": "swe-2-high", "analyst": "claude-opus-5-5-high", "verifier": "gpt-6-1-sol-high", "escalation": "gpt-6-astra-xhigh"},
  "families": [{"family": "swe", "prefixes": ["swe-"]}, {"family": "claude", "prefixes": ["claude-"]}, {"family": "gpt", "prefixes": ["gpt-"]}]
}"#;

fn write(root: &Path, relative: &str, text: &str) {
    let path = root.join(relative);
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(path, text).unwrap();
}

fn json(output: &Output) -> serde_json::Value {
    assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));
    serde_json::from_slice(&output.stdout).unwrap()
}

fn text(output: &Output) -> String {
    assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));
    String::from_utf8(output.stdout.clone()).unwrap()
}

fn failure(output: &Output, needle: &str) {
    assert_eq!(output.status.code(), Some(2), "{}", String::from_utf8_lossy(&output.stdout));
    assert!(String::from_utf8_lossy(&output.stderr).contains(needle), "{}", String::from_utf8_lossy(&output.stderr));
}

#[test]
fn init_records_profile_rule_and_missing_skills() {
    let setup = Setup::new();
    let init = setup.init();
    assert_eq!(init["criteria"][0]["id"], "ac-1");
    assert_eq!(init["family_rule"], true);
    assert_eq!(init["skills_source"], "approximate");
    assert_eq!(init["missing_skills"]["skilled-worker"][0], "special-skill");
}

#[test]
fn assign_prints_and_stores_the_brief() {
    let setup = Setup::new();
    setup.init();
    let brief = text(&setup.run(&["assign", "--role", "backend-worker", "--goal", "Add GET /health", "--paths", "src/"], None));
    assert!(brief.starts_with("foxmorrow assignment a-01 run r-"), "{brief}");
    assert!(brief.contains("Role: backend-worker Profile: foxmorrow:backend-worker\n"));
    assert!(brief.contains("Goal: Add GET /health\n"));
    assert_eq!(text(&setup.run(&["brief", "-a", "a-01"], None)), brief);

    let status = json(&setup.run(&["status"], None));
    assert_eq!(status["assignments"][0]["role"], "backend-worker");

    let check = setup.run(&["check", "-a", "a-01", "--", "sh", "-c", "echo hello; exit 3"], None);
    assert_eq!(check.status.code(), Some(3));
    let report = r#"{"summary":"tried","status":"completed","outcome":"failed","evidence":["chk-001"]}"#;
    json(&setup.run(&["report", "-a", "a-01"], Some(report)));
    failure(&setup.run(&["report", "-a", "a-01"], Some(report)), "already has a report");
}

#[test]
fn assign_refuses_stale_roles_and_missing_required_skills() {
    let setup = Setup::new();
    setup.init();
    std::fs::write(setup.agent("agents/backend-worker.md"), "edited").unwrap();
    failure(
        &setup.run(&["assign", "--role", "backend-worker", "--goal", "g", "--paths", "src/"], None),
        "sync --plugin",
    );
    failure(
        &setup.run(&["assign", "--role", "skilled-worker", "--goal", "g", "--paths", "src/"], None),
        "missing required skills: special-skill",
    );
    let brief = text(&setup.run(&["assign", "--role", "skilled-worker", "--goal", "g", "--paths", "src/", "--allow-missing-skills"], None));
    assert!(brief.contains("Missing:\n- special-skill\n"));
    failure(&setup.run(&["assign", "--role", "backend-worker", "--paths", "src/"], None), "--goal");
}

#[test]
fn project_overrides_need_sync_and_use_project_roles() {
    let setup = Setup::new();
    setup.init();
    write(setup.project.path(), ".foxmorrow/profile.json", r#"{"roles": {"backend-worker": "claude-sonnet-5-5-high"}}"#);
    failure(&setup.run(&["assign", "--role", "backend-worker", "--goal", "g", "--paths", "src/"], None), "foxmorrow sync");
    let check = setup.run(&["sync", "--check"], None);
    assert_eq!(check.status.code(), Some(1));
    json(&setup.run(&["sync"], None));
    assert!(setup.project.path().join(".devin/agents/fox-backend-worker.md").is_file());
    let brief = text(&setup.run(&["assign", "--role", "backend-worker", "--goal", "g", "--paths", "src/"], None));
    assert!(brief.contains("Profile: fox-backend-worker\n"));
    assert!(setup.run(&["sync", "--check"], None).status.success());
}

#[test]
fn root_assignments_and_doctor() {
    let setup = Setup::new();
    setup.init();
    let root = json(&setup.run(&["assign", "--root", "--paths", "src/"], None));
    assert_eq!(root["kind"], "root");
    let doctor = json(&setup.run(&["doctor"], None));
    assert_eq!(doctor["roles"], 3);
    assert_eq!(doctor["stale_agents"].as_array().unwrap().len(), 0);
    std::fs::remove_file(setup.agent("agents/reviewer.md")).unwrap();
    assert_eq!(setup.run(&["doctor"], None).status.code(), Some(1));
}

#[test]
fn status_outside_a_run_fails() {
    let setup = Setup::new();
    failure(&setup.run(&["status"], None), "no foxmorrow run");
}
```

- [ ] **Шаг 3.** Запустить: `cargo test -p foxmorrow --test cli`. Ожидается: тесты падают, потому что команд `sync`, `brief`, `doctor` и новых флагов `assign` нет.

- [ ] **Шаг 4.** `crates/foxmorrow/src/project.rs`:

```rust
use std::path::{Path, PathBuf};

use foxmorrow_core::catalog::CATALOG_FILE;
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

pub fn plugin_root(explicit: Option<PathBuf>) -> Result<PathBuf> {
    if let Some(path) = explicit.or_else(|| std::env::var_os("FOXMORROW_PLUGIN_ROOT").map(PathBuf::from)) {
        return Ok(path);
    }
    let executable = std::env::current_exe()?;
    executable
        .ancestors()
        .find(|dir| dir.join(CATALOG_FILE).is_file())
        .map(Path::to_path_buf)
        .ok_or_else(|| Error::NotFound("foxmorrow plugin root; set FOXMORROW_PLUGIN_ROOT or pass --plugin-root".into()))
}
```

- [ ] **Шаг 5.** `crates/foxmorrow/src/main.rs`:

```rust
mod project;

use std::collections::BTreeMap;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use clap::{Parser, Subcommand};
use foxmorrow_core::assignment::{self, BriefDraft, NewAssignment};
use foxmorrow_core::brief::{self, BriefInput, ContextItem};
use foxmorrow_core::catalog::Catalog;
use foxmorrow_core::error::invalid;
use foxmorrow_core::model::Kind;
use foxmorrow_core::run::{self, NewRun};
use foxmorrow_core::{Error, Result, baseline, criteria, receipt, report, status};
use foxmorrow_devin::agents::{self, PROJECT_PREFIX};
use foxmorrow_devin::profile::{Profile, ProjectProfile};
use foxmorrow_devin::skills::{self, Inventory};
use serde::Serialize;

#[derive(Parser)]
#[command(name = "foxmorrow", version, about = "Provable multi-agent runs")]
struct Cli {
    #[arg(long, global = true)]
    project: Option<PathBuf>,
    #[arg(long, global = true)]
    plugin_root: Option<PathBuf>,
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
        session_id: Option<String>,
    },
    Assign {
        #[arg(long, required_unless_present = "root_assignment")]
        role: Option<String>,
        #[arg(long = "root", conflicts_with = "role")]
        root_assignment: bool,
        #[arg(long)]
        goal: Option<String>,
        #[arg(long = "paths", required = true, num_args = 1..)]
        paths: Vec<String>,
        #[arg(long = "depends-on", num_args = 1..)]
        depends_on: Vec<String>,
        #[arg(long)]
        previous: Option<String>,
        #[arg(long)]
        allow_missing_skills: bool,
        #[arg(long)]
        run: Option<String>,
    },
    Brief {
        #[arg(short = 'a', long = "assignment")]
        assignment: String,
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
    Sync {
        #[arg(long)]
        plugin: bool,
        #[arg(long)]
        check: bool,
    },
    Doctor,
}

struct Plugin {
    root: PathBuf,
    catalog: Catalog,
    profile: Profile,
}

impl Plugin {
    fn load(explicit: Option<PathBuf>) -> Result<Self> {
        let root = project::plugin_root(explicit)?;
        Ok(Self { catalog: Catalog::load(&root)?, profile: Profile::load(&root)?, root })
    }
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
            let (created, _) = run::create_run(NewRun {
                project_root: root,
                criteria_path: file.to_string(),
                criteria_heading: heading.to_string(),
                criteria_markdown: markdown,
                allowed_paths: paths,
                baselines,
                root_session_id: session_id,
                family_rule: plugin.profile.family_rule(&project_profile),
                skills_summary: skills_summary.clone(),
            })?;
            print_json(&serde_json::json!({
                "run": created.id,
                "criteria": created.criteria,
                "family_rule": created.family_rule,
                "skills_source": found.source,
                "missing_skills": skills_summary,
            }))?;
        }
        Command::Assign { role, root_assignment, goal, paths, depends_on, previous, allow_missing_skills, run: run_id } => {
            let project_root = project::for_existing(cli.project, &cwd)?;
            let (current, mut journal) = run::open_run(&project_root, run_id.as_deref())?;
            if root_assignment {
                let created = assignment::create(
                    &mut journal,
                    &current,
                    NewAssignment {
                        role: "root".into(),
                        kind: Kind::Root,
                        profile: None,
                        expected_models: Vec::new(),
                        brief_sha256: None,
                        role_sha256: None,
                        allowed_paths: paths,
                        depends_on,
                        previous_id: previous,
                    },
                )?;
                print_json(&created)?;
                return Ok(ExitCode::SUCCESS);
            }
            let Some(role) = role else {
                return invalid("--role or --root is required");
            };
            let Some(goal) = goal else {
                return invalid("--goal is required for a delegated assignment");
            };
            let plugin = Plugin::load(cli.plugin_root)?;
            let project_profile = ProjectProfile::load(&project_root)?;
            let spec = plugin.catalog.role(&role)?.clone();
            let agent = agents::definition(&plugin.root, &project_root, &plugin.catalog, &plugin.profile, &project_profile, &role)?;
            agents::ensure_current(&agent)?;
            let found = inventory(&project_root);
            let missing = found.missing(&spec.skills.required);
            if !missing.is_empty() && !allow_missing_skills {
                return invalid(format!(
                    "role {role} is missing required skills: {}; install them or pass --allow-missing-skills",
                    missing.join(", ")
                ));
            }
            let apply: Vec<String> = spec
                .skills
                .required
                .iter()
                .chain(&spec.skills.optional)
                .filter(|skill| found.names.contains(*skill))
                .cloned()
                .collect();
            let template = brief::load_template(&plugin.root, &plugin.profile.brief_language)?;
            let mut context = Vec::new();
            for id in &depends_on {
                let dependency = assignment::get(journal.conn(), id)?;
                if let Some((body, _)) = report::get(journal.conn(), id)? {
                    context.push(ContextItem {
                        assignment_id: dependency.id,
                        role: dependency.role,
                        outcome: body.outcome,
                        summary: body.summary,
                    });
                }
            }
            let references = plugin.root.join("skills").join("fox").join("references").display().to_string();
            let language = plugin.profile.brief_language.clone();
            let (created, text) = assignment::create_with_brief(
                &mut journal,
                &current,
                NewAssignment {
                    role,
                    kind: spec.kind,
                    profile: Some(agent.profile.clone()),
                    expected_models: vec![agent.model.clone()],
                    brief_sha256: None,
                    role_sha256: Some(agent.role_sha256.clone()),
                    allowed_paths: paths,
                    depends_on,
                    previous_id: previous,
                },
                |draft| {
                    let body = brief::render(
                        &template,
                        &BriefInput {
                            run: &current,
                            assignment: draft,
                            references_dir: &references,
                            skills: &apply,
                            missing_skills: &missing,
                            goal: &goal,
                            context: &context,
                        },
                    )?;
                    Ok(BriefDraft { language, body })
                },
            )?;
            eprintln!("foxmorrow: {} profile={} model={}", created.id, agent.profile, agent.model);
            print!("{text}");
        }
        Command::Brief { assignment: id, run: run_id } => {
            let project_root = project::for_existing(cli.project, &cwd)?;
            let (_, journal) = run::open_run(&project_root, run_id.as_deref())?;
            let Some((_, text)) = assignment::brief(journal.conn(), &id)? else {
                return Err(Error::NotFound(format!("brief for {id}")));
            };
            print!("{text}");
        }
        Command::Check { assignment: id, run: run_id, command } => {
            let project_root = project::for_existing(cli.project, &cwd)?;
            let (_, mut journal) = run::open_run(&project_root, run_id.as_deref())?;
            let recorded = receipt::run_check(&mut journal, &id, &cwd, &command)?;
            print!("{}", recorded.output_tail.as_deref().unwrap_or(""));
            let shown = recorded.exit_code.map_or_else(|| "none".to_string(), |code| code.to_string());
            eprintln!("foxmorrow: {} exit={shown}", recorded.id);
            let code = recorded.exit_code.and_then(|code| u8::try_from(code).ok()).unwrap_or(1);
            return Ok(ExitCode::from(code));
        }
        Command::Report { assignment: id, run: run_id } => {
            let project_root = project::for_existing(cli.project, &cwd)?;
            let (current, mut journal) = run::open_run(&project_root, run_id.as_deref())?;
            let mut input = String::new();
            std::io::stdin().read_to_string(&mut input)?;
            let body = report::submit(&mut journal, &current, &id, &input)?;
            print_json(&serde_json::json!({ "assignment": id, "status": body.status, "outcome": body.outcome }))?;
        }
        Command::Status { run: run_id } => {
            let project_root = project::for_existing(cli.project, &cwd)?;
            let (current, journal) = run::open_run(&project_root, run_id.as_deref())?;
            print_json(&status::status(journal.conn(), &current)?)?;
        }
        Command::Sync { plugin: plugin_mode, check } => {
            let plugin = Plugin::load(cli.plugin_root)?;
            let report = if plugin_mode {
                let definitions = agents::plugin_definitions(&plugin.root, &plugin.catalog, &plugin.profile)?;
                agents::sync(&definitions, &plugin.root.join("agents"), None, check)?
            } else {
                let project_root = project::for_init(cli.project, &cwd);
                let project_profile = ProjectProfile::load(&project_root)?;
                let definitions = agents::project_definitions(&plugin.root, &project_root, &plugin.catalog, &plugin.profile, &project_profile)?;
                agents::sync(&definitions, &project_root.join(".devin").join("agents"), Some(PROJECT_PREFIX), check)?
            };
            print_json(&report)?;
            if check && !report.stale.is_empty() {
                return Ok(ExitCode::from(1));
            }
        }
        Command::Doctor => {
            let plugin = Plugin::load(cli.plugin_root)?;
            let project_root = project::for_init(cli.project, &cwd);
            let project_profile = ProjectProfile::load(&project_root)?;
            let found = inventory(&project_root);
            let plugin_definitions = agents::plugin_definitions(&plugin.root, &plugin.catalog, &plugin.profile)?;
            let mut stale = agents::sync(&plugin_definitions, &plugin.root.join("agents"), None, true)?.stale;
            let project_definitions = agents::project_definitions(&plugin.root, &project_root, &plugin.catalog, &plugin.profile, &project_profile)?;
            stale.extend(agents::sync(&project_definitions, &project_root.join(".devin").join("agents"), Some(PROJECT_PREFIX), true)?.stale);
            print_json(&serde_json::json!({
                "plugin_root": plugin.root,
                "roles": plugin.catalog.roles.len(),
                "brief_language": plugin.profile.brief_language,
                "family_rule": plugin.profile.family_rule(&project_profile),
                "skills_source": found.source,
                "missing_required_skills": missing_required(&plugin.catalog, &found),
                "stale_agents": stale,
            }))?;
            if !stale.is_empty() {
                return Ok(ExitCode::from(1));
            }
        }
    }
    Ok(ExitCode::SUCCESS)
}

fn devin_binary() -> PathBuf {
    std::env::var_os("FOXMORROW_DEVIN_BIN").map(PathBuf::from).unwrap_or_else(|| PathBuf::from("devin"))
}

fn home_dir() -> PathBuf {
    std::env::var_os("HOME")
        .or_else(|| std::env::var_os("USERPROFILE"))
        .map(PathBuf::from)
        .unwrap_or_default()
}

fn inventory(project_root: &Path) -> Inventory {
    skills::inventory(project_root, &devin_binary(), &home_dir())
}

fn missing_required(catalog: &Catalog, found: &Inventory) -> BTreeMap<String, Vec<String>> {
    catalog
        .roles
        .iter()
        .map(|(name, spec)| (name.clone(), found.missing(&spec.skills.required)))
        .filter(|(_, missing)| !missing.is_empty())
        .collect()
}

fn print_json(value: &impl Serialize) -> Result<()> {
    println!("{}", serde_json::to_string_pretty(value)?);
    Ok(())
}
```

- [ ] **Шаг 6.** Запустить: `cargo test -p foxmorrow --test cli`. Ожидается: 6 тестов пройдены.

- [ ] **Шаг 7.** Коммит:

```sh
git add crates/foxmorrow
git commit -m "Derive assignment profile, model and brief in the CLI"
```

## Задача 10. Генерация ролей плагина и проверки реального содержимого

**Файлы:** создать `agents/<role>.md` (генерация), `crates/foxmorrow-devin/tests/repository.rs`.

- [ ] **Шаг 1.** Сгенерировать роли плагина из корня репозитория:

```sh
cargo run -q -p foxmorrow -- --plugin-root . sync --plugin
cargo run -q -p foxmorrow -- --plugin-root . sync --plugin --check
```

Ожидается: первая команда записывает 21 файл, вторая печатает пустой `stale` и завершается с кодом 0.

- [ ] **Шаг 2.** Тест `crates/foxmorrow-devin/tests/repository.rs`:

```rust
use std::collections::BTreeSet;
use std::path::PathBuf;

use foxmorrow_core::brief;
use foxmorrow_core::catalog::{self, Catalog};
use foxmorrow_core::model::Kind;
use foxmorrow_devin::agents;
use foxmorrow_devin::profile::{Profile, ProjectProfile};

const ROLES: [&str; 21] = [
    "product-manager", "architect", "supervising-architect", "researcher", "documenter", "ai-slops-hunter",
    "marketing-growth-strategist", "rag-retrieval-engineer", "astro-worker", "backend-worker", "bun-worker",
    "devops-worker", "elixir-phoenix-worker", "frontend-worker", "golang-worker", "ios-worker", "python-worker",
    "typescript-worker", "reviewer", "qa-verifier", "senior-qa-verifier",
];

const FORBIDDEN: [&str; 16] = [
    "codex", "traceable-runs", "agent_config", "native response", "native evidence", "native action",
    "delegation packet", "orchestrator", "journal-backed", "verification-only", "chat delivery",
    "restricted to chat", "spawn_agent", "../references/", "flows.md", "implementation-plan-authoring",
];

const PLACEHOLDERS: [&str; 13] = [
    "assignment_id", "role", "profile", "role_sha256", "expected_models", "references_dir", "skills",
    "missing_skills", "goal", "criteria_source", "criteria", "allowed_paths", "context",
];

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..").canonicalize().unwrap()
}

#[test]
fn catalog_has_exactly_the_v1_roles() {
    let catalog = Catalog::load(&root()).unwrap();
    let names: BTreeSet<&str> = catalog.roles.keys().map(String::as_str).collect();
    assert_eq!(names, ROLES.into_iter().collect());
}

#[test]
fn verifiers_never_share_a_model_family_with_authors() {
    let root = root();
    let catalog = Catalog::load(&root).unwrap();
    let profile = Profile::load(&root).unwrap();
    let none = ProjectProfile::default();
    let family = |role: &str| {
        let spec = catalog.role(role).unwrap();
        profile.family_of(&profile.model_for(role, spec.tier, &none).model).unwrap().to_string()
    };
    for (verifier, spec) in catalog.roles.iter().filter(|(_, spec)| spec.kind.is_verification()) {
        for (author, _) in catalog.roles.iter().filter(|(_, spec)| spec.kind == Kind::Author) {
            assert_ne!(family(verifier), family(author), "{verifier} and {author} share a model family");
        }
        assert_eq!(spec.tools, catalog::ToolSet::Verify);
    }
}

#[test]
fn generated_plugin_agents_are_current() {
    let root = root();
    let catalog = Catalog::load(&root).unwrap();
    let profile = Profile::load(&root).unwrap();
    let definitions = agents::plugin_definitions(&root, &catalog, &profile).unwrap();
    let report = agents::sync(&definitions, &root.join("agents"), None, true).unwrap();
    assert!(report.stale.is_empty(), "run foxmorrow sync --plugin: {:?}", report.stale);
}

#[test]
fn role_texts_are_ported_cleanly() {
    let root = root();
    for role in ROLES {
        let text = catalog::role_text(&root, role).unwrap();
        assert!(text.starts_with(&format!("# {role}\n")), "{role} must start with its heading");
        let lower = text.to_lowercase();
        for term in FORBIDDEN {
            assert!(!lower.contains(term), "{role} still contains '{term}'");
        }
        for (index, _) in text.match_indices("references/") {
            let rest = &text[index + "references/".len()..];
            assert!(
                rest.starts_with("doc-flow.md") || rest.starts_with("contract.md"),
                "{role} links to an unknown reference: {}",
                &rest[..rest.len().min(40)]
            );
        }
    }
}

#[test]
fn brief_templates_agree_on_commands_and_placeholders() {
    let root = root();
    let commands = |language: &str| -> Vec<String> {
        let template = brief::load_template(&root, language).unwrap();
        for placeholder in PLACEHOLDERS {
            assert!(template.contains(&format!("{{{{{placeholder}}}}}")), "{language} lacks {placeholder}");
        }
        template
            .lines()
            .map(str::trim)
            .filter(|line| line.starts_with("foxmorrow ") || line.starts_with('{') || line.starts_with("\"criteria\"") || line.starts_with("\"evidence\""))
            .map(str::to_string)
            .collect()
    };
    let english = commands("en");
    assert_eq!(english.len(), 5);
    assert_eq!(english, commands("ru"));
}
```

- [ ] **Шаг 3.** Запустить: `cargo test -p foxmorrow-devin --test repository`. Ожидается: 5 тестов пройдены. Если `role_texts_are_ported_cleanly` падает, исправить перенос по задаче 8 и повторить шаг 1.

- [ ] **Шаг 4.** Коммит:

```sh
git add agents crates/foxmorrow-devin/tests/repository.rs
git commit -m "Generate plugin role files and check the shipped content"
```

## Задача 11. Итоговая проверка

- [ ] **Шаг 1.**

```sh
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cargo run -q -p foxmorrow -- --plugin-root . sync --plugin --check
rg -n '^\s*//' crates --glob '*.rs'
```

Ожидается: первые четыре команды завершаются с кодом 0, последняя ничего не находит. Предупреждения `clippy` исправляются в коде без `allow`, кроме `#![allow(dead_code)]` в общих модулях тестов.

- [ ] **Шаг 2.** Ручная проверка в этом репозитории без запуска Devin. `HOME` указывает на реальный домашний каталог, поэтому список skills берётся из `devin skills list --json`:

```sh
cargo run -q -p foxmorrow -- --plugin-root . doctor
```

Ожидается: `"roles": 21`, `"skills_source": "devin"`, пустой `stale_agents` и пустой `missing_required_skills`. Все обязательные skills (`product-manager`, `arch-simplicity-guard`, `ai-slops-hunter`, `astro`, `astro-best-practices`, `elixir-essentials`) лежат в `~/.agents/skills`; `arch-simplicity-guard` — ссылкой на `~/Sites/arch-simplicity-guard`. Если какой-то из них в выводе есть, проверить, что он лежит в `~/.agents/skills`, а не только в `~/.codex/skills`. У других пользователей guard ставится через `requiredPlugins` плагина (план 005).

- [ ] **Шаг 3.** Сверить реализацию с разделом «Критерии приёмки»: для каждого пункта назвать тест или команду, которые его подтверждают.

- [ ] **Шаг 4.** Коммит исправлений, если они были:

```sh
git add -A crates roles profiles templates agents Cargo.lock
git commit -m "Fix formatting and lint findings"
```

## Связь с планом 004

Тексты ролей ссылаются на `references/doc-flow.md` с якорями `document-approval`, `routing`, `brd`, `prd`, `adr`, `plan` и на `references/contract.md`. План 004 создаёт эти файлы в `skills/fox/references/` и добавляет тест, проверяющий, что каждый якорь из текстов ролей существует.
