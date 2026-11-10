# План 005. Дистрибуция

Document status: approved
Based on: [foxmorrow v1: дизайн](../design/2026-11-10-foxmorrow-v1-design.md), [дорожная карта](roadmap-v1.md), [план 002](impl-002-devin-evidence.md), [план 004](impl-004-skill-doc-flow.md)

Задачи выполняются по порядку, шаги отмечаются чекбоксами `- [ ]`. Шаги «Коммит» выполняются, только если пользователь разрешил коммиты в этом run.

**Цель:** foxmorrow ставится командой `devin plugins install svishniakov/foxmorrow` и работает без сборки у пользователя. Загрузчик скачивает бинарник своей версии из GitHub Releases и сверяет SHA-256. CI проверяет код на трёх ОС, выпуск по тегу собирает пять бинарников.

**Архитектура:**
* Корень репозитория остаётся плагином Devin. Добавляются манифест `.devin-plugin/plugin.json`, `hooks.json`, `checksums.json` и загрузчики `bin/foxmorrow` (POSIX sh), `bin/foxmorrow.cmd` с `bin/foxmorrow.ps1` (Windows).
* Загрузчик задаёт `FOXMORROW_PLUGIN_ROOT`, поэтому бинарник из кеша находит каталог плагина.
* `scripts/write-checksums.sh` формирует `checksums.json` из собранных файлов. Его используют выпуск в CI и тест загрузчика.
* Выпуск по тегу собирает бинарники, создаёт GitHub Release и коммитит `checksums.json` в `main`.

**Стек:** POSIX sh, PowerShell 5.1, GitHub Actions. В Rust новых зависимостей нет.

План разделён на две части с отдельными критериями приёмки, как требует [Doc Flow](../../skills/fox/references/doc-flow.md#plan). **Локальная разработка** (задачи с 1 по 6) не требует доступа к GitHub. **Внешняя настройка и живые проверки** (задачи с 7 по 10) выполняются после разрешений пользователя: push, выпуск, установка плагина.

## Решения плана, уточняющие дизайн

| Вопрос | Решение | Основание |
|---|---|---|
| `requiredPlugins` для `arch-simplicity-guard` | Не указывается. Guard ставится как обычный skill в `~/.agents/skills` и применяется ролями как обычный skill: он записан в `required` у `architect`, `supervising-architect` и `reviewer`, а `assign` и `doctor` проверяют его наличие. README объясняет установку | Решение пользователя. Репозиторий `svishniakov/arch-simplicity-guard` не плагин Devin: `SKILL.md` лежит в корне, манифеста нет |
| Загрузчик Windows | `bin/foxmorrow.cmd` вызывает `bin/foxmorrow.ps1` через `powershell` 5.1 | В cmd нет средств для загрузки и SHA-256; PowerShell 5.1 входит в Windows 10 с версии 1607 и в Windows 11 |
| Формат ассетов | Отдельный файл на цель: `foxmorrow-<target>`, `foxmorrow-x86_64-pc-windows-msvc.exe`, без архивов | Загрузчику не нужен распаковщик |
| Кеш | `${XDG_CACHE_HOME:-$HOME/.cache}/foxmorrow/<version>/<target>/`, на Windows `%LOCALAPPDATA%\foxmorrow\<version>\<target>\` | Дизайн §9.1 |
| Режим разработки | `FOXMORROW_DEV_BINARY` запускает указанный бинарник без загрузки; `FOXMORROW_RELEASE_BASE` заменяет адрес выпусков | Первое позволяет проверить плагин до первого выпуска, второе нужно тестам |
| `checksums.json` между тегом и выпуском | Версию в `Cargo.toml`, манифесте и `checksums.json` поднимают до тега, список бинарников пуст. Выпуск заполняет его и коммитит в `main` | Пока выпуск не завершён, загрузчик сообщает, что бинарника для версии нет |
| Hooks | Только `UserPromptSubmit` и `PostCompaction` | План 002 отказался от записи `PostToolUse` |
| Перевод строк | `.gitattributes` с `eol=lf` | Без этого checkout в Windows получает CRLF, и `sync --check` расходится с `agents/` |
| Actions | Закреплены по SHA: `actions/checkout` v7.0.1 `3d3c42e5aac5ba805825da76410c181273ba90b1`, `actions/upload-artifact` v7.0.2 `cf430e030ddbb5b0abf93d22962f4752f3646cd9`, `actions/download-artifact` v8.0.2 `9000827ccba6bdab643e8b6fd33ac0654aef8333` | Последние выпуски на 10 ноября 2026 года, старше 7 дней |
| Видимость репозитория | Публичный | Загрузчик скачивает выпуск без авторизации |

## Критерии приёмки

* `cargo fmt --all --check`, `cargo clippy --workspace --all-targets -- -D warnings` и `cargo test --workspace` завершаются успешно на macOS.
* `bin/foxmorrow` скачивает бинарник своей платформы, сверяет SHA-256, кеширует его и запускает с аргументами и `FOXMORROW_PLUGIN_ROOT`. При несовпадении хеша, отсутствии цели в `checksums.json` или сбое загрузки он завершается с кодом 2, понятным сообщением и без файла в кеше. `FOXMORROW_DEV_BINARY` запускает указанный бинарник без загрузки.
* `scripts/write-checksums.sh` создаёт корректный JSON для пустого и непустого набора бинарников.
* Версии в `Cargo.toml`, `.devin-plugin/plugin.json` и `checksums.json` совпадают; цели в `checksums.json` входят в пять поддерживаемых, хеши имеют вид 64 шестнадцатеричных символов.
* `hooks.json` вызывает `${PLUGIN_ROOT}/bin/foxmorrow hook <event>` только для `UserPromptSubmit` и `PostCompaction`.
* Файлы `bin/foxmorrow` и `scripts/write-checksums.sh` исполняемые.
* Workflows `ci.yml` и `release.yml` проходят разбор YAML и используют закреплённые SHA actions.
* README описывает установку, зависимости, три потока `$fox`, переопределения моделей, что foxmorrow доказывает и чего не доказывает, ограничения v1, диагностику и выпуск.
* В коде нет поясняющих комментариев.

## Критерии приёмки внешней части

* CI проходит на `ubuntu-latest`, `macos-latest` и `windows-latest` для коммита в `main`.
* Выпуск `v0.1.0` содержит пять бинарников и `checksums.json`; `checksums.json` в `main` совпадает с выпуском.
* `devin plugins install --local svishniakov/foxmorrow` устанавливает плагин; `devin plugins info foxmorrow` показывает 21 роль, skill `fox` и два hooks.
* В живой сессии Devin на тестовом проекте: `$fox` записывает `.foxmorrow/session` и вызывает skill; загрузчик скачивает бинарник; назначение автора проходит `verify` со статусом `accepted` по живой `sessions.db`; run закрывается `finalize`.
* Скачанный загрузчиком бинарник на macOS запускается, атрибут `com.apple.quarantine` у него отсутствует.
* Результаты живых проверок записаны в раздел «Результаты внешней части» этого плана.

## Структура файлов

```
.devin-plugin/plugin.json
.gitattributes
hooks.json
checksums.json
bin/foxmorrow
bin/foxmorrow.cmd
bin/foxmorrow.ps1
scripts/write-checksums.sh
.github/workflows/ci.yml
.github/workflows/release.yml
README.md
crates/foxmorrow/tests/loader.rs
crates/foxmorrow/tests/loader_windows.rs
crates/foxmorrow-devin/tests/repository.rs
```

## Задача 1. Скрипт контрольных сумм и загрузчик POSIX

**Файлы:** создать `scripts/write-checksums.sh`, `bin/foxmorrow`, `crates/foxmorrow/tests/loader.rs`.

- [ ] **Шаг 1.** Тест `crates/foxmorrow/tests/loader.rs`:

```rust
#![cfg(unix)]

use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

const FAKE: &str = "#!/bin/sh\necho \"fake $* root=$FOXMORROW_PLUGIN_ROOT\"\n";

fn target() -> &'static str {
    match (std::env::consts::OS, std::env::consts::ARCH) {
        ("macos", "aarch64") => "aarch64-apple-darwin",
        ("macos", "x86_64") => "x86_64-apple-darwin",
        ("linux", "x86_64") => "x86_64-unknown-linux-musl",
        ("linux", "aarch64") => "aarch64-unknown-linux-musl",
        other => panic!("unsupported test platform {other:?}"),
    }
}

