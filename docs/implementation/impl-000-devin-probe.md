# План 000. Этап 0: проба Devin

Document status: approved
Based on: [foxmorrow v1: дизайн](../design/2026-11-10-foxmorrow-v1-design.md), §10

**Цель:** подтвердить или опровергнуть восемь предположений §10 дизайна на реальном Devin и получить fixture `sessions.db`, на котором план 002 построит reader.

**Тип работы:** живая проверка. Продуктового кода нет. Нужны вход в Devin CLI (`devin auth status` показывает вход в аккаунт Devin) и кредиты на одну сессию с тремя субагентами.

**Что создаётся вне репозитория:** каталог `/tmp/foxmorrow-probe/` и временный плагин `fox-probe`, установленный с `--local`. Оба удаляются в задаче 9. Глобальный `~/.config/devin/config.json` не меняется: для OpenTelemetry используется отдельный файл через `--config`.

**Что попадает в репозиторий:** `tests/fixtures/devin-<версия>/sessions.db`, `tests/fixtures/devin-<версия>/manifest.json` и раздел «Результаты» этого плана.

`D` в командах ниже означает путь к Devin CLI:

```sh
D=~/Applications/Devin.app/Contents/Resources/app/extensions/windsurf/devin/bin/devin
```

Все задачи выполняются в одной сессии shell: переменные `D`, `NONCE`, `MODEL_A`, `MODEL_B`, `MODEL_C`, `DB`, `SID` используются в следующих задачах. В новой сессии их нужно восстановить из `/tmp/foxmorrow-probe/out/` (`nonce`, `models-chosen.txt`, `session-id`).

## Критерии приёмки

- Для каждого из пунктов 1–8 §10 дизайна в разделе «Результаты» записан ответ «подтверждено» или «не подтверждено» со ссылкой на команду и её фактический вывод.
- `profiles/devin.json` в плане 003 можно заполнить по записанному выводу `devin models`: в нём есть как минимум два семейства моделей.
- В `tests/fixtures/devin-<версия>/` есть `sessions.db` только с сессией пробы и `manifest.json` с версией Devin, версией миграций `refinery_schema_history`, `session_id`, `agent_id` и nonce.
- В fixture нет системных сообщений с личными правилами пользователя: содержимое всех системных сообщений без маркера `foxmorrow-role-sha256` заменено на `<redacted>`.
- Плагин `fox-probe` удалён, каталог `/tmp/foxmorrow-probe/` удалён.
- Если пункт не подтвердился, в «Результатах» указано, какое решение из колонки «Если не подтвердится» §10 применяется. Изменение дизайна сверх этой колонки согласуется с пользователем отдельно.

## Задача 1. Подготовка каталога и nonce

- [ ] **Шаг 1.** Проверить вход:

```sh
$D auth status
```

Ожидается сообщение о входе в аккаунт Devin. При «Not logged in» остановиться и попросить пользователя выполнить `$D auth login`.

- [ ] **Шаг 2.** Создать каталоги и nonce:

```sh
mkdir -p /tmp/foxmorrow-probe/plugin/.devin-plugin /tmp/foxmorrow-probe/plugin/agents \
  /tmp/foxmorrow-probe/plugin/scripts /tmp/foxmorrow-probe/project/.devin/agents /tmp/foxmorrow-probe/out
NONCE=$(openssl rand -hex 4); echo "$NONCE" > /tmp/foxmorrow-probe/out/nonce
$D --version > /tmp/foxmorrow-probe/out/devin-version.txt
```

## Задача 2. Список моделей и skills (пункты 7, 8)

- [ ] **Шаг 1.** Записать модели:

```sh
$D models > /tmp/foxmorrow-probe/out/models.txt 2>&1; cat /tmp/foxmorrow-probe/out/models.txt
```

- [ ] **Шаг 2.** Выбрать три модели разных семейств для ролей пробы и записать их:

```sh
MODEL_A=<модель семейства 1>; MODEL_B=<модель семейства 2>; MODEL_C=<модель семейства 3 или 1>
printf 'A=%s\nB=%s\nC=%s\n' "$MODEL_A" "$MODEL_B" "$MODEL_C" > /tmp/foxmorrow-probe/out/models-chosen.txt
```

Значения берутся из вывода шага 1. Если доступно только одно семейство, это фиксируется в «Результатах»: правило о семействах в профиле по умолчанию тогда придётся отключить.

- [ ] **Шаг 3.** Изучить команду skills:

