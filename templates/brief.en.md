Role: {{role}}   Profile: {{profile}}   role-sha256 {{role_sha256}}
Expected model: {{expected_models}}
References: {{references_dir}}
CLI: {{foxmorrow}}

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

## Project memory

{{memory}}

## Rules

- Change only the allowed paths. Do not commit, push, publish or deploy.
- Run every check through:
  {{foxmorrow}} check -a {{assignment_id}} -- <command>
- Finish with exactly one report:

```sh
{{foxmorrow}} report -a {{assignment_id}} <<'JSON'
{"summary": "...", "status": "completed", "outcome": "passed",
 "criteria": {"ac-1": {"status": "passed", "reason": "...", "evidence": ["chk-001"]}},
 "evidence": ["chk-001"]}
JSON
```

`status` is `completed` or `interrupted`; `outcome` and criterion `status` are `passed`, `failed` or `unverified`.
