-- Replace email fields with Telegram chat_id
ALTER TABLE users ADD COLUMN telegram_chat_id TEXT;
ALTER TABLE users ADD COLUMN telegram_username TEXT;

-- For now, keep email fields for backward compatibility during transition
-- Will remove them in a later migration once we're sure everything works

-- Add configuration table for Telegram bot settings
CREATE TABLE IF NOT EXISTS telegram_config (
    id INTEGER PRIMARY KEY,
    bot_token TEXT NOT NULL,
    webhook_url TEXT,
    created_at INTEGER NOT NULL DEFAULT 0,
    updated_at INTEGER NOT NULL DEFAULT 0
);