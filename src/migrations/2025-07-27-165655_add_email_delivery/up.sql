-- Create email_configs table for user SMTP configurations
CREATE TABLE email_configs (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    user_id INTEGER NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    smtp_host TEXT NOT NULL,
    smtp_port INTEGER NOT NULL DEFAULT 587,
    smtp_username TEXT NOT NULL,
    smtp_password TEXT NOT NULL,  -- AES-256-GCM encrypted
    smtp_use_tls BOOLEAN NOT NULL DEFAULT true,
    from_email TEXT NOT NULL,
    from_name TEXT,
    is_active BOOLEAN NOT NULL DEFAULT true,
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL,
    UNIQUE(user_id)  -- One email config per user
);

-- Add delivery_method column to subscriptions table
-- 0: telegram_only (default for backward compatibility)
-- 1: email_only  
-- 2: both
ALTER TABLE subscriptions ADD COLUMN delivery_method INTEGER NOT NULL DEFAULT 0;

-- Create index for faster lookups
CREATE INDEX idx_email_configs_user_id ON email_configs(user_id);
CREATE INDEX idx_subscriptions_delivery_method ON subscriptions(delivery_method);