fn repo() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn executable(path: &Path, text: &str) {
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(path, text).unwrap();
    std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o755)).unwrap();
}

fn checksums(dir: &Path) -> String {
    let output = Command::new("sh")
        .arg(repo().join("scripts/write-checksums.sh"))
        .args(["9.9.9", "test/foxmorrow"])
        .arg(dir)
        .output()
        .unwrap();
    assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));
    String::from_utf8(output.stdout).unwrap()
}

struct Release {
    plugin: tempfile::TempDir,
    assets: tempfile::TempDir,
    cache: tempfile::TempDir,
}

impl Release {
    fn new() -> Self {
        let release = Release {
            plugin: tempfile::tempdir().unwrap(),
            assets: tempfile::tempdir().unwrap(),
            cache: tempfile::tempdir().unwrap(),
        };
        let loader = std::fs::read_to_string(repo().join("bin/foxmorrow")).unwrap();
        executable(&release.plugin.path().join("bin/foxmorrow"), &loader);
        executable(&release.asset(), FAKE);
        let json = checksums(&release.assets.path().join("v9.9.9"));
        std::fs::write(release.plugin.path().join("checksums.json"), json).unwrap();
        release
    }

    fn asset(&self) -> PathBuf {
        self.assets.path().join("v9.9.9").join(format!("foxmorrow-{}", target()))
    }

    fn cached(&self) -> PathBuf {
        self.cache.path().join("foxmorrow/9.9.9").join(target()).join("foxmorrow")
    }

    fn run(&self, args: &[&str]) -> Output {
        Command::new(self.plugin.path().join("bin/foxmorrow"))
            .args(args)
            .env("FOXMORROW_RELEASE_BASE", format!("file://{}", self.assets.path().display()))
            .env("XDG_CACHE_HOME", self.cache.path())
            .env_remove("FOXMORROW_DEV_BINARY")
            .output()
            .unwrap()
    }
}

fn failed(output: &Output, needle: &str) {
    assert_eq!(output.status.code(), Some(2), "{}", String::from_utf8_lossy(&output.stdout));
    assert!(String::from_utf8_lossy(&output.stderr).contains(needle), "{}", String::from_utf8_lossy(&output.stderr));
}

#[test]
fn writes_checksums_for_present_binaries() {
    let empty = tempfile::tempdir().unwrap();
    let value: serde_json::Value = serde_json::from_str(&checksums(empty.path())).unwrap();
    assert_eq!(value["version"], "9.9.9");
    assert_eq!(value["repository"], "test/foxmorrow");
    assert!(value["binaries"].as_object().unwrap().is_empty());

    let release = Release::new();
    let json = std::fs::read_to_string(release.plugin.path().join("checksums.json")).unwrap();
    let value: serde_json::Value = serde_json::from_str(&json).unwrap();
    let sha = value["binaries"][target()].as_str().unwrap();
    assert_eq!(sha, foxmorrow_core::util::sha256_hex(FAKE.as_bytes()));
}

#[test]
fn downloads_verifies_caches_and_runs() {
    let release = Release::new();
    let output = release.run(&["status", "--run", "r-1"]);
    assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.starts_with("fake status --run r-1 root="), "{stdout}");
    let plugin_name = release.plugin.path().file_name().unwrap().to_str().unwrap();
    assert!(stdout.trim_end().ends_with(plugin_name), "{stdout}");
    assert!(release.cached().is_file());

    std::fs::remove_file(release.asset()).unwrap();
    assert!(release.run(&["again"]).status.success());
}