```sh
$D skills --help > /tmp/foxmorrow-probe/out/skills-help.txt 2>&1
$D skills list > /tmp/foxmorrow-probe/out/skills-list.txt 2>&1
$D skills list --json > /tmp/foxmorrow-probe/out/skills-list-json.txt 2>&1
```

Пункт 8 подтверждён, если есть машиночитаемый вывод со списком доступных skills и их источниками.

## Задача 3. Плагин пробы с ролями и hooks (пункты 1, 2, 3)

- [ ] **Шаг 1.** Манифест:

```sh
cat > /tmp/foxmorrow-probe/plugin/.devin-plugin/plugin.json <<'JSON'
{ "name": "fox-probe", "version": "0.0.1", "description": "foxmorrow stage 0 probe" }
JSON
```

- [ ] **Шаг 2.** Роли плагина. Маркер роли в теле проверяет, попадает ли текст роли в системные сообщения:

```sh
for R in a b; do
  M=$([ "$R" = a ] && echo "$MODEL_A" || echo "$MODEL_B")
  cat > /tmp/foxmorrow-probe/plugin/agents/probe-$R.md <<EOF
---
name: probe-$R
description: foxmorrow probe role $R
model: $M
allowed-tools:
  - read
  - exec
---

foxmorrow-role-sha256: probe-$R-$NONCE

You are probe role $R. Run exactly the one shell command given in the task with the exec tool, then reply with its output and nothing else.
EOF
done
```

- [ ] **Шаг 3.** Проектная роль для проверки `.devin/agents/`:

```sh
cat > /tmp/foxmorrow-probe/project/.devin/agents/fox-probe-c.md <<EOF
---
name: fox-probe-c
description: foxmorrow probe project role c
model: $MODEL_C
allowed-tools:
  - read
  - exec
---

foxmorrow-role-sha256: probe-c-$NONCE

You are probe role c. Run exactly the one shell command given in the task with the exec tool, then reply with its output and nothing else.
EOF
```

- [ ] **Шаг 4.** Скрипт hook. Он сохраняет полезную нагрузку и только имена переменных окружения, без значений:

```sh
cat > /tmp/foxmorrow-probe/plugin/scripts/log.sh <<'SH'
#!/bin/sh
cat >> "/tmp/foxmorrow-probe/out/hook-$1.jsonl"
echo >> "/tmp/foxmorrow-probe/out/hook-$1.jsonl"
env | cut -d= -f1 | sort > "/tmp/foxmorrow-probe/out/hook-env-$1.txt"
SH
chmod +x /tmp/foxmorrow-probe/plugin/scripts/log.sh
```

- [ ] **Шаг 5.** `hooks.json` с четырьмя способами указать путь. Каждый вариант пишет в свой файл, поэтому сразу видно, какой сработал:

```sh
cat > /tmp/foxmorrow-probe/plugin/hooks.json <<'JSON'
{
  "UserPromptSubmit": [
    { "matcher": "", "hooks": [ { "type": "command", "command": "/tmp/foxmorrow-probe/plugin/scripts/log.sh prompt-abs" } ] }
  ],
  "PostToolUse": [
    { "matcher": "", "hooks": [
      { "type": "command", "command": "/tmp/foxmorrow-probe/plugin/scripts/log.sh tool-abs" },
      { "type": "command", "command": "./scripts/log.sh tool-rel" },
      { "type": "command", "command": "${PLUGIN_ROOT}/scripts/log.sh tool-plugin-root" },
      { "type": "command", "command": "${CLAUDE_PLUGIN_ROOT}/scripts/log.sh tool-claude-root" }
    ] }
  ]
}
JSON
```

- [ ] **Шаг 6.** Проверить и установить плагин:

```sh
cd /tmp/foxmorrow-probe/project && git init -q
$D doctor
$D plugins install --local /tmp/foxmorrow-probe/plugin -y
$D plugins info fox-probe
```

Ожидается: `doctor` без ошибок разбора ролей, `plugins info` показывает роли `probe-a`, `probe-b` и hooks.

## Задача 4. Сессия пробы (пункты 1, 2, 3)

- [ ] **Шаг 1.** Запустить одну сессию в каталоге проекта:

