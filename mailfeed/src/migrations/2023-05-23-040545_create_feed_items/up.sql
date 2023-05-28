CREATE TABLE feed_items (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    feed_id INTEGER NOT NULL,
    title TEXT NOT NULL,
    link TEXT NOT NULL,
    pub_date INTEGER NOT NULL,
    description TEXT,
    author TEXT,
    categories TEXT,
    FOREIGN KEY(feed_id) REFERENCES feeds(id)
);