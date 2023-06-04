CREATE TABLE users (
    id INTEGER PRIMARY KEY AUTOINCREMENT NOT NULL,
    login_email TEXT NOT NULL,
    send_email TEXT NOT NULL,
    password TEXT NOT NULL,
    created_at INTEGER NOT NULL,
    is_active BOOLEAN NOT NULL DEFAULT 1,
    daily_send_time TEXT NOT NULL,
    role TEXT NOT NULL DEFAULT 'user',
    refresh_token TEXT
);