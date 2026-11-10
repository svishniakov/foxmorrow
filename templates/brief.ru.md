Роль: {{role}}   Профиль: {{profile}}   role-sha256 {{role_sha256}}
Ожидаемая модель: {{expected_models}}
Справочники: {{references_dir}}
CLI: {{foxmorrow}}

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
  {{foxmorrow}} check -a {{assignment_id}} -- <command>
- Заверши работу ровно одним отчётом:

```sh
{{foxmorrow}} report -a {{assignment_id}} <<'JSON'
{"summary": "...", "status": "completed", "outcome": "passed",
 "criteria": {"ac-1": {"status": "passed", "reason": "...", "evidence": ["chk-001"]}},
 "evidence": ["chk-001"]}
JSON
```

`status` принимает значения `completed` или `interrupted`; `outcome` и `status` критерия принимают значения `passed`, `failed` или `unverified`.
