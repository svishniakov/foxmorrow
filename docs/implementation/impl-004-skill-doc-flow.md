# План 004. Skill `fox` и Doc Flow

Document status: approved
Based on: [foxmorrow v1: дизайн](../design/2026-11-10-foxmorrow-v1-design.md), [дорожная карта](roadmap-v1.md), [план 002](impl-002-devin-evidence.md), [план 003](impl-003-roles-profiles-briefs.md)

Задачи выполняются по порядку, шаги отмечаются чекбоксами `- [ ]`. Шаги «Коммит» выполняются, только если пользователь разрешил коммиты в этом run.

**Цель:** root получает инструкции для работы через foxmorrow, а документы проходят Doc Flow с проверкой цепочки `Based on`. После плана `$fox` можно применить к задаче с критериями, к реализации утверждённого плана и к подготовке BRD, ADR, PRD или Implementation Plan.

**Архитектура:**
* **Skill `fox`**: инструкции только для root, это `skills/fox/SKILL.md` и три справочника в `skills/fox/references/`. Тексты написаны для агентов на английском, как и тексты ролей.
* **Разбор документов** в ядре (`document`). Модуль читает заголовок `Document status` и `Based on`, находит раздел открытых вопросов и проверяет цепочку утверждённых документов. От Devin он не зависит.
* **`init` получает три источника критериев:** `--criteria <файл#раздел>`, `--plan <путь>` и `--doc <тип> --based-on <путь…>`. Указывается ровно один.
* **Путь к CLI в брифе.** Субагент не знает, где лежит `foxmorrow`, поэтому бриф передаёт абсолютный путь через новый заполнитель `{{foxmorrow}}`.

**Стек:** тот же, что в планах с 001 по 003. Новых внешних зависимостей нет.

**Вне плана:** загрузчик `bin/foxmorrow`, `hooks.json`, манифест плагина, CI, README (план 005); проверка skill в живом Devin (план 005).

## Решения плана, уточняющие дизайн

| Вопрос | Решение | Основание |
|---|---|---|
| Состав справочников | `orchestrator.md`, `doc-flow.md`, `contract.md`. Выбор потока описан в самом `SKILL.md`, отдельного `flows.md` нет | В дизайне §5.1 указан и `flows`. Но выбор потока занимает одну таблицу, а отдельный файл дублировал бы `orchestrator.md` |
| Якоря `doc-flow.md` | Заголовки `## Document approval`, `## Routing`, `## BRD`, `## PRD`, `## ADR`, `## Plan` дают якоря, на которые уже ссылаются тексты ролей из плана 003 | Тест проверяет, что каждый якорь из ролей существует |
| Критерии для документа | `init --doc <тип>` берёт критерии из раздела `### <TYPE> acceptance criteria` в `doc-flow.md` плагина. В run записывается абсолютный путь к этому файлу | Дизайн §7.1: продукт такого run это документ. Критерии типа документа одинаковы для всех проектов |
| Критерии для плана | `init --plan` берёт раздел `Критерии приёмки`, а если его нет, `Acceptance criteria` | Планы этого репозитория используют первый вариант, а другие проекты с плагином могут писать на английском |
| `Based on` для типов | Для `brd` список может быть пустым. Для `adr`, `prd` и `plan` нужен хотя бы один документ. Тип вышестоящих документов не проверяется | Дизайн §7.1 задаёт порядок цепочки, но проверка типов потребовала бы поля типа в заголовке документа, которого в дизайне нет |
| Открытые вопросы | Раздел с заголовком `Открытые вопросы` или `Open questions` (номер раздела допускается) открыт, если в нём есть строка, кроме `Нет`, `None` и их вариантов. Текст в блоках кода не учитывается | Дизайн §7.1: документ с открытыми вопросами не утверждается |
| Решение о новом ADR после PRD | Отдельной команды нет. Root назначает `architect` в run PRD, и решение с причиной попадает в его отчёт | Дизайн §7.1 требует записать решение в run PRD; отчёт `architect` уже хранится в журнале |
| Путь к CLI | `{{foxmorrow}}` = `FOXMORROW_BIN`, иначе `<plugin root>/bin/foxmorrow` (`foxmorrow.cmd` на Windows), если файл есть, иначе путь текущего бинарника | Субагенты не наследуют `PATH` root, а загрузчик появится в плане 005 |
| Утверждение документа | Статус `approved` ставит root только после явного одобрения пользователя, отдельным изменением после `finalize` | Дизайн §7.1: вывод проверки не утверждает документ |

## Критерии приёмки

* `cargo fmt --all --check`, `cargo clippy --workspace --all-targets -- -D warnings` и `cargo test --workspace` завершаются успешно на macOS.
* Модуль `document` читает `Document status` и `Based on` до первого заголовка `## `, разрешает ссылки относительно каталога документа, пропускает внешние ссылки и якоря. Он находит открытые вопросы с учётом вариантов «нет вопросов» и блоков кода.
* Проверка цепочки возвращает все документы цепочки, если каждый существует, утверждён и не имеет открытых вопросов. Иначе перечисляет все найденные проблемы. Циклы ссылок не зацикливают проверку.
* `init --plan` создаёт run только при утверждённой цепочке и берёт критерии из раздела плана. `init --doc` создаёт run только при утверждённых `--based-on` и берёт критерии типа из `doc-flow.md`. Для `adr`, `prd` и `plan` без `--based-on` команда отказывает. Без источника критериев или с двумя источниками `init` отказывает.
* Бриф содержит абсолютный путь к CLI во всех командах `check` и `report`. Шаблоны `en` и `ru` используют одинаковые команды.
* `skills/fox/SKILL.md` содержит frontmatter с `name: fox` и непустым `description`. Каждая ссылка `references/doc-flow.md#…` из текстов ролей ведёт на существующий заголовок. Раздел критериев каждого типа документа разбирается в список.
* Роли плагина перегенерированы, `sync --plugin --check` не находит расхождений.
* В коде нет поясняющих комментариев.