#[test]
fn refuses_a_wrong_checksum_and_leaves_no_cache() {
    let release = Release::new();
    let path = release.plugin.path().join("checksums.json");
    let json = std::fs::read_to_string(&path).unwrap();
    let sha = foxmorrow_core::util::sha256_hex(FAKE.as_bytes());
    std::fs::write(&path, json.replace(&sha, &"0".repeat(64))).unwrap();
    failed(&release.run(&["status"]), "checksum mismatch");
    let dir = release.cached().parent().unwrap().to_path_buf();
    assert!(std::fs::read_dir(&dir).map(|entries| entries.count()).unwrap_or(0) == 0);
}

#[test]
fn reports_a_missing_release_or_download_failure() {
    let release = Release::new();
    std::fs::remove_file(release.asset()).unwrap();
    failed(&release.run(&["status"]), "download failed");

    let empty = tempfile::tempdir().unwrap();
    std::fs::write(release.plugin.path().join("checksums.json"), checksums(empty.path())).unwrap();
    failed(&release.run(&["status"]), "no released binary");
}

#[test]
fn dev_binary_skips_the_download() {
    let release = Release::new();
    let dev = release.plugin.path().join("dev-foxmorrow");
    executable(&dev, "#!/bin/sh\necho \"dev $*\"\n");
    let output = Command::new(release.plugin.path().join("bin/foxmorrow"))
        .arg("doctor")
        .env("FOXMORROW_DEV_BINARY", &dev)
        .env("FOXMORROW_RELEASE_BASE", "file:///nonexistent")
        .env("XDG_CACHE_HOME", release.cache.path())
        .output()
        .unwrap();
    assert_eq!(String::from_utf8(output.stdout).unwrap(), "dev doctor\n");
    assert!(!release.cached().exists());
}
```

- [ ] **Шаг 2.** Запустить: `cargo test -p foxmorrow --test loader`. Ожидается: тесты падают, потому что нет `scripts/write-checksums.sh` и `bin/foxmorrow`.

- [ ] **Шаг 3.** `scripts/write-checksums.sh`:

```sh
#!/bin/sh
set -eu

if [ "$#" -ne 3 ]; then
  echo "usage: write-checksums.sh <version> <owner/repo> <assets dir>" >&2
  exit 2
fi
version=$1
repository=$2
dir=$3

hash() {
  if command -v sha256sum >/dev/null 2>&1; then
    sha256sum "$1" | cut -d ' ' -f 1
  else
    shasum -a 256 "$1" | cut -d ' ' -f 1
  fi
}

printf '{\n  "version": "%s",\n  "repository": "%s",\n  "binaries": {' "$version" "$repository"
separator=""
for target in aarch64-apple-darwin x86_64-apple-darwin x86_64-unknown-linux-musl aarch64-unknown-linux-musl x86_64-pc-windows-msvc; do
  file="$dir/foxmorrow-$target"
  if [ "$target" = x86_64-pc-windows-msvc ]; then
    file="$file.exe"
  fi
  if [ -f "$file" ]; then
    printf '%s\n    "%s": "%s"' "$separator" "$target" "$(hash "$file")"
    separator=","
  fi
done
if [ -n "$separator" ]; then
  printf '\n  '
fi
printf '}\n}\n'
```

- [ ] **Шаг 4.** `bin/foxmorrow`:

```sh
#!/bin/sh
set -eu

here=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
root=$(dirname -- "$here")
FOXMORROW_PLUGIN_ROOT=$root
export FOXMORROW_PLUGIN_ROOT

if [ -n "${FOXMORROW_DEV_BINARY:-}" ]; then
  exec "$FOXMORROW_DEV_BINARY" "$@"
fi

fail() {
  echo "foxmorrow: $*" >&2
  exit 2
}

checksums="$root/checksums.json"
[ -f "$checksums" ] || fail "missing $checksums"
field() {
  sed -n "s/.*\"$1\": *\"\([^\"]*\)\".*/\1/p" "$checksums" | head -n 1
}

case "$(uname -s)-$(uname -m)" in
  Darwin-arm64) target=aarch64-apple-darwin ;;
  Darwin-x86_64) target=x86_64-apple-darwin ;;
  Linux-x86_64) target=x86_64-unknown-linux-musl ;;
  Linux-aarch64 | Linux-arm64) target=aarch64-unknown-linux-musl ;;
  *) fail "unsupported platform $(uname -s) $(uname -m)" ;;
esac

version=$(field version)
repository=$(field repository)
expected=$(field "$target")
[ -n "$expected" ] || fail "no released binary for $target in version $version; run devin plugins update or wait for the release"

cache="${XDG_CACHE_HOME:-$HOME/.cache}/foxmorrow/$version/$target"
binary="$cache/foxmorrow"
if [ ! -x "$binary" ]; then
  mkdir -p "$cache"
  partial="$cache/foxmorrow.download.$$"
  trap 'rm -f "$partial"' EXIT INT TERM
  base="${FOXMORROW_RELEASE_BASE:-https://github.com/$repository/releases/download}"
  url="$base/v$version/foxmorrow-$target"
  if command -v curl >/dev/null 2>&1; then
    curl -fsSL "$url" -o "$partial" || fail "download failed: $url"
  elif command -v wget >/dev/null 2>&1; then
    wget -q -O "$partial" "$url" || fail "download failed: $url"
  else
    fail "download failed: neither curl nor wget is available"
  fi
  if command -v sha256sum >/dev/null 2>&1; then
    actual=$(sha256sum "$partial" | cut -d ' ' -f 1)
  else
    actual=$(shasum -a 256 "$partial" | cut -d ' ' -f 1)
  fi
  [ "$actual" = "$expected" ] || fail "checksum mismatch for $url: expected $expected, got $actual"
  chmod +x "$partial"
  mv "$partial" "$binary"
  trap - EXIT INT TERM
fi

exec "$binary" "$@"
```

- [ ] **Шаг 5.** Сделать файлы исполняемыми: `chmod +x bin/foxmorrow scripts/write-checksums.sh`.

- [ ] **Шаг 6.** Запустить: `cargo test -p foxmorrow --test loader`. Ожидается: 5 тестов пройдены.

- [ ] **Шаг 7.** Коммит:

```sh
git add bin/foxmorrow scripts crates/foxmorrow/tests/loader.rs
git commit -m "Download, verify and cache the foxmorrow binary from the POSIX loader"
```

## Задача 2. Загрузчик Windows

Тест выполняется только в CI на `windows-latest`; локально на macOS он не компилируется.

**Файлы:** создать `bin/foxmorrow.cmd`, `bin/foxmorrow.ps1`, `crates/foxmorrow/tests/loader_windows.rs`.

- [ ] **Шаг 1.** Тест `crates/foxmorrow/tests/loader_windows.rs`:

```rust
#![cfg(windows)]

