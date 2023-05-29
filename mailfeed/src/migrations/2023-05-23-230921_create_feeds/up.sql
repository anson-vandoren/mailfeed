CREATE TABLE feeds (
    id INTEGER PRIMARY KEY AUTOINCREMENT NOT NULL,
    url TEXT NOT NULL,
    feed_type INTEGER NOT NULL,
    title TEXT NOT NULL,
    last_checked INTEGER NOT NULL DEFAULT 0,
    last_updated INTEGER NOT NULL,
    error_time INTEGER NOT NULL DEFAULT 0,
    error_message TEXT
);