## Структура файлов

```
crates/foxmorrow-core/src/document.rs           заголовок документа, открытые вопросы, цепочка
crates/foxmorrow-core/src/lib.rs
crates/foxmorrow-core/src/brief.rs              заполнитель {{foxmorrow}}
crates/foxmorrow-core/tests/document.rs
crates/foxmorrow-core/tests/brief.rs
crates/foxmorrow/src/main.rs                    источники критериев init, путь к CLI в брифе
crates/foxmorrow/tests/cli.rs
crates/foxmorrow-devin/tests/repository.rs      проверки skill и шаблонов
templates/brief.en.md
templates/brief.ru.md
roles/_common.md
agents/*.md                                     перегенерация
skills/fox/SKILL.md
skills/fox/references/orchestrator.md
skills/fox/references/doc-flow.md
skills/fox/references/contract.md
```

## Задача 1. Разбор документов и проверка цепочки

**Файлы:** создать `crates/foxmorrow-core/src/document.rs`, `tests/document.rs`; изменить `src/lib.rs`.

- [ ] **Шаг 1.** Тест `crates/foxmorrow-core/tests/document.rs`:

```rust
use std::path::Path;

use foxmorrow_core::Error;
use foxmorrow_core::document;

fn write(root: &Path, relative: &str, text: &str) {
    let path = root.join(relative);
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(path, text).unwrap();
}

fn doc(status: &str, based_on: &str, body: &str) -> String {
    format!("# Title\n\nDocument status: {status}\nBased on: {based_on}\n\n## Body\n\n{body}\n")
}

#[test]
fn reads_the_header() {
    let parsed = document::parse(
        Path::new("/p/docs/plan/impl.md"),
        &doc(
            "Approved",
            "[design](../design/d.md), [section](../prd/p.md#scope), [site](https://example.com), [here](#top)",
            "Document status: draft\nBased on: [late](late.md)",
        ),
    );
    assert_eq!(parsed.status.as_deref(), Some("approved"));
    assert_eq!(
        parsed.based_on,
        vec![Path::new("/p/docs/plan/../design/d.md").to_path_buf(), Path::new("/p/docs/plan/../prd/p.md").to_path_buf()]
    );
    assert!(!parsed.open_questions);
    assert_eq!(document::parse(Path::new("x.md"), "# Only a title\n").status, None);
}

#[test]
fn finds_open_questions() {
    let open = |body: &str| document::parse(Path::new("x.md"), &doc("draft", "", body)).open_questions;
    assert!(open("## 11. Открытые вопросы\n\n1. Кто владелец репозитория?"));
    assert!(open("### Open questions\n\n- Which region?"));
    assert!(!open("## Открытые вопросы\n\nНет."));
    assert!(!open("## Open questions\n\n- None\n\n## Next\n\n- item"));
    assert!(!open("## Notes\n\n```\n## Open questions\n- in code\n```"));
    assert!(open("## Открытые вопросы\n\n### Подраздел\n\nВопрос без списка"));
}

#[test]
fn checks_the_whole_chain() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    write(root, "docs/brd.md", &doc("approved", "", "text"));
    write(root, "docs/prd.md", &doc("approved", "[brd](brd.md)", "text"));
    write(root, "docs/plan.md", &doc("approved", "[prd](prd.md), [brd](brd.md)", "text"));
    let checked = document::check_chain(&[root.join("docs/plan.md")]).unwrap();
    assert_eq!(checked.len(), 3);

    write(root, "docs/brd.md", &doc("draft", "", "## Open questions\n\n- Budget?"));
    write(root, "docs/prd.md", &doc("approved", "[brd](brd.md), [gone](gone.md)", "text"));
    let error = document::check_chain(&[root.join("docs/plan.md")]).unwrap_err();
    assert!(matches!(error, Error::Invalid(_)));
    let text = error.to_string();
    for needle in ["brd.md has status draft", "brd.md has open questions", "gone.md does not exist"] {
        assert!(text.contains(needle), "{text}");
    }
}

#[test]
fn survives_reference_cycles() {
    let dir = tempfile::tempdir().unwrap();
    write(dir.path(), "a.md", &doc("approved", "[b](b.md)", "text"));
    write(dir.path(), "b.md", &doc("approved", "[a](a.md)", "text"));
    assert_eq!(document::check_chain(&[dir.path().join("a.md")]).unwrap().len(), 2);
    assert!(document::check_chain(&[]).unwrap().is_empty());
}
```

- [ ] **Шаг 2.** Запустить: `cargo test -p foxmorrow-core --test document`. Ожидается ошибка компиляции: модуля `document` нет.

- [ ] **Шаг 3.** `crates/foxmorrow-core/src/document.rs`:

```rust
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use crate::error::{Error, Result, invalid};

const QUESTION_HEADINGS: [&str; 2] = ["открытые вопросы", "open questions"];
const NONE_MARKERS: [&str; 5] = ["нет", "none", "нет открытых вопросов", "no open questions", "—"];

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Document {
    pub path: PathBuf,
    pub status: Option<String>,
    pub based_on: Vec<PathBuf>,
    pub open_questions: bool,
}

pub fn read(path: &Path) -> Result<Document> {
    let text = std::fs::read_to_string(path).map_err(|_| Error::NotFound(format!("document {}", path.display())))?;
    Ok(parse(path, &text))
}

pub fn parse(path: &Path, text: &str) -> Document {
    let base = path.parent().unwrap_or(Path::new(""));
    let mut status = None;
    let mut based_on = Vec::new();
    for line in text.lines().take_while(|line| !line.starts_with("## ")) {
        if let Some(value) = line.strip_prefix("Document status:") {
            status = Some(value.trim().to_lowercase());
        }
        if let Some(value) = line.strip_prefix("Based on:") {
            based_on = links(value).into_iter().map(|target| base.join(target)).collect();
        }
    }
    Document { path: path.to_path_buf(), status, based_on, open_questions: has_open_questions(text) }
}

