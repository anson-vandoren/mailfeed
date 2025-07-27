-- Drop indexes
DROP INDEX IF EXISTS idx_subscriptions_delivery_method;
DROP INDEX IF EXISTS idx_email_configs_user_id;

-- Remove delivery_method column from subscriptions table
-- Note: SQLite doesn't support DROP COLUMN directly, so we need to recreate the table
CREATE TABLE subscriptions_backup AS SELECT 
    id, user_id, friendly_name, frequency, last_sent_time, max_items, is_active, feed_id
FROM subscriptions;

DROP TABLE subscriptions;

CREATE TABLE subscriptions (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    user_id INTEGER NOT NULL,
    friendly_name TEXT NOT NULL,
    frequency INTEGER NOT NULL,
    last_sent_time INTEGER NOT NULL,
    max_items INTEGER NOT NULL,
    is_active BOOLEAN NOT NULL,
    feed_id INTEGER NOT NULL
);

INSERT INTO subscriptions (id, user_id, friendly_name, frequency, last_sent_time, max_items, is_active, feed_id)
SELECT id, user_id, friendly_name, frequency, last_sent_time, max_items, is_active, feed_id
FROM subscriptions_backup;

DROP TABLE subscriptions_backup;

-- Drop email_configs table
DROP TABLE IF EXISTS email_configs;