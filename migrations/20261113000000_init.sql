-- nrzPlan: Boards (Kanban), Todos, Kalender, API-Keys

CREATE TABLE IF NOT EXISTS boards (
    id          TEXT PRIMARY KEY NOT NULL,
    name        TEXT NOT NULL,
    description TEXT,
    position    INTEGER NOT NULL DEFAULT 0,
    created_at  TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%S', 'now'))
);

CREATE TABLE IF NOT EXISTS board_columns (
    id        TEXT PRIMARY KEY NOT NULL,
    board_id  TEXT NOT NULL REFERENCES boards(id) ON DELETE CASCADE,
    name      TEXT NOT NULL,
    position  INTEGER NOT NULL DEFAULT 0,
    is_done   INTEGER NOT NULL DEFAULT 0
);
CREATE INDEX IF NOT EXISTS idx_columns_board ON board_columns(board_id, position);

CREATE TABLE IF NOT EXISTS labels (
    id       TEXT PRIMARY KEY NOT NULL,
    board_id TEXT NOT NULL REFERENCES boards(id) ON DELETE CASCADE,
    name     TEXT NOT NULL,
    color    TEXT NOT NULL DEFAULT 'gray',
    UNIQUE(board_id, name)
);

CREATE TABLE IF NOT EXISTS cards (
    id          TEXT PRIMARY KEY NOT NULL,
    board_id    TEXT NOT NULL REFERENCES boards(id) ON DELETE CASCADE,
    column_id   TEXT NOT NULL REFERENCES board_columns(id) ON DELETE CASCADE,
    title       TEXT NOT NULL,
    description TEXT,
    labels      TEXT NOT NULL DEFAULT '[]',
    priority    INTEGER NOT NULL DEFAULT 0,
    due_date    TEXT,
    url         TEXT,
    source      TEXT,
    external_id TEXT,
    position    INTEGER NOT NULL DEFAULT 0,
    created_at  TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%S', 'now')),
    updated_at  TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%S', 'now'))
);
CREATE INDEX IF NOT EXISTS idx_cards_column ON cards(column_id, position);
CREATE UNIQUE INDEX IF NOT EXISTS idx_cards_external ON cards(board_id, source, external_id)
    WHERE source IS NOT NULL AND external_id IS NOT NULL;

CREATE TABLE IF NOT EXISTS checklist_items (
    id       TEXT PRIMARY KEY NOT NULL,
    card_id  TEXT NOT NULL REFERENCES cards(id) ON DELETE CASCADE,
    text     TEXT NOT NULL,
    done     INTEGER NOT NULL DEFAULT 0,
    position INTEGER NOT NULL DEFAULT 0
);
CREATE INDEX IF NOT EXISTS idx_checklist_card ON checklist_items(card_id, position);

CREATE TABLE IF NOT EXISTS todos (
    id          TEXT PRIMARY KEY NOT NULL,
    title       TEXT NOT NULL,
    notes       TEXT,
    list        TEXT,
    due_date    TEXT,
    priority    INTEGER NOT NULL DEFAULT 0,
    done        INTEGER NOT NULL DEFAULT 0,
    done_at     TEXT,
    source      TEXT,
    external_id TEXT,
    created_at  TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%S', 'now'))
);
CREATE INDEX IF NOT EXISTS idx_todos_due ON todos(done, due_date);
CREATE UNIQUE INDEX IF NOT EXISTS idx_todos_external ON todos(source, external_id)
    WHERE source IS NOT NULL AND external_id IS NOT NULL;

CREATE TABLE IF NOT EXISTS events (
    id          TEXT PRIMARY KEY NOT NULL,
    title       TEXT NOT NULL,
    description TEXT,
    location    TEXT,
    start_at    TEXT NOT NULL,
    end_at      TEXT NOT NULL,
    all_day     INTEGER NOT NULL DEFAULT 0,
    color       TEXT NOT NULL DEFAULT 'blue',
    source      TEXT,
    external_id TEXT,
    created_at  TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%S', 'now'))
);
CREATE INDEX IF NOT EXISTS idx_events_start ON events(start_at);
CREATE UNIQUE INDEX IF NOT EXISTS idx_events_external ON events(source, external_id)
    WHERE source IS NOT NULL AND external_id IS NOT NULL;

CREATE TABLE IF NOT EXISTS api_keys (
    id           TEXT PRIMARY KEY NOT NULL,
    name         TEXT NOT NULL,
    prefix       TEXT NOT NULL,
    key_hash     TEXT NOT NULL UNIQUE,
    created_at   TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%S', 'now')),
    last_used_at TEXT
);