pub fn check_chain(starts: &[PathBuf]) -> Result<Vec<Document>> {
    let mut problems = Vec::new();
    let mut checked = Vec::new();
    let mut seen = BTreeSet::new();
    let mut queue: Vec<PathBuf> = starts.iter().rev().cloned().collect();
    while let Some(path) = queue.pop() {
        if !seen.insert(path.canonicalize().unwrap_or_else(|_| path.clone())) {
            continue;
        }
        let Ok(document) = read(&path) else {
            problems.push(format!("{} does not exist", path.display()));
            continue;
        };
        if document.status.as_deref() != Some("approved") {
            problems.push(format!("{} has status {}", path.display(), document.status.as_deref().unwrap_or("none")));
        }
        if document.open_questions {
            problems.push(format!("{} has open questions", path.display()));
        }
        queue.extend(document.based_on.iter().rev().cloned());
        checked.push(document);
    }
    if !problems.is_empty() {
        return invalid(format!("document chain is not approved: {}", problems.join("; ")));
    }
    Ok(checked)
}

fn links(text: &str) -> Vec<String> {
    let mut found = Vec::new();
    let mut rest = text;
    while let Some(start) = rest.find("](") {
        let after = &rest[start + 2..];
        let Some(end) = after.find(')') else {
            break;
        };
        let target = after[..end].split('#').next().unwrap_or("").trim();
        if !target.is_empty() && !target.contains("://") {
            found.push(target.to_string());
        }
        rest = &after[end + 1..];
    }
    found
}

fn has_open_questions(text: &str) -> bool {
    let mut in_fence = false;
    let mut section: Option<usize> = None;
    for line in text.lines() {
        if line.trim_start().starts_with("```") {
            in_fence = !in_fence;
            continue;
        }
        if in_fence {
            continue;
        }
        if let Some((depth, title)) = heading(line) {
            if section.is_some_and(|open| depth <= open) {
                section = None;
            }
            if section.is_none() && is_question_heading(&title) {
                section = Some(depth);
            }
            continue;
        }
        if section.is_some() && !is_none_marker(line) {
            return true;
        }
    }
    false
}

fn heading(line: &str) -> Option<(usize, String)> {
    let depth = line.bytes().take_while(|byte| *byte == b'#').count();
    if !(1..=6).contains(&depth) {
        return None;
    }
    let title = line[depth..].strip_prefix(' ')?;
    Some((depth, title.trim().to_lowercase()))
}

fn is_question_heading(title: &str) -> bool {
    let title = title.trim_start_matches(|c: char| c.is_ascii_digit() || c == '.' || c == ' ' || c == '§');
    QUESTION_HEADINGS.contains(&title)
}

fn is_none_marker(line: &str) -> bool {
    let text = line.trim().trim_start_matches(['-', '*']).trim().trim_end_matches('.').to_lowercase();
    text.is_empty() || NONE_MARKERS.contains(&text.as_str())
}
```

- [ ] **Шаг 4.** В `crates/foxmorrow-core/src/lib.rs` добавить `pub mod document;` между `pub mod criteria;` и `pub mod error;`.

- [ ] **Шаг 5.** Запустить: `cargo test -p foxmorrow-core --test document`. Ожидается: 4 теста пройдены.

- [ ] **Шаг 6.** Коммит:

```sh
git add crates/foxmorrow-core
git commit -m "Check document status, open questions and Based on chains"
```

## Задача 2. Путь к CLI в брифе

**Файлы:** изменить `crates/foxmorrow-core/src/brief.rs`, `tests/brief.rs`, `templates/brief.en.md`, `templates/brief.ru.md`, `roles/_common.md`, `crates/foxmorrow/src/main.rs`, `crates/foxmorrow-devin/tests/repository.rs`; перегенерировать `agents/`.

- [ ] **Шаг 1.** Тесты. В `crates/foxmorrow-core/tests/brief.rs`:
  * в константе `TEMPLATE` заменить конец `"foxmorrow report -a {{assignment_id}}\n"` на `"{{foxmorrow}} report -a {{assignment_id}}\n"`;
  * во всех литералах `BriefInput { … }` после `references_dir: …,` добавить `foxmorrow: "/plugin/bin/foxmorrow",`;
  * в ожидаемом тексте теста `fills_every_placeholder` заменить последнюю строку `foxmorrow report -a a-01\n` на `/plugin/bin/foxmorrow report -a a-01\n`.

В `crates/foxmorrow-devin/tests/repository.rs`:
  * константу `PLACEHOLDERS` расширить до 14 элементов, добавив `"foxmorrow"`;
  * в фильтре теста `brief_templates_agree_on_commands_and_placeholders` условие `line.starts_with("foxmorrow ")` заменить на `line.starts_with("{{foxmorrow}} ")`.

- [ ] **Шаг 2.** Запустить: `cargo test -p foxmorrow-core --test brief`. Ожидается ошибка компиляции: в `BriefInput` нет поля `foxmorrow`.

- [ ] **Шаг 3.** `brief.rs`. В `BriefInput` после `references_dir` добавить поле `pub foxmorrow: &'a str,`. В вектор `values` после пары `references_dir` добавить:

```rust
        ("foxmorrow", input.foxmorrow.to_string()),
```

- [ ] **Шаг 4.** Шаблоны. В `templates/brief.en.md` и `templates/brief.ru.md` заменить `foxmorrow check -a {{assignment_id}}` на `{{foxmorrow}} check -a {{assignment_id}}` и `foxmorrow report -a {{assignment_id}}` на `{{foxmorrow}} report -a {{assignment_id}}`. В `en` после строки `References: {{references_dir}}` добавить:

```
CLI: {{foxmorrow}}
```

В `ru` после строки `Справочники: {{references_dir}}` добавить:

```
CLI: {{foxmorrow}}
```

