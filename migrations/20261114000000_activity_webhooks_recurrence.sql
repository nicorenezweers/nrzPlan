-- Wiederkehrende Todos, Aktivitätslog, Webhooks, Einstellungen

ALTER TABLE todos ADD COLUMN recurrence TEXT;

CREATE TABLE IF NOT EXISTS activity (
    id         TEXT PRIMARY KEY NOT NULL,
    kind       TEXT NOT NULL,
    message    TEXT NOT NULL,
    actor      TEXT NOT NULL DEFAULT 'nrzPlan',
    board_id   TEXT,
    card_id    TEXT,
    created_at TEXT NOT NULL
);
CREATE INDEX IF NOT EXISTS idx_activity_board ON activity(board_id, created_at);
CREATE INDEX IF NOT EXISTS idx_activity_created ON activity(created_at);

CREATE TABLE IF NOT EXISTS webhooks (
    id          TEXT PRIMARY KEY NOT NULL,
    name        TEXT NOT NULL,
    url         TEXT NOT NULL,
    secret      TEXT,
    events      TEXT NOT NULL DEFAULT '[]',
    active      INTEGER NOT NULL DEFAULT 1,
    created_at  TEXT NOT NULL,
    last_status INTEGER,
    last_error  TEXT,
    last_at     TEXT
);

CREATE TABLE IF NOT EXISTS settings (
    key   TEXT PRIMARY KEY NOT NULL,
    value TEXT NOT NULL
);