use std::path::PathBuf;
use std::process::Command;

fn repo() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

#[test]
fn downloads_verifies_and_runs_on_windows() {
    let plugin = tempfile::tempdir().unwrap();
    let assets = tempfile::tempdir().unwrap();
    let cache = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(plugin.path().join("bin")).unwrap();
    for name in ["foxmorrow.cmd", "foxmorrow.ps1"] {
        std::fs::copy(repo().join("bin").join(name), plugin.path().join("bin").join(name)).unwrap();
    }
    let release = assets.path().join("v9.9.9");
    std::fs::create_dir_all(&release).unwrap();
    let asset = release.join("foxmorrow-x86_64-pc-windows-msvc.exe");
    std::fs::copy(env!("CARGO_BIN_EXE_foxmorrow"), &asset).unwrap();
    let sha = foxmorrow_core::util::sha256_hex(&std::fs::read(&asset).unwrap());
    let write = |sha: &str| {
        std::fs::write(
            plugin.path().join("checksums.json"),
            format!("{{\n  \"version\": \"9.9.9\",\n  \"repository\": \"test/foxmorrow\",\n  \"binaries\": {{\n    \"x86_64-pc-windows-msvc\": \"{sha}\"\n  }}\n}}\n"),
        )
        .unwrap();
    };
    let base = format!("file:///{}", assets.path().display().to_string().replace('\\', "/"));
    let run = || {
        Command::new(plugin.path().join("bin/foxmorrow.cmd"))
            .arg("--version")
            .env("FOXMORROW_RELEASE_BASE", &base)
            .env("LOCALAPPDATA", cache.path())
            .env_remove("FOXMORROW_DEV_BINARY")
            .output()
            .unwrap()
    };

    write(&"0".repeat(64));
    let refused = run();
    assert_eq!(refused.status.code(), Some(2));
    assert!(String::from_utf8_lossy(&refused.stderr).contains("checksum mismatch"));

    write(&sha);
    let output = run();
    assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));
    assert!(String::from_utf8_lossy(&output.stdout).starts_with("foxmorrow "));
    assert!(cache.path().join("foxmorrow/9.9.9/x86_64-pc-windows-msvc/foxmorrow.exe").is_file());
}
```

- [ ] **Шаг 2.** `bin/foxmorrow.cmd`:

```bat
@echo off
powershell -NoProfile -ExecutionPolicy Bypass -File "%~dp0foxmorrow.ps1" %*
exit /b %ERRORLEVEL%
```

- [ ] **Шаг 3.** `bin/foxmorrow.ps1`:

```powershell
$ErrorActionPreference = 'Stop'
$root = Split-Path -Parent $PSScriptRoot
$env:FOXMORROW_PLUGIN_ROOT = $root

function Fail([string] $message) {
    [Console]::Error.WriteLine("foxmorrow: $message")
    exit 2
}

if ($env:FOXMORROW_DEV_BINARY) {
    & $env:FOXMORROW_DEV_BINARY @args
    exit $LASTEXITCODE
}

$checksumsPath = Join-Path $root 'checksums.json'
if (-not (Test-Path $checksumsPath)) { Fail "missing $checksumsPath" }
$checksums = Get-Content -Raw $checksumsPath | ConvertFrom-Json
if ($env:PROCESSOR_ARCHITECTURE -ne 'AMD64') { Fail "unsupported platform Windows $($env:PROCESSOR_ARCHITECTURE)" }
$target = 'x86_64-pc-windows-msvc'
$version = $checksums.version
$expected = $checksums.binaries.$target
if (-not $expected) { Fail "no released binary for $target in version $version; run devin plugins update or wait for the release" }

$cacheRoot = if ($env:LOCALAPPDATA) { $env:LOCALAPPDATA } else { Join-Path $HOME '.cache' }
$cache = Join-Path $cacheRoot "foxmorrow\$version\$target"
$binary = Join-Path $cache 'foxmorrow.exe'
if (-not (Test-Path $binary)) {
    New-Item -ItemType Directory -Force -Path $cache | Out-Null
    $partial = Join-Path $cache "foxmorrow.download.$PID.exe"
    $base = if ($env:FOXMORROW_RELEASE_BASE) { $env:FOXMORROW_RELEASE_BASE } else { "https://github.com/$($checksums.repository)/releases/download" }
    $url = "$base/v$version/foxmorrow-$target.exe"
    try {
        (New-Object System.Net.WebClient).DownloadFile($url, $partial)
    } catch {
        Remove-Item -ErrorAction SilentlyContinue $partial
        Fail "download failed: $url"
    }
    $stream = [System.IO.File]::OpenRead($partial)
    try {
        $digest = [System.Security.Cryptography.SHA256]::Create().ComputeHash($stream)
    } finally {
        $stream.Dispose()
    }
    $actual = -join ($digest | ForEach-Object { $_.ToString('x2') })
    if ($actual -ne $expected) {
        Remove-Item -ErrorAction SilentlyContinue $partial
        Fail "checksum mismatch for ${url}: expected $expected, got $actual"
    }
    Move-Item -Force $partial $binary
}

& $binary @args
exit $LASTEXITCODE
```

- [ ] **Шаг 4.** Проверить, что тест Windows не ломает сборку на macOS: `cargo test -p foxmorrow`. Ожидается: тест `loader_windows` не компилируется в набор (`0 tests`), остальные пройдены.

- [ ] **Шаг 5.** Коммит:

```sh
git add bin/foxmorrow.cmd bin/foxmorrow.ps1 crates/foxmorrow/tests/loader_windows.rs
git commit -m "Add the Windows loader"
```

## Задача 3. Манифест, hooks и контрольные суммы

**Файлы:** создать `.devin-plugin/plugin.json`, `hooks.json`, `checksums.json`, `.gitattributes`; изменить `crates/foxmorrow-devin/tests/repository.rs`.

- [ ] **Шаг 1.** Тесты. В конец `crates/foxmorrow-devin/tests/repository.rs` добавить:

```rust
const TARGETS: [&str; 5] = [
    "aarch64-apple-darwin",
    "x86_64-apple-darwin",
    "x86_64-unknown-linux-musl",
    "aarch64-unknown-linux-musl",
    "x86_64-pc-windows-msvc",
];