- [ ] **Шаг 5.** `roles/_common.md`. Первый абзац после заголовка дополнить предложением в конце: `Run foxmorrow only through the absolute CLI path given in the brief.`

- [ ] **Шаг 6.** `crates/foxmorrow/src/main.rs`. В литерал `BriefInput { … }` ветки `Command::Assign` после `references_dir: &references,` добавить `foxmorrow: &cli_path,`, а перед вызовом `assignment::create_with_brief` добавить:

```rust
            let cli_path = foxmorrow_cli(&plugin.root).display().to_string();
```

В конец файла добавить:

```rust
fn foxmorrow_cli(plugin_root: &Path) -> PathBuf {
    if let Some(path) = std::env::var_os("FOXMORROW_BIN") {
        return PathBuf::from(path);
    }
    let launcher = plugin_root.join("bin").join(if cfg!(windows) { "foxmorrow.cmd" } else { "foxmorrow" });
    if launcher.is_file() {
        return launcher;
    }
    std::env::current_exe().unwrap_or_else(|_| PathBuf::from("foxmorrow"))
}
```

- [ ] **Шаг 7.** Перегенерировать роли и проверить:

```sh
cargo run -q -p foxmorrow -- --plugin-root . sync --plugin
cargo test --workspace
```

Ожидается: `sync` переписывает 21 файл, потому что изменился `_common.md`; все тесты пройдены.

- [ ] **Шаг 8.** Коммит:

```sh
git add crates templates roles agents
git commit -m "Pass the absolute foxmorrow path to subagents in briefs"
```

## Задача 3. Источники критериев `init`

**Файлы:** изменить `crates/foxmorrow/src/main.rs`, `tests/cli.rs`.

- [ ] **Шаг 1.** Тесты. В `crates/foxmorrow/tests/cli.rs` в `Setup::new` после записи `schema-maps/devin.json` добавить:

```rust
        write(
            plugin,
            "skills/fox/references/doc-flow.md",
            "# Doc Flow\n\n## PRD\n\n### PRD acceptance criteria\n\n- Scope is coherent\n- Status reflects facts\n\n## BRD\n\n### BRD acceptance criteria\n\n- Problem is stated\n",
        );
```

В конец файла добавить:

```rust
fn document_text(status: &str, based_on: &str, body: &str) -> String {
    format!("# Doc\n\nDocument status: {status}\nBased on: {based_on}\n\n{body}\n")
}

#[test]
fn init_plan_requires_an_approved_chain() {
    let setup = Setup::new();
    let root = setup.project.path();
    write(root, "docs/prd.md", &document_text("approved", "", "## Scope\n\n- health"));
    write(root, "docs/plan.md", &document_text("approved", "[prd](prd.md)", "## Критерии приёмки\n\n- Health returns 200\n- Tests pass"));
    let init = json(&setup.run(&["init", "--plan", "docs/plan.md", "--paths", "src/"], None));
    assert_eq!(init["criteria"][1]["text"], "Tests pass");
    assert_eq!(json(&setup.run(&["status"], None))["criteria_path"], "docs/plan.md");

    write(root, "docs/prd.md", &document_text("draft", "", "## Open questions\n\n- Which port?"));
    let refused = setup.run(&["init", "--plan", "docs/plan.md", "--paths", "src/"], None);
    failure(&refused, "prd.md has status draft");
    assert!(String::from_utf8_lossy(&refused.stderr).contains("prd.md has open questions"));

    write(root, "docs/en.md", &document_text("approved", "", "## Acceptance criteria\n\n- Works"));
    assert_eq!(json(&setup.run(&["init", "--plan", "docs/en.md", "--paths", "src/"], None))["criteria"][0]["text"], "Works");
    write(root, "docs/none.md", &document_text("approved", "", "## Notes\n\n- nothing"));
    failure(&setup.run(&["init", "--plan", "docs/none.md", "--paths", "src/"], None), "Критерии приёмки");
}

#[test]
fn init_doc_uses_type_criteria_and_checks_the_basis() {
    let setup = Setup::new();
    let root = setup.project.path();
    let brd = json(&setup.run(&["init", "--doc", "brd", "--paths", "docs/brd.md"], None));
    assert_eq!(brd["criteria"][0]["text"], "Problem is stated");

    failure(&setup.run(&["init", "--doc", "prd", "--paths", "docs/prd.md"], None), "--based-on");
    write(root, "docs/brd.md", &document_text("draft", "", "text"));
    failure(&setup.run(&["init", "--doc", "prd", "--based-on", "docs/brd.md", "--paths", "docs/prd.md"], None), "has status draft");
    write(root, "docs/brd.md", &document_text("approved", "", "text"));
    let prd = json(&setup.run(&["init", "--doc", "prd", "--based-on", "docs/brd.md", "--paths", "docs/prd.md"], None));
    assert_eq!(prd["criteria"][0]["text"], "Scope is coherent");
    let status = json(&setup.run(&["status"], None));
    assert!(status["criteria_path"].as_str().unwrap().ends_with("skills/fox/references/doc-flow.md"));
}

#[test]
fn init_needs_exactly_one_criteria_source() {
    let setup = Setup::new();
    assert_eq!(setup.run(&["init", "--paths", "src/"], None).status.code(), Some(2));
    let both = setup.run(&["init", "--criteria", "task.md#Критерии приёмки", "--doc", "brd", "--paths", "src/"], None);
    assert_eq!(both.status.code(), Some(2));
    assert_eq!(setup.run(&["init", "--based-on", "x.md", "--criteria", "task.md#Критерии приёмки", "--paths", "src/"], None).status.code(), Some(2));
}
```

- [ ] **Шаг 2.** Запустить: `cargo test -p foxmorrow --test cli`. Ожидается: новые тесты падают, потому что у `init` нет `--plan`, `--doc` и `--based-on`.

