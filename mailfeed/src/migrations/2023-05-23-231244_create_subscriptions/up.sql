CREATE TABLE subscriptions (
    id INTEGER PRIMARY KEY AUTOINCREMENT NOT NULL,
    user_id INTEGER NOT NULL,
    friendly_name TEXT NOT NULL,
    frequency INTEGER NOT NULL,
    last_sent_time INTEGER NOT NULL DEFAULT 0,
    max_items INTEGER NOT NULL DEFAULT 0,
    is_active BOOLEAN NOT NULL DEFAULT 1,
    feed_id INTEGER NOT NULL,
    FOREIGN KEY(feed_id) REFERENCES feeds(id),
    FOREIGN KEY(user_id) REFERENCES users(id)
);