fn json_file(relative: &str) -> serde_json::Value {
    serde_json::from_str(&std::fs::read_to_string(root().join(relative)).unwrap()).unwrap()
}

#[test]
fn versions_agree_across_the_plugin() {
    let cargo = std::fs::read_to_string(root().join("Cargo.toml")).unwrap();
    let version = cargo
        .lines()
        .skip_while(|line| *line != "[workspace.package]")
        .find_map(|line| line.strip_prefix("version = \""))
        .map(|rest| rest.trim_end_matches('"').to_string())
        .unwrap();
    let manifest = json_file(".devin-plugin/plugin.json");
    let checksums = json_file("checksums.json");
    assert_eq!(manifest["name"], "foxmorrow");
    assert_eq!(manifest["version"], version.as_str());
    assert_eq!(checksums["version"], version.as_str());
    assert_eq!(checksums["repository"], "svishniakov/foxmorrow");
    for (target, sha) in checksums["binaries"].as_object().unwrap() {
        assert!(TARGETS.contains(&target.as_str()), "unknown target {target}");
        let sha = sha.as_str().unwrap();
        assert!(sha.len() == 64 && sha.chars().all(|c| c.is_ascii_hexdigit()), "{target}: {sha}");
    }
}

#[test]
fn hooks_call_the_loader_for_handled_events() {
    let hooks = json_file("hooks.json");
    let events = hooks.as_object().unwrap();
    let names: Vec<&str> = events.keys().map(String::as_str).collect();
    assert_eq!(names, vec!["PostCompaction", "UserPromptSubmit"]);
    for (event, entries) in events {
        for entry in entries.as_array().unwrap() {
            for hook in entry["hooks"].as_array().unwrap() {
                assert_eq!(hook["type"], "command");
                assert_eq!(hook["command"], format!("${{PLUGIN_ROOT}}/bin/foxmorrow hook {event}"));
            }
        }
    }
}

#[cfg(unix)]
#[test]
fn launchers_are_executable() {
    use std::os::unix::fs::PermissionsExt;
    for relative in ["bin/foxmorrow", "scripts/write-checksums.sh"] {
        let mode = std::fs::metadata(root().join(relative)).unwrap().permissions().mode();
        assert!(mode & 0o111 != 0, "{relative} is not executable");
    }
}
```

- [ ] **Шаг 2.** Запустить: `cargo test -p foxmorrow-devin --test repository`. Ожидается: новые тесты падают, файлов нет.

- [ ] **Шаг 3.** `.devin-plugin/plugin.json`:

```json
{
  "name": "foxmorrow",
  "version": "0.1.0",
  "description": "Provable multi-agent runs for Devin: role subagents with their own models, a journal of assignments and reports, and acceptance checked against Devin session evidence.",
  "author": { "name": "Sergejs Visnakovs" },
  "homepage": "https://github.com/svishniakov/foxmorrow",
  "repository": "https://github.com/svishniakov/foxmorrow",
  "license": "MIT",
  "keywords": ["agents", "subagents", "verification", "doc-flow"]
}
```

- [ ] **Шаг 4.** `hooks.json`:

```json
{
  "UserPromptSubmit": [
    { "matcher": "", "hooks": [{ "type": "command", "command": "${PLUGIN_ROOT}/bin/foxmorrow hook UserPromptSubmit" }] }
  ],
  "PostCompaction": [
    { "matcher": "", "hooks": [{ "type": "command", "command": "${PLUGIN_ROOT}/bin/foxmorrow hook PostCompaction" }] }
  ]
}
```

- [ ] **Шаг 5.** `checksums.json` создать скриптом:

```sh
mkdir -p /tmp/foxmorrow-empty-assets
scripts/write-checksums.sh 0.1.0 svishniakov/foxmorrow /tmp/foxmorrow-empty-assets > checksums.json
rmdir /tmp/foxmorrow-empty-assets
```

- [ ] **Шаг 6.** `.gitattributes`:

```
* text=auto eol=lf
*.cmd text eol=crlf
*.ps1 text eol=crlf
```

- [ ] **Шаг 7.** Запустить: `cargo test -p foxmorrow-devin --test repository`. Ожидается: 11 тестов пройдены.

- [ ] **Шаг 8.** Коммит:

```sh
git add .devin-plugin hooks.json checksums.json .gitattributes crates/foxmorrow-devin/tests/repository.rs
git commit -m "Add the plugin manifest, hooks and checksums"
```

## Задача 4. CI и выпуск

**Файлы:** создать `.github/workflows/ci.yml`, `.github/workflows/release.yml`.

- [ ] **Шаг 1.** `.github/workflows/ci.yml`:

```yaml
name: ci

on:
  push:
    branches: [main]
  pull_request:

permissions:
  contents: read

jobs:
  test:
    strategy:
      fail-fast: false
      matrix:
        os: [ubuntu-latest, macos-latest, windows-latest]
    runs-on: ${{ matrix.os }}
    steps:
      - uses: actions/checkout@3d3c42e5aac5ba805825da76410c181273ba90b1
      - run: rustup toolchain install
      - run: cargo fmt --all --check
      - run: cargo clippy --workspace --all-targets --locked -- -D warnings
      - run: cargo test --workspace --locked
      - run: cargo run -q --locked -p foxmorrow -- --plugin-root . sync --plugin --check
```

- [ ] **Шаг 2.** `.github/workflows/release.yml`:

```yaml
name: release

on:
  push:
    tags: ["v*.*.*"]

permissions:
  contents: read