- [ ] **Шаг 3.** `crates/foxmorrow/src/main.rs`. Добавить импорт `use foxmorrow_core::document;` и изменить импорт clap на `use clap::{ArgGroup, Parser, Subcommand, ValueEnum};`. Вариант `Init` в `Command` заменить на:

```rust
    #[command(group(ArgGroup::new("source").required(true).multiple(false)))]
    Init {
        #[arg(long, group = "source")]
        criteria: Option<String>,
        #[arg(long, group = "source")]
        plan: Option<PathBuf>,
        #[arg(long, group = "source", value_enum)]
        doc: Option<DocType>,
        #[arg(long = "based-on", num_args = 1.., requires = "doc")]
        based_on: Vec<PathBuf>,
        #[arg(long = "paths", required = true, num_args = 1..)]
        paths: Vec<String>,
        #[arg(long = "repo")]
        repos: Vec<PathBuf>,
        #[arg(long)]
        session_id: Option<String>,
    },
```

После перечисления `Command` добавить:

```rust
#[derive(Clone, Copy, ValueEnum)]
enum DocType {
    Brd,
    Adr,
    Prd,
    Plan,
}

impl DocType {
    fn heading(self) -> &'static str {
        match self {
            DocType::Brd => "BRD acceptance criteria",
            DocType::Adr => "ADR acceptance criteria",
            DocType::Prd => "PRD acceptance criteria",
            DocType::Plan => "Plan acceptance criteria",
        }
    }
}

struct CriteriaSource {
    path: String,
    heading: String,
    markdown: String,
}
```

В ветке `Command::Init` заменить заголовок сопоставления и две строки чтения критериев:

```rust
        Command::Init { criteria: reference, plan, doc, based_on, paths, repos, session_id } => {
            let root = project::for_init(cli.project, &cwd);
            let plugin = Plugin::load(cli.plugin_root)?;
            let source = criteria_source(&root, &plugin.root, reference, plan, doc, &based_on)?;
```

Удалить строки `let (file, heading) = criteria::split_reference(&reference)?;` и `let markdown = std::fs::read_to_string(root.join(file))?;`. В литерале `NewRun` поля критериев заменить на:

```rust
                criteria_path: source.path,
                criteria_heading: source.heading,
                criteria_markdown: source.markdown,
```

В конец файла добавить:

```rust
fn criteria_source(
    root: &Path,
    plugin_root: &Path,
    reference: Option<String>,
    plan: Option<PathBuf>,
    doc: Option<DocType>,
    based_on: &[PathBuf],
) -> Result<CriteriaSource> {
    if let Some(reference) = reference {
        let (file, heading) = criteria::split_reference(&reference)?;
        return Ok(CriteriaSource {
            path: file.to_string(),
            heading: heading.to_string(),
            markdown: std::fs::read_to_string(root.join(file))?,
        });
    }
    if let Some(plan) = plan {
        let path = root.join(&plan);
        document::check_chain(std::slice::from_ref(&path))?;
        let markdown = std::fs::read_to_string(&path)?;
        let Some(heading) = ["Критерии приёмки", "Acceptance criteria"]
            .into_iter()
            .find(|heading| criteria::parse_section(&markdown, heading).is_ok())
        else {
            return invalid(format!(
                "{} has no «Критерии приёмки» or «Acceptance criteria» section with list items",
                plan.display()
            ));
        };
        return Ok(CriteriaSource { path: plan.display().to_string(), heading: heading.to_string(), markdown });
    }
    let Some(doc) = doc else {
        return invalid("one of --criteria, --plan or --doc is required");
    };
    if based_on.is_empty() && !matches!(doc, DocType::Brd) {
        return invalid("this document type needs --based-on with its approved source documents");
    }
    let basis: Vec<PathBuf> = based_on.iter().map(|path| root.join(path)).collect();
    document::check_chain(&basis)?;
    let file = plugin_root.join("skills").join("fox").join("references").join("doc-flow.md");
    Ok(CriteriaSource {
        markdown: std::fs::read_to_string(&file)
            .map_err(|_| Error::NotFound(format!("Doc Flow reference {}", file.display())))?,
        path: file.display().to_string(),
        heading: doc.heading().to_string(),
    })
}
```

- [ ] **Шаг 4.** Запустить: `cargo test -p foxmorrow --test cli`. Ожидается: все тесты CLI пройдены, включая 3 новых. Ошибки clap (нет источника, два источника, `--based-on` без `--doc`) завершаются кодом 2, как и ошибки `foxmorrow`.

- [ ] **Шаг 5.** Коммит:

```sh
git add crates/foxmorrow
git commit -m "Take run criteria from an approved plan or a Doc Flow type"
```

## Задача 4. Skill `fox` и справочники

Это тексты для агентов, поэтому `humanizer-ru` и `ai-slops-hunter` к ним не применяются. Порядок Doc Flow и критерии PRD, ADR и плана перенесены из Agent Flow `882b794` (`references/flows.md`, `references/implementation-plan-authoring.md`) без механизмов, которых нет в foxmorrow: документов только в чате, `verification_only`, отдельных селекторов guard. Порядок и критерии BRD написаны заново.

**Файлы:** создать `skills/fox/SKILL.md`, `skills/fox/references/orchestrator.md`, `skills/fox/references/doc-flow.md`, `skills/fox/references/contract.md`.

- [ ] **Шаг 1.** `skills/fox/SKILL.md`:

````markdown
---
name: fox
description: Runs a task through foxmorrow when the user explicitly asks with $fox or $foxmorrow, or invokes /foxmorrow:fox. Root coordinates role subagents and accepts each result only through the foxmorrow journal and Devin session evidence. Do not use without that explicit request.
---

# foxmorrow root

You are root, the Devin session that talks to the user. You coordinate the run: you create assignments, start role subagents, verify their reports and close the run. Role subagents author and check the work. You do not author product files or documents yourself, and you do not accept a result that `foxmorrow verify` has not accepted.

## Locate the CLI

The plugin root is two directories above this file, which lives at `<plugin root>/skills/fox/SKILL.md`. Run the CLI as `<plugin root>/bin/foxmorrow` and use that absolute path in every command. Briefs already pass the same path to subagents. `<plugin root>/bin/foxmorrow doctor` shows the plugin root, missing required skills and stale role files.

## Choose the flow

| Request | Start the run with |
|---|---|
| Implement an approved Implementation Plan | `foxmorrow init --plan <plan> --paths <paths…>` |
| Create or substantively revise a BRD, ADR, PRD or Implementation Plan | `foxmorrow init --doc <brd\|adr\|prd\|plan> [--based-on <documents…>] --paths <document>` |
| Any other task that already has a list of acceptance criteria | `foxmorrow init --criteria <file>#<heading> --paths <paths…>` |

If the task has no written acceptance criteria, agree them with the user, write them as a list under a heading in a task file inside the project, and start with `--criteria`. One run covers one task, one document or one plan.

Then follow [the orchestration procedure](references/orchestrator.md). For documents also follow [Doc Flow](references/doc-flow.md). Assignments, checks and reports use [the contract](references/contract.md).

## Hard rules

- Pass every brief to `run_subagent` verbatim, with the profile that `foxmorrow assign` printed, in the foreground.
- A role subagent sees only its brief. Put everything it needs into the assignment: the goal, the paths and the dependencies.
- Accepted reports never change. A correction is a new assignment with `--previous`.
- Do not commit, push, publish or deploy without the user's explicit permission.
- Set `Document status: approved` only after the user approves the verified document.
- When a missing decision changes scope or correctness, ask the user instead of guessing.
````

- [ ] **Шаг 2.** `skills/fox/references/orchestrator.md`:

````markdown
# Orchestration procedure

`foxmorrow` below means the absolute CLI path from `SKILL.md`.

## Start

1. Run the `init` command chosen in `SKILL.md`. It refuses a plan or a document basis that is not approved or has open questions; report that to the user and stop.
2. Read the `init` output:
   - `criteria` are the acceptance criteria of the run; every later report covers them by ID;
   - `missing_skills` lists required skills that are not installed; ask the user whether to install them or continue without them;
   - `root_session_id` and `root_model` empty means the session evidence is not linked yet; `verify` will retry, but tell the user if it stays empty.
3. Decide the assignments before starting them: authors with disjoint allowed paths, then one review, then QA. Use `architect` or `product-manager` first only when the task needs a decision they own.

## Assignments

1. Create each assignment with `foxmorrow assign --role <role> --goal "<goal>" --paths <paths…> [--depends-on <ids…>] [--previous <id>]`. The goal names the result and points to the sections of the plan, PRD or ADR to follow; do not copy those documents into it.
2. `assign` prints the brief on stdout and `profile=… model=…` on stderr. Start `run_subagent` with that profile and the brief as the task, verbatim, in the foreground. Authors may need permissions; ask the user before starting background work.
3. When the subagent returns, run `foxmorrow verify -a <id>`.
   - `accepted`: continue.
   - `rejected`: read the failed claims, then create a new assignment with `--previous <id>` for the right owner.
   - `unverified`: the evidence is missing; check `foxmorrow doctor` and the stderr reason, then retry `verify` or tell the user what cannot be proven.
4. After all authors are accepted, assign `reviewer` with `--depends-on` on every author assignment. After an accepted review with outcome `passed`, assign `qa-verifier` with `--depends-on` on the review.
5. `foxmorrow status` shows the run at any time. After context compaction, read it before acting.

## Routing findings

| Finding | Next assignment |
|---|---|
| Implementation defect | the author role, `--previous` on the author assignment |
| Architecture problem | `architect` |
| Disputed architectural choice | `supervising-architect` |
| Insufficient or doubtful verification | the verifier again, or `senior-qa-verifier` for a QA adequacy dispute |
| Missing information or a manual action | ask the user |

After a correction, repeat the affected review and QA with new assignments.

## Close

1. Close the run with `foxmorrow finalize --report <qa assignment>` after an accepted QA report.
2. If the run cannot reach a passing QA, create a root assignment with `foxmorrow assign --root --paths <paths…>`, submit a root report with `outcome` `failed` or `unverified`, a `reason` and the `unverified` criterion IDs, verify it, and finalize with that report.
3. Tell the user the final outcome, the root model from the final record and what remains unverified.
4. If a plan or ADR turns out wrong during implementation, stop the dependent work, finalize the run as not passed and propose a separate Doc Flow run to revise the document.
````

- [ ] **Шаг 3.** `skills/fox/references/doc-flow.md`:

````markdown
# Doc Flow

Documents are prepared with the same run mechanics as code; the product of the run is the document. Each document is created or substantively revised in its own run, started with `foxmorrow init --doc <type> --based-on <documents…> --paths <document>`.

## Document chain

```
BRD → ADR → PRD (one per feature) ─┬→ Implementation Plan → implementation
                                   └→ ADR (architecture changes) → Implementation Plan → implementation
```

Every document starts with a header:

```
Document status: draft
Based on: [title](relative/path.md), …
```

The first ADR is based on the BRD, a PRD on the BRD and the ADR, a feature ADR on its PRD, a plan on its PRD and, if present, on the feature ADR. A BRD may have no basis.

## Document approval

A document cannot have status `approved` while it contains an open question, whatever its importance. A question is closed when a justified answer or a decision of an authorized participant is recorded. Deleting the question, hiding it in another section or replacing it with an unconfirmed assumption does not close it.

Drafts may contain open questions; list them under a heading `Open questions`. An action that is planned but not done yet is not an open question when the decision, owner and conditions are defined. An unresolved choice, an unknown contract or an unconfirmed premise stays an open question.

Only the current revision with status `approved` and no open questions can be the basis for further documents or implementation. `foxmorrow init` checks the whole `Based on` chain. A review or QA verdict is `PASS` or `FAIL`; neither approves the document. Root sets `approved` only after the user approves the verified document.

