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
