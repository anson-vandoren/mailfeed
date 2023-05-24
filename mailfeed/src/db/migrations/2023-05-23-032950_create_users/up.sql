CREATE TABLE users (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    login_email VARCHAR(255) NOT NULL,
    send_email VARCHAR(255) NOT NULL,
    password VARCHAR(255) NOT NULL,
    created_at INTEGER NOT NULL,
    is_active BOOLEAN NOT NULL DEFAULT 1,
    daily_send_time VARCHAR(255) NOT NULL,
    roles VARCHAR(255) NOT NULL DEFAULT 'user'
);