## Routing

Root coordinates the document run: it chooses roles, creates assignments, passes the current text and the previous verdict, routes findings to the owner and decides the next step. Authors write content; reviewer and QA accept it independently. The order for every type:

1. The author writes the document and, for Russian text, applies `humanizer-ru`.
2. A separate `ai-slops-hunter` assignment edits the whole document and preserves decisions, numbers, conditions and technical syntax.
3. An independent `reviewer` checks the document against its sources and its acceptance criteria, including that the editing preserved the meaning.
4. After `PASS`, an independent `qa-verifier` checks coverage and readiness.
5. Root finalizes the run and gives the user the verified document with its status and any open questions.

A material change after review repeats the editing and the affected checks. A missing required role or skill leaves its step not done; the author's self-check does not replace independent acceptance.

## BRD

`product-manager` writes the BRD: the business problem, target users or segments, value, success metrics, scope and out-of-scope, constraints such as budget, timeline and compliance, risks and labelled assumptions. When market data is needed, root first assigns `marketing-growth-strategist` and gives the author its accepted report through `--depends-on`.

### BRD acceptance criteria

- The business problem, the target users and the value are stated and consistent.
- Success metrics are measurable and tied to the problem.
- Scope and out-of-scope are explicit; constraints are listed with their source.
- Market claims rest on the assigned market input or are labelled as assumptions.
- Risks and assumptions are labelled; status and open questions reflect the facts.

## PRD

`product-manager` writes the PRD for one coherent block of functionality, even when the requirements are already settled. Independent capabilities become sequential PRDs with their own results and order. The PRD explains the goal and context, the user scenario with the expected behavior, constraints, boundaries and acceptance conditions. Each requirement has one main place. It keeps significant scenarios, permissions, errors, business constraints, data safety and measurable outcomes, and leaves architecture, protocols and verification methods to the ADR and the plan.

After a PRD is approved, root assigns `architect` in the PRD run to decide whether a feature ADR is needed. The decision and its reason are part of that accepted report. When the architecture changes and there is no new ADR, the plan cannot be approved.

### PRD acceptance criteria

- The scope is one coherent block of functionality with a clear result.
- The goal, context, user scenario with expected behavior, constraints, boundaries and acceptance conditions are present; each requirement has one main place.
- Significant scenarios, permissions, errors, business constraints, data safety and measurable outcomes are preserved; implementation design is left to the ADR and the plan.
- `Based on` links the BRD and the ADR; status and open questions reflect the facts.

## ADR

`architect` writes the ADR for a significant decision or closely related decisions that must be assessed together; independent decisions get separate ADRs. The ADR states the problem and source requirements, constraints, the chosen decision, significant alternatives and consequences, the responsibility of components, data owners and contracts. Each decision has one main place; decisions for one service stay in its section. The ADR is not a list of files or tasks.

The architect applies `arch-simplicity-guard` to the draft. A disputed choice goes to `supervising-architect` at once, through root; it decides within the assignment and updates the ADR itself. The reviewer applies the guard again in its own session.

### ADR acceptance criteria

- The decision is tied to the source requirements and confirmed constraints; assumptions and unknowns are labelled.
- The choice, its scope, the responsibility of affected components and significant contracts are defined unambiguously.
- Applicable alternatives are considered; the reasons and the significant positive and negative consequences are explained.
- Requirements, risks, guarantees and verifiable invariants are preserved; there are no unnecessary mechanisms or contradictions with current decisions.
- The structure is logical and the ADR is not replaced by a plan of file changes; status and open questions reflect the facts.

## Plan

`architect` writes the Implementation Plan from the approved PRD and ADRs and the current code, even when the approach is already chosen. A contradiction with an ADR goes back to the ADR process through root; a new decision is not hidden in a task. The plan separates local development from external setup and live integration checks, each with its own acceptance criteria; local work and its QA do not depend on external accounts, credentials or remote CI. A new stage is justified only by a real dependency, a contract change, a data migration or a separately verifiable result.

The plan has a section `Критерии приёмки` or `Acceptance criteria` with a list; `foxmorrow init --plan` takes the run criteria from it. External setup and live checks are separate sections or separate plans so that they get their own runs.

The architect applies `arch-simplicity-guard`; disputed choices go to `supervising-architect` as for the ADR. Before the final review, a separate `qa-verifier` assignment reviews the requirements coverage of the draft.

### Plan acceptance criteria

- The scope and stages implement the relevant requirements and accepted decisions; nothing is lost and no ADR is revised silently.
- Each stage has a result, a change area, responsible roles, real dependencies, criteria and sufficient checks.
- Local development has its own acceptance without external credentials or service availability; external prerequisites, owners and live checks are defined separately.
- The plan matches the current code and known contracts, keeps significant risks and adds no unjustified mechanisms or procedures.
- `Based on` links the PRD and, if present, the feature ADR; status and open questions reflect the facts.
````

- [ ] **Шаг 4.** `skills/fox/references/contract.md`:

````markdown
# Assignment contract

`foxmorrow` below means the absolute CLI path from the brief or from `SKILL.md`.

## Checks

`foxmorrow check -a <assignment> -- <command> [args…]` runs the command once in the current directory and records a receipt: cwd, argv, exit code, time, a hash and the tail of the output. It exits with the command's exit code. Reference the printed `chk-…` ID as evidence. If a check was interrupted, inspect what it actually did before running it again.

## Report

Each assignment ends with exactly one `foxmorrow report -a <assignment>` call that reads JSON from stdin:

```json
{
  "summary": "what was done and found",
  "status": "completed",
  "outcome": "passed",
  "criteria": {"ac-1": {"status": "passed", "reason": "why", "evidence": ["chk-001"]}},
  "evidence": ["chk-001", "<tool call id>"],
  "resolves": ["a-03"]
}
```

