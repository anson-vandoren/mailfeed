-- Revert Telegram changes
DROP TABLE IF EXISTS telegram_config;
ALTER TABLE users DROP COLUMN telegram_username;
ALTER TABLE users DROP COLUMN telegram_chat_id;