```sh
cd /tmp/foxmorrow-probe/project
$D --permission-mode dangerous -p -- "Run three subagents in the foreground, one after another, and do nothing else.
1. Profile fox-probe:probe-a, task: 'PROBE-TASK-A $NONCE. Run: echo PROBE-A-$NONCE'
2. Profile fox-probe:probe-b, task: 'PROBE-TASK-B $NONCE. Run: sh -c \"echo PROBE-B-$NONCE; exit 3\"'
3. Profile fox-probe-c, task: 'PROBE-TASK-C $NONCE. Run: echo PROBE-C-$NONCE'
Finally reply with the agent_id of each subagent." | tee /tmp/foxmorrow-probe/out/session.txt
```

`dangerous` допустим только здесь: каталог временный, а команды задания безвредны. Второе задание намеренно завершается с кодом 3, чтобы проверить запись ненулевого exit code.

- [ ] **Шаг 2.** Найти сессию:

```sh
DB="file:$HOME/.local/share/devin/cli/sessions.db?mode=ro"
SID=$(sqlite3 -readonly "$DB" "SELECT id FROM sessions WHERE working_directory LIKE '%foxmorrow-probe/project' ORDER BY created_at DESC LIMIT 1")
echo "$SID" > /tmp/foxmorrow-probe/out/session-id
sqlite3 -readonly "$DB" "SELECT agent_id, chain_node_id FROM subagent_heads WHERE session_id='$SID'" | tee /tmp/foxmorrow-probe/out/heads.txt
```

Ожидаются три строки.

- [ ] **Шаг 3.** Для каждой цепочки вывести роль, модель и начало текста сообщений:

```sh
while IFS='|' read -r AGENT HEAD; do
  echo "== $AGENT"
  sqlite3 -readonly "$DB" "WITH RECURSIVE c(n, d) AS (SELECT $HEAD, 0 UNION ALL SELECT m.parent_node_id, d + 1 FROM message_nodes m JOIN c ON m.node_id = c.n AND m.session_id = '$SID' WHERE m.parent_node_id IS NOT NULL AND d < 200)
    SELECT c.d, json_extract(m.chat_message, '$.role'), json_extract(m.chat_message, '$.metadata.generation_model'),
           instr(json_extract(m.chat_message, '$.content'), 'foxmorrow-role-sha256: probe-') > 0,
           instr(json_extract(m.chat_message, '$.content'), 'PROBE-TASK-') > 0,
           json_extract(m.chat_message, '$.tool_calls')
    FROM c JOIN message_nodes m ON m.node_id = c.n AND m.session_id = '$SID' ORDER BY c.d DESC"
done < /tmp/foxmorrow-probe/out/heads.txt | tee /tmp/foxmorrow-probe/out/chains.txt
```

Пункт 1 подтверждён, если у каждой цепочки все ответы `assistant` имеют `generation_model`, равный модели её роли (A, B, C), и в системных сообщениях есть маркер своей роли.

- [ ] **Шаг 4.** Вызовы `run_subagent` у root и exit code у `exec`:

```sh
sqlite3 -readonly "$DB" "SELECT tool_call_id, json_extract(tool_call_json, '$.rawInput.profile'), substr(json_extract(tool_call_json, '$.rawInput.task'), 1, 60), substr(tool_call_update_json, 1, 300) FROM tool_call_state WHERE session_id = '$SID' AND tool_call_json LIKE '%run_subagent%'" | tee /tmp/foxmorrow-probe/out/run-subagent.txt
sqlite3 -readonly "$DB" "SELECT tool_call_id, json_extract(tool_call_update_json, '$._meta.terminal_exit.exit_code') FROM tool_call_state WHERE session_id = '$SID' AND tool_call_id LIKE 'exec%'" | tee /tmp/foxmorrow-probe/out/exec-exit.txt
```

Ожидается: три вызова с профилями `fox-probe:probe-a`, `fox-probe:probe-b`, `fox-probe-c`; среди exit code есть `0` и `3`.

- [ ] **Шаг 5.** Разобрать результаты hooks:

```sh
ls -la /tmp/foxmorrow-probe/out/ | grep hook
head -c 2000 /tmp/foxmorrow-probe/out/hook-tool-abs.jsonl
cat /tmp/foxmorrow-probe/out/hook-env-tool-abs.txt
```

Пункт 3: записать, какие из файлов `hook-tool-rel`, `hook-tool-plugin-root`, `hook-tool-claude-root` появились. Пункт 2: найти в записях `hook-tool-abs.jsonl` для вызовов субагентов поле, отличающее субагента от root (например, `agent_id` или отдельный `session_id`); в списке переменных окружения найти переменные с признаком субагента.