- `summary` is a non-empty string.
- `status` is `completed` or `interrupted`; interrupted work cannot have outcome `passed`.
- `outcome` and each criterion `status` are `passed`, `failed` or `unverified`.
- `criteria` is required for QA and covers every run criterion; other roles include the criteria they address.
- `evidence` references `chk-…` receipts of the same assignment, with exit code 0 for passed claims, or IDs of your own tool calls.
- `resolves` lists earlier negative reports that this report supersedes.
- A root stop report adds `reason` and an `unverified` list of criterion IDs.

A submitted report cannot be changed.

## Next action

Root reads `foxmorrow verify -a <assignment>`: `accepted`, `rejected` with the failed claims, or `unverified` when the session evidence is missing. A completed report with outcome `failed` does not end the task; root chooses the next assignment from the findings by the routing table in `orchestrator.md`.
````

- [ ] **Шаг 5.** Коммит:

```sh
git add skills
git commit -m "Add the fox skill with orchestration, Doc Flow and contract references"
```

## Задача 5. Проверки skill в репозитории

**Файлы:** изменить `crates/foxmorrow-devin/tests/repository.rs`.

- [ ] **Шаг 1.** В импорты добавить `use foxmorrow_core::criteria;`. В конец файла добавить:

```rust
fn slug(heading: &str) -> String {
    heading
        .trim()
        .to_lowercase()
        .chars()
        .filter_map(|c| match c {
            ' ' => Some('-'),
            c if c.is_alphanumeric() || c == '-' || c == '_' => Some(c),
            _ => None,
        })
        .collect()
}

fn anchors(markdown: &str) -> BTreeSet<String> {
    markdown
        .lines()
        .filter_map(|line| {
            let depth = line.bytes().take_while(|byte| *byte == b'#').count();
            ((1..=6).contains(&depth)).then(|| line[depth..].trim()).filter(|title| !title.is_empty())
        })
        .map(slug)
        .collect()
}

#[test]
fn skill_has_frontmatter_and_existing_references() {
    let root = root();
    let skill = std::fs::read_to_string(root.join("skills/fox/SKILL.md")).unwrap();
    let frontmatter = skill.strip_prefix("---\n").and_then(|rest| rest.split_once("\n---\n")).unwrap().0;
    assert!(frontmatter.lines().any(|line| line == "name: fox"));
    assert!(frontmatter.lines().any(|line| line.strip_prefix("description: ").is_some_and(|text| !text.trim().is_empty())));
    for reference in ["orchestrator.md", "doc-flow.md", "contract.md"] {
        assert!(skill.contains(&format!("references/{reference}")), "SKILL.md does not link {reference}");
        assert!(root.join("skills/fox/references").join(reference).is_file());
    }
}

#[test]
fn role_links_point_to_existing_doc_flow_sections() {
    let root = root();
    let doc_flow = anchors(&std::fs::read_to_string(root.join("skills/fox/references/doc-flow.md")).unwrap());
    for role in ROLES {
        let text = catalog::role_text(&root, role).unwrap();
        for (index, _) in text.match_indices("references/doc-flow.md#") {
            let anchor: String = text[index + "references/doc-flow.md#".len()..]
                .chars()
                .take_while(|c| c.is_alphanumeric() || *c == '-' || *c == '_')
                .collect();
            assert!(doc_flow.contains(&anchor), "{role} links to a missing doc-flow section #{anchor}");
        }
    }
}

#[test]
fn every_document_type_has_parsable_criteria() {
    let markdown = std::fs::read_to_string(root().join("skills/fox/references/doc-flow.md")).unwrap();
    for kind in ["BRD", "ADR", "PRD", "Plan"] {
        let (_, found) = criteria::parse_section(&markdown, &format!("{kind} acceptance criteria")).unwrap();
        assert!(found.len() >= 4, "{kind} has {} criteria", found.len());
    }
}
```

- [ ] **Шаг 2.** Запустить: `cargo test -p foxmorrow-devin --test repository`. Ожидается: 8 тестов пройдены. Если `role_links_point_to_existing_doc_flow_sections` падает, исправить заголовок в `doc-flow.md`, а не ссылку в роли.

- [ ] **Шаг 3.** Коммит:

```sh
git add crates/foxmorrow-devin
git commit -m "Check the fox skill, Doc Flow anchors and criteria sections"
```

## Задача 6. Итоговая проверка

- [ ] **Шаг 1.**

```sh
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cargo run -q -p foxmorrow -- --plugin-root . sync --plugin --check
rg -n '^\s*//' crates --glob '*.rs'
```

Ожидается: первые четыре команды завершаются с кодом 0, последняя ничего не находит.

- [ ] **Шаг 2.** Проверить цепочку документов этого репозитория:

```sh
cargo run -q -p foxmorrow -- --plugin-root . --project /tmp/foxmorrow-chain-check init --plan "$PWD/docs/implementation/impl-004-skill-doc-flow.md" --paths src/
```

Ожидается, что цепочка `impl-004` → дизайн, дорожная карта, планы 002 и 003 проходит проверку: все документы утверждены, открытых вопросов нет. Команда создаёт run во временном каталоге `/tmp/foxmorrow-chain-check`, который после проверки удаляется.

- [ ] **Шаг 3.** Сверить реализацию с разделом «Критерии приёмки»: для каждого пункта назвать тест или команду, которые его подтверждают.

- [ ] **Шаг 4.** Коммит исправлений, если они были:

```sh
git add -A crates skills templates roles agents Cargo.lock
git commit -m "Fix formatting and lint findings"
```

## Что остаётся непроверенным после плана

* Как root в живом Devin следует skill `fox` и находит путь к плагину (план 005).
* Что Devin показывает путь к файлу skill при вызове `/foxmorrow:fox`. Если не показывает, путь даёт `foxmorrow doctor`, а загрузчик из плана 005 задаёт `FOXMORROW_PLUGIN_ROOT`.
