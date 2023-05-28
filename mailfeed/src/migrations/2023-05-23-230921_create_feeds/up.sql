CREATE TABLE feeds (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    url TEXT NOT NULL,
    feed_type TEXT NOT NULL,
    title TEXT NOT NULL,
    last_checked INTEGER NOT NULL DEFAULT 0,
    last_updated INTEGER NOT NULL,
    error_time INTEGER NOT NULL DEFAULT 0,
    error_message TEXT
);