## Задача 5. Автообновление плагина (пункт 4)

- [ ] **Шаг 1.** Сделать из плагина git-репозиторий и установить его по URL:

```sh
cp -R /tmp/foxmorrow-probe/plugin /tmp/foxmorrow-probe/plugin-git
cd /tmp/foxmorrow-probe/plugin-git && git init -q && git add -A && git -c user.name=probe -c user.email=probe@local commit -qm v1
git clone -q --bare /tmp/foxmorrow-probe/plugin-git /tmp/foxmorrow-probe/remote.git
$D plugins remove fox-probe --local
$D plugins install --local file:///tmp/foxmorrow-probe/remote.git -y
$D plugins list | tee /tmp/foxmorrow-probe/out/plugins-v1.txt
```

Если установка по `file://` не поддерживается, записать вывод ошибки и отметить пункт 4 как «не проверено локально»; проверка переносится в план 005 на настоящем репозитории GitHub.

- [ ] **Шаг 2.** Выпустить новую версию и начать новую сессию без `plugins update`:

```sh
cd /tmp/foxmorrow-probe/plugin-git
sed -i '' 's/"0.0.1"/"0.0.2"/' .devin-plugin/plugin.json
git -c user.name=probe -c user.email=probe@local commit -qam v2 && git push -q /tmp/foxmorrow-probe/remote.git HEAD
cd /tmp/foxmorrow-probe/project && $D -p -- "Reply with OK."
$D plugins list | tee /tmp/foxmorrow-probe/out/plugins-after-session.txt
```

Пункт 4 подтверждён, если после сессии версия `0.0.2` без явного `plugins update`.

## Задача 6. Атрибут quarantine (пункт 5)

- [ ] **Шаг 1.**

```sh
curl -fsSL -o /tmp/foxmorrow-probe/out/download.bin https://static.rust-lang.org/dist/channel-rust-stable.toml
xattr -l /tmp/foxmorrow-probe/out/download.bin | tee /tmp/foxmorrow-probe/out/xattr.txt
```

Пункт 5 подтверждён для скачивания через `curl`, если в выводе нет `com.apple.quarantine`. Запуск скачанного неподписанного бинарника проверяется в плане 005 на собранном `foxmorrow`.

## Задача 7. OpenTelemetry (пункт 6)

- [ ] **Шаг 1.** Приёмник, который сохраняет тела запросов:

```sh
cat > /tmp/foxmorrow-probe/otel.py <<'PY'
import http.server, itertools
counter = itertools.count()
class H(http.server.BaseHTTPRequestHandler):
    def do_POST(self):
        body = self.rfile.read(int(self.headers.get("Content-Length", 0)))
        with open(f"/tmp/foxmorrow-probe/out/otel-{next(counter):04d}{self.path.replace('/', '_')}.json", "wb") as f:
            f.write(body)
        self.send_response(200); self.end_headers()
    def log_message(self, *a): pass
http.server.HTTPServer(("127.0.0.1", 4318), H).serve_forever()
PY
python3 /tmp/foxmorrow-probe/otel.py & echo $! > /tmp/foxmorrow-probe/out/otel.pid
cat > /tmp/foxmorrow-probe/config.json <<'JSON'
{ "otel": { "endpoint": "http://127.0.0.1:4318", "disable_user_prompt_logging": true, "disable_tool_logging": true, "log_export_interval_ms": 1000 } }
JSON
```

- [ ] **Шаг 2.** Короткая сессия с одним субагентом:

```sh
cd /tmp/foxmorrow-probe/project
OTEL_EXPORTER_OTLP_PROTOCOL=http/json $D --config /tmp/foxmorrow-probe/config.json --permission-mode dangerous -p -- \
  "Run one foreground subagent with profile fox-probe:probe-a, task: 'PROBE-OTEL $NONCE. Run: echo OTEL-$NONCE'. Then reply with OK."
sleep 3; kill "$(cat /tmp/foxmorrow-probe/out/otel.pid)"
grep -l '"api_request"' /tmp/foxmorrow-probe/out/otel-*_v1_logs.json | head
```

Пункт 6 подтверждён, если у событий `api_request` или `tool_result` субагента есть атрибут, отличающий его от root. Проверяются ключи атрибутов в найденных файлах.

## Задача 8. Fixture для плана 002

- [ ] **Шаг 1.** Скопировать только сессию пробы:

