CREATE TABLE settings (
    id INTEGER PRIMARY KEY,
    user_id INTEGER,
    key TEXT NOT NULL,
    value TEXT NOT NULL,
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL
);