jobs:
  build:
    strategy:
      fail-fast: true
      matrix:
        include:
          - { target: aarch64-apple-darwin, os: macos-latest }
          - { target: x86_64-apple-darwin, os: macos-latest }
          - { target: x86_64-unknown-linux-musl, os: ubuntu-latest, musl: true }
          - { target: aarch64-unknown-linux-musl, os: ubuntu-24.04-arm, musl: true }
          - { target: x86_64-pc-windows-msvc, os: windows-latest, exe: .exe }
    runs-on: ${{ matrix.os }}
    steps:
      - uses: actions/checkout@3d3c42e5aac5ba805825da76410c181273ba90b1
      - name: Check that the tag matches the crate version
        shell: bash
        run: |
          version=$(sed -n '/^\[workspace.package\]/,/^\[/s/^version = "\(.*\)"/\1/p' Cargo.toml)
          test "v$version" = "$GITHUB_REF_NAME"
      - if: matrix.musl
        run: sudo apt-get update && sudo apt-get install -y musl-tools
      - run: rustup toolchain install && rustup target add ${{ matrix.target }}
      - run: cargo build --release --locked -p foxmorrow --target ${{ matrix.target }}
      - shell: bash
        run: |
          mkdir -p dist
          cp target/${{ matrix.target }}/release/foxmorrow${{ matrix.exe }} dist/foxmorrow-${{ matrix.target }}${{ matrix.exe }}
      - uses: actions/upload-artifact@cf430e030ddbb5b0abf93d22962f4752f3646cd9
        with:
          name: foxmorrow-${{ matrix.target }}
          path: dist/*
          if-no-files-found: error

  publish:
    needs: build
    runs-on: ubuntu-latest
    permissions:
      contents: write
    steps:
      - uses: actions/checkout@3d3c42e5aac5ba805825da76410c181273ba90b1
        with:
          ref: main
      - uses: actions/download-artifact@9000827ccba6bdab643e8b6fd33ac0654aef8333
        with:
          path: dist
          merge-multiple: true
      - name: Write checksums
        run: |
          version=${GITHUB_REF_NAME#v}
          sh scripts/write-checksums.sh "$version" "$GITHUB_REPOSITORY" dist > checksums.json
          test "$(grep -c '"[a-z0-9_-]*": "[0-9a-f]\{64\}"' checksums.json)" -eq 5
      - name: Create the release
        env:
          GH_TOKEN: ${{ github.token }}
        run: gh release create "$GITHUB_REF_NAME" dist/* checksums.json --title "$GITHUB_REF_NAME" --notes "foxmorrow $GITHUB_REF_NAME"
      - name: Commit checksums to main
        run: |
          git config user.name "github-actions[bot]"
          git config user.email "41898282+github-actions[bot]@users.noreply.github.com"
          git add checksums.json
          git commit -m "Record checksums for $GITHUB_REF_NAME"
          git push origin HEAD:main
```

`git config` в CI задаёт автора только в одноразовом checkout раннера и не меняет вашу конфигурацию.

- [ ] **Шаг 3.** Проверить разбор YAML и закреплённые SHA:

```sh
python3 -c "import sys, yaml; [yaml.safe_load(open(p)) for p in sys.argv[1:]]; print('ok')" .github/workflows/ci.yml .github/workflows/release.yml
rg -n 'uses: ' .github/workflows | rg -v '@[0-9a-f]{40}$' || echo pinned
```

Ожидается `ok` и `pinned`. Если модуля `yaml` нет, установить его не нужно: вместо этого разбор проверяет первый прогон CI во внешней части.

- [ ] **Шаг 4.** Коммит:

```sh
git add .github
git commit -m "Run checks on three systems and release five binaries by tag"
```

## Задача 5. README

README предназначен людям и написан на английском, как и весь распространяемый плагин. После написания выполнить независимую редактуру `ai-slops-hunter` через отдельного субагента; `humanizer-ru` к английскому тексту не применяется.

**Файлы:** создать `README.md`.

- [ ] **Шаг 1.** `README.md`:

````markdown
# foxmorrow

foxmorrow runs a task in Devin through role subagents and accepts a result only when the evidence agrees with it. Each role has its own model. Root, the session you talk to, creates assignments, starts subagents with a brief, and checks every report against the foxmorrow journal and the Devin session database before moving on.

## What it proves

For every report, `foxmorrow verify` checks:

| Claim | Meaning |
|---|---|
| C1 | The subagent received the recorded brief and role text |
| C2 | The report was submitted from that subagent's own chain |
| C3 | Every answer in the chain came from the model of the role's profile |
| C4 | Referenced checks and tool calls exist and passed |
| C5 | A reviewer or QA did not edit files |
| C6 | Review and QA depend on the work they check, on a different model family |

An unknown Devin database layout gives `unverified`, never a guessed `passed`. Accepted reports are immutable, and `finalize` re-checks the stored snapshots.

It does not prove what it cannot see: a reviewer can still change files through shell commands, and a user who edits `sessions.db` or the journal by hand can forge results.

## Requirements

* Devin CLI or Devin Desktop on macOS, Linux or Windows x64. Devin Cloud sessions are not supported in v1: plugin subagents do not load there.
* `curl` or `wget` on macOS and Linux, PowerShell 5.1 on Windows.
* The `arch-simplicity-guard` skill, required by `architect`, `supervising-architect` and `reviewer`. Install it into `~/.agents/skills/arch-simplicity-guard` from https://github.com/svishniakov/arch-simplicity-guard.

## Install

```sh
devin plugins install svishniakov/foxmorrow
```

The first foxmorrow command downloads the binary of the plugin's version from GitHub Releases, checks its SHA-256 against `checksums.json` and caches it in `~/.cache/foxmorrow/` (`%LOCALAPPDATA%\foxmorrow\` on Windows). Update with `devin plugins update foxmorrow`.

## Use

Start a message with `$fox` or `$foxmorrow`, or run `/foxmorrow:fox`:

```text
$fox Implement the plan docs/implementation/impl-004-health.md
$fox Write the PRD for team invitations based on docs/brd.md and docs/adr/adr-001.md
$fox Fix the login timeout; acceptance criteria are in docs/tasks/login.md#Acceptance criteria
```

| Flow | Run starts with |
|---|---|
| Implement an approved plan | `foxmorrow init --plan <plan>` takes the criteria from the plan and refuses a plan whose `Based on` chain is not approved |
| Write or revise a BRD, ADR, PRD or plan | `foxmorrow init --doc <type> --based-on <documents>` uses the Doc Flow criteria of that type |
| Any task with written acceptance criteria | `foxmorrow init --criteria <file>#<heading>` |

Documents carry `Document status: draft|approved` and `Based on:` in their header. A document with open questions cannot be approved, and only you approve documents.

## Models

Roles have tiers; `profiles/devin.json` maps tiers to Devin models. Override them for one project in `.foxmorrow/profile.json` and run `foxmorrow sync` to generate project roles:

```json
{ "tiers": { "worker": "swe-2-max" }, "roles": { "reviewer": "gpt-6-astra-high" }, "family_rule": true }
```

With `family_rule` on, reviewer and QA must use a model family other than the author's.

## Diagnose

`foxmorrow doctor` shows the plugin root, where the skill list came from, missing required skills and stale role files. `foxmorrow status` shows the current run. Hooks are best effort: when they do not run, `verify` still reads `sessions.db`.

## Release

1. Set the same version in `Cargo.toml`, `.devin-plugin/plugin.json` and `checksums.json` (with an empty `binaries` list), commit to `main`.
2. Push the tag `v<version>`. The release workflow builds five binaries, publishes the GitHub Release and commits the filled `checksums.json` to `main`.

## License

MIT
````

- [ ] **Шаг 2.** Независимая редактура: отдельный субагент читает `/Users/ucnlejumper/.agents/skills/ai-slops-hunter/SKILL.md` и правит только прозу README, не меняя команды, пути, JSON, таблицу утверждений и смысл. Основной агент проверяет итоговые правки.

- [ ] **Шаг 3.** Коммит:

```sh
git add README.md
git commit -m "Describe installation, flows, guarantees and release"
```

## Задача 6. Итоговая проверка локальной части

- [ ] **Шаг 1.**

```sh
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cargo run -q -p foxmorrow -- --plugin-root . sync --plugin --check
rg -n '^\s*//' crates --glob '*.rs'
```

Ожидается: первые четыре команды завершаются с кодом 0, последняя ничего не находит.

- [ ] **Шаг 2.** Проверить загрузчик в режиме разработки на этом репозитории:

```sh
cargo build -q -p foxmorrow
FOXMORROW_DEV_BINARY="$PWD/target/debug/foxmorrow" bin/foxmorrow doctor
bin/foxmorrow --version; echo "exit=$?"
```

Ожидается: `doctor` печатает `plugin_root`, равный корню репозитория, и `"roles": 21`. Второй вызов завершается с кодом 2 и сообщением `no released binary`, потому что выпуска ещё нет.

- [ ] **Шаг 3.** Сверить реализацию с разделом «Критерии приёмки».

## Задача 7. Доступ к GitHub и публикация репозитория

Нужны разрешения пользователя: push в `svishniakov/foxmorrow` и изменение видимости, если репозиторий не публичный. До разрешения выполняются только чтение и диагностика.

- [ ] **Шаг 1.** Авторизация по правилам `github.md`. Хранилище: `$(git rev-parse --path-format=absolute --git-common-dir)/gh`. Все вызовы `gh` идут с `GH_CONFIG_DIR` этого каталога и без `GH_TOKEN` и `GITHUB_TOKEN`:

```sh
export GH_CONFIG_DIR="$(git rev-parse --path-format=absolute --git-common-dir)/gh"
env -u GH_TOKEN -u GITHUB_TOKEN gh auth status --hostname github.com
```

Если входа нет, пользователь выполняет `env -u GH_TOKEN -u GITHUB_TOKEN GH_CONFIG_DIR="<путь>" gh auth login --hostname github.com --web --insecure-storage`.

- [ ] **Шаг 2.** Состояние репозитория (только чтение): видимость, ветка по умолчанию, наличие коммитов и выпусков, права текущего пользователя.

```sh
env -u GH_TOKEN -u GITHUB_TOKEN gh repo view svishniakov/foxmorrow --json visibility,defaultBranchRef,isEmpty,viewerPermission
env -u GH_TOKEN -u GITHUB_TOKEN gh release list --repo svishniakov/foxmorrow
```

Пользователь сообщил, что репозиторий новый и содержит только файл лицензии. Его историю не затирать: получить её и объединить с локальной историей, сохранив лицензию.

- [ ] **Шаг 3.** Пользователь разрешил push. Добавить remote, объединить историю с коммитом лицензии и отправить `main`:

```sh
git remote add origin https://github.com/svishniakov/foxmorrow.git
git -c credential.helper= -c 'credential.helper=!gh auth git-credential' fetch origin
git merge --allow-unrelated-histories --no-edit origin/main
git -c credential.helper= -c 'credential.helper=!gh auth git-credential' push -u origin main
```

`gh auth git-credential` получает тот же `GH_CONFIG_DIR` из окружения.

## Задача 8. CI и выпуск `v0.1.0`

- [ ] **Шаг 1.** Дождаться CI для отправленного коммита:

```sh
env -u GH_TOKEN -u GITHUB_TOKEN gh run watch --repo svishniakov/foxmorrow --exit-status "$(env -u GH_TOKEN -u GITHUB_TOKEN gh run list --repo svishniakov/foxmorrow --workflow ci --limit 1 --json databaseId --jq '.[0].databaseId')"
```

Падения на Windows или Linux исправляются в коде, проверяются локально по задаче 6 и отправляются новым коммитом с разрешения пользователя.

- [ ] **Шаг 2.** С разрешения пользователя на публикацию выпуска:

```sh
git tag v0.1.0
git -c credential.helper= -c 'credential.helper=!gh auth git-credential' push origin v0.1.0
```

- [ ] **Шаг 3.** Дождаться `release` и проверить результат:

```sh
env -u GH_TOKEN -u GITHUB_TOKEN gh release view v0.1.0 --repo svishniakov/foxmorrow --json assets --jq '.assets[].name'
git -c credential.helper= -c 'credential.helper=!gh auth git-credential' pull --ff-only
```

Ожидается: пять бинарников и `checksums.json`; в `main` пришёл коммит `Record checksums for v0.1.0`, локальный `checksums.json` содержит пять хешей. Тест `versions_agree_across_the_plugin` проходит.

## Задача 9. Живые проверки в Devin

Каталог проверки: `/tmp/foxmorrow-live`. Плагин ставится с `--local`, чтобы не попасть в личный манифест в Devin Cloud. Сессии Devin запускаются из оболочки с чистым окружением (`env -i HOME=… PATH=…`), как в плане 000.

- [ ] **Шаг 1.** Установка: `devin plugins install --local svishniakov/foxmorrow -y`, затем `devin plugins info foxmorrow`. Ожидается: 21 роль `foxmorrow:<role>`, skill `fox`, hooks `UserPromptSubmit` и `PostCompaction`.

- [ ] **Шаг 2.** Тестовый проект: репозиторий git с файлом `task.md` и разделом `Критерии приёмки` из одного пункта (`- scripts/hello.sh prints hello`). Пользователь доверяет каталог интерактивным запуском `devin`.

- [ ] **Шаг 3.** Сессия `devin --permission-mode dangerous -p "$fox Create scripts/hello.sh that prints hello. Acceptance criteria are in task.md#Критерии приёмки. Use one python-worker author, then reviewer and qa-verifier."`. После сессии проверить:
  * hook записал `.foxmorrow/session`, файл содержит `session_id` этой сессии;
  * загрузчик скачал бинарник в `~/.cache/foxmorrow/0.1.0/<target>/`, `xattr -l` на нём не показывает `com.apple.quarantine`;
  * `foxmorrow status` показывает назначения автора, review и QA с `verification: accepted` и `final_outcome`;
  * в журнале у принятых наблюдений источник `devin-sessions-db map=1 migration=…`.

- [ ] **Шаг 4.** `PostCompaction`: в интерактивной сессии с активным run выполнить `/compact` и проверить, что root получил состояние run (видно в ответе на вопрос «what is the foxmorrow run state?»). Если Devin не передаёт `additionalContext` для этого события, записать это в результаты: root восстанавливает состояние через `foxmorrow status` по `SKILL.md`.

- [ ] **Шаг 5.** Обновление: после следующего коммита в `main` проверить, подхватывает ли новая сессия изменения без `devin plugins update`. Записать результат; README уже указывает `devin plugins update`.

## Задача 10. Уборка и результаты

- [ ] **Шаг 1.** Удалить `/tmp/foxmorrow-live`, плагин оставить установленным только с согласия пользователя (`devin plugins remove foxmorrow --local`).
- [ ] **Шаг 2.** Заполнить раздел «Результаты внешней части»: для каждого критерия внешней части результат, команда и ключевая строка вывода.
- [ ] **Шаг 3.** Коммит результатов с разрешения пользователя.

## Результаты внешней части

Проверки выполнены 10 ноября 2026 года на macOS arm64, Devin CLI `3000.12.43`.

| Критерий | Результат | Доказательство |
|---|---|---|
| CI на трёх ОС | подтверждено | Run `42880901630` для `90e4fd3`: `ubuntu-latest`, `macos-latest`, `windows-latest` успешны. Первый прогон упал на Windows: `Get-FileHash` недоступен в PowerShell 5.1 раннера; загрузчик теперь считает SHA-256 и скачивает средствами .NET |
| Выпуск `v0.1.0` | подтверждено | Run `42881374951`: пять сборок и `publish` успешны. В выпуске пять бинарников и `checksums.json`; коммит `b2b2ee6 Record checksums for v0.1.0` в `main`, тест версий и хешей проходит |
| Установка плагина | подтверждено | `devin plugins install --local svishniakov/foxmorrow -y`: `version: 0.1.0`, источник GitHub, skill `/foxmorrow:fox`, 21 роль `foxmorrow:<role>`, hooks `UserPromptSubmit` и `PostCompaction` |
| Живой run `$fox` | подтверждено | Run `r-da20e1efd8` в `/tmp/foxmorrow-live`: hook записал `.foxmorrow/session` (`session_id` сессии и `prompt_id`); `a-01` `python-worker` (`swe-2-high`), `a-03` `reviewer` и `a-04` `qa-verifier` (`gpt-6-1-sol-high`) приняты с источником `devin-sessions-db map=1 migration=20`; `a-02` reviewer отклонён за ссылку на чужую квитанцию; final `passed`, `root_model: swe-2-high`, `family_rule: true`. `scripts/hello.sh` печатает `hello` |
| Quarantine на macOS | подтверждено | Бинарник в `~/.cache/foxmorrow/0.1.0/aarch64-apple-darwin/foxmorrow` запускается, `xattr -l` не показывает `com.apple.quarantine`, SHA-256 совпадает с `checksums.json` |
| `PostCompaction` | не проверено | Нужна интерактивная сессия с `/compact`; в режиме `-p` её не вызвать |
| Обновление плагина | проверено | Новая сессия не подхватывает коммит в `main`; после `devin plugins update foxmorrow` содержимое обновилось. README это и указывает |

Найдено в живом run и исправлено коммитом `eb628ab`:

* Текст `qa-verifier` требовал поле `reviewer` в отчёте, а контракт `contract.md` описывал `resolves` как список. Схема отчёта отклоняла оба варианта, root дважды возвращал QA к отчёту. Теперь роль и контракт совпадают со схемой ядра.

Найдено и не исправлено в этом репозитории:

* `arch-simplicity-guard` в Devin не выполняется: его `scripts/render_instructions.py` требует `CODEX_THREAD_ID`. Reviewer провёл проверку без guard и указал это в отчёте. Исправление относится к репозиторию guard.

## Что остаётся непроверенным после плана

* Запуск hooks на Windows: команда `${PLUGIN_ROOT}/bin/foxmorrow` указывает на загрузчик POSIX, а оболочка, которой Devin выполняет hooks на Windows, не проверялась. На Windows hooks могут не сработать; `verify` в этом случае читает `sessions.db`.
* Путь к `sessions.db` на Windows.
* Подпись и нотаризация бинарников macOS не выполняются: загрузка через `curl` не ставит quarantine, но загрузка браузером поставит.