```sh
VER=$(awk '{print $2}' /tmp/foxmorrow-probe/out/devin-version.txt)
FIX=/Users/ucnlejumper/Sites/foxmorrow/tests/fixtures/devin-$VER
mkdir -p "$FIX"
SRC="$HOME/.local/share/devin/cli/sessions.db"
sqlite3 -readonly "file:$SRC?mode=ro" ".schema sessions" ".schema message_nodes" ".schema tool_call_state" ".schema subagent_heads" ".schema refinery_schema_history" > /tmp/foxmorrow-probe/out/schema.sql
sqlite3 "$FIX/sessions.db" < /tmp/foxmorrow-probe/out/schema.sql
sqlite3 "$FIX/sessions.db" "ATTACH 'file:$SRC?mode=ro' AS src;
  INSERT INTO refinery_schema_history SELECT * FROM src.refinery_schema_history;
  INSERT INTO sessions SELECT * FROM src.sessions WHERE id = '$SID';
  INSERT INTO message_nodes SELECT * FROM src.message_nodes WHERE session_id = '$SID';
  INSERT INTO tool_call_state SELECT * FROM src.tool_call_state WHERE session_id = '$SID';
  INSERT INTO subagent_heads SELECT * FROM src.subagent_heads WHERE session_id = '$SID';
  DETACH src;"
```

`.schema` без явного `CREATE TABLE sqlite_sequence` может вывести служебную таблицу; если `sqlite3` сообщит о ней ошибку, удалить её строку из `schema.sql` и повторить.

- [ ] **Шаг 2.** Обезличить системные сообщения и проверить:

```sh
sqlite3 "$FIX/sessions.db" "UPDATE message_nodes SET chat_message = json_set(chat_message, '$.content', '<redacted>')
  WHERE json_extract(chat_message, '$.role') = 'system' AND instr(json_extract(chat_message, '$.content'), 'foxmorrow-role-sha256') = 0;
  UPDATE sessions SET working_directory = '/fixture/project', workspace_dirs = NULL, title = 'foxmorrow probe';
  VACUUM;"
sqlite3 "$FIX/sessions.db" "SELECT count(*) FROM message_nodes WHERE json_extract(chat_message, '$.role') = 'system' AND json_extract(chat_message, '$.content') != '<redacted>'"
```

Ожидается число системных сообщений с маркерами ролей (по одному на цепочку, если пункт 1 подтвердился). Просмотреть оставшиеся системные сообщения вручную: в них не должно быть личных правил пользователя.

- [ ] **Шаг 3.** `manifest.json`:

```sh
MIG=$(sqlite3 "$FIX/sessions.db" "SELECT max(version) FROM refinery_schema_history")
python3 - "$FIX" "$VER" "$MIG" "$SID" "$NONCE" <<'PY'
import json, sys
fix, ver, mig, sid, nonce = sys.argv[1:]
heads = [l.split("|")[0] for l in open("/tmp/foxmorrow-probe/out/heads.txt").read().split()]
json.dump({"devin_version": ver, "schema_migration": int(mig), "session_id": sid, "agent_ids": heads, "nonce": nonce,
           "models": dict(l.strip().split("=", 1) for l in open("/tmp/foxmorrow-probe/out/models-chosen.txt"))},
          open(f"{fix}/manifest.json", "w"), indent=2)
PY
cat "$FIX/manifest.json"
```

## Задача 9. Запись результатов и уборка

- [ ] **Шаг 1.** Заполнить раздел «Результаты» ниже: для каждого пункта ответ, команда и ключевая строка вывода. Выводы `models.txt`, `skills-*.txt` и список ключей hooks скопировать в результаты дословно, без значений переменных окружения.

- [ ] **Шаг 2.** Удалить плагин и временные файлы:

```sh
$D plugins remove fox-probe --local
$D plugins list
rm -rf /tmp/foxmorrow-probe
```

Ожидается: `fox-probe` отсутствует в списке.

- [ ] **Шаг 3.** Коммит, если пользователь разрешил коммиты в этом run:

```sh
cd /Users/ucnlejumper/Sites/foxmorrow
git add tests/fixtures docs/implementation/impl-000-devin-probe.md
git commit -m "Record Devin stage 0 probe results and sessions.db fixture"
```

## Результаты

Проба выполнена 10 ноября 2026 года на Devin CLI `3000.12.43 (517d03d73926)`, сессия `dedicated-sherbet`, nonce `42fdbeca`.

| № | Пункт §10 | Результат | Доказательство |
|---|---|---|---|
| 1 | `model:` у ролей плагина и проекта; текст роли в системных сообщениях | подтверждено | Все ответы цепочек имеют `generation_model` своей роли: `853de4cc` → `swe-2-medium` (`fox-probe:probe-a`), `edc2836c` → `claude-haiku-5-5-low` (`fox-probe:probe-b`), `f2a9cfff` → `gpt-6-1-sol-low` (проектная `fox-probe-c`). Первое системное сообщение каждой цепочки содержит `foxmorrow-role-sha256: <роль>-42fdbeca`. `rawInput.profile` у `run_subagent` совпадает с профилем, задание сохранено дословно. Exit code `exec`: `0`, `3`, `0` |
| 2 | Признак субагента в hooks | не подтверждено | Полезная нагрузка `PostToolUse` содержит `hook_event_name`, `prompt_id`, `session_id`, `tool_input`, `tool_name`, `tool_response`, `tool_use_id`. `session_id` у вызовов субагентов тот же, что у root. Но `tool_use_id` совпадает с ID вызова в цепочке `sessions.db`, поэтому авторство устанавливается соединением по `tool_use_id`. Применяется решение §10: авторство C2 и C4 по `sessions.db` |
| 3 | Путь к корню плагина в `hooks.json` | подтверждено | Сработали `${PLUGIN_ROOT}` и `${CLAUDE_PLUGIN_ROOT}`, абсолютный путь тоже; `./scripts/…` не сработал. В окружении hook есть `PLUGIN_ROOT`, `DEVIN_PLUGIN_ROOT`, `CLAUDE_PLUGIN_ROOT`, `PLUGIN_DATA`, `DEVIN_PROJECT_DIR`, `DEVIN_TRACEPARENT` |
| 4 | Автообновление плагинов | не проверено локально | `devin plugins install --local file:///…` → `fatal: transport 'file' not allowed`. Проверка переносится в план 005 на репозитории GitHub |
| 5 | quarantine у скачанного файла | подтверждено для `curl` | `xattr -l` у файла, скачанного `curl`, пуст. Запуск скачанного бинарника проверяется в плане 005 |
| 6 | Признак субагента в OpenTelemetry | не подтверждено | Ключи атрибутов событий: `session.id`, `prompt.id`, `model`, `tool_use_id` и счётчики; признака субагента нет, события субагента отличаются только `model`. OpenTelemetry не используется |
| 7 | Доступные модели | подтверждено | `devin models`: 49 семейств, в том числе SWE-2, SWE-2 Flash, Claude Opus 5.5, Sonnet 5.5, Haiku 5.5, Fable 5.x, GPT-6 Astra, GPT-6 Sol, GPT-6.1 Sol/Terra, Gemini 4 Argon, Kimi K3.1 Pro, GLM-5.3, Grok 4.7, DeepSeek V4. Уровень reasoning входит в ID (`-low`, `-medium`, `-high`, `-xhigh`, `-max`). Для правила о семействах доступно больше двух семейств |
| 8 | Список skills | подтверждено | `devin skills list --json` возвращает массив объектов с `name`, `description`, `triggers`, `provider`, `base_dir`, `display_name`, `warnings`, `errors` |

Отклонения от плана при выполнении:

- CLI видит вход в аккаунт только при запуске с чистым окружением: переменные Devin Desktop (`WINDSURF_*`, `DEVIN_FD_PATH`) мешают найти учётные данные. Все команды выполнялись через `env -i HOME=… PATH=… ~/.local/bin/devin`. Для `foxmorrow` это значит: если hooks или CLI будут вызывать `devin`, окружение Desktop нужно учитывать.
- `devin -p` отказывается работать в непроверенном каталоге. Пользователь доверил `/tmp/foxmorrow-probe/project` интерактивным запуском. `--config` с `respect_workspace_trust: false` не помог.
- Схема для fixture взята из `sqlite_master` только с таблицами и индексами: триггеры `message_nodes` ссылаются на FTS-таблицу `message_search`.
- Обезличивание шире плана: кроме системных сообщений удалены `metadata.extensions` (пути правил и skills пользователя), очищены `sessions.metadata` и `cogs_json`. Поиск `ucnlejumper|svishniakov|polydesk` по fixture пуст.

Fixture: `tests/fixtures/devin-3000.12.43/sessions.db` (100 сообщений, 6 вызовов инструментов, 3 цепочки, миграция схемы 20) и `manifest.json`.
