# Email Delivery Feature Design Document

## Overview

This document outlines the design for adding email delivery functionality to MailFeed, allowing users to receive RSS/Atom feed updates via email in addition to or instead of Telegram delivery.

## Goals

- **User Choice**: Allow users to choose email delivery, Telegram delivery, or both
- **Self-Hosted**: Users provide their own SMTP configuration (no external email service dependencies)
- **Security**: Secure storage of SMTP credentials with encryption
- **Backward Compatibility**: Existing Telegram functionality remains unchanged
- **Unified Experience**: Consistent delivery scheduling and formatting across methods

## Current Architecture Analysis

### Existing Delivery System
- **Location**: `src/tasks/telegram_sender/runner.rs`
- **Pattern**: Background task polls subscriptions every `CHECK_INTERVAL`
- **Logic**: Frequency-based delivery (realtime, hourly, daily) with `last_sent_time` tracking
- **User Config**: `telegram_chat_id` and `telegram_username` in users table
- **System Config**: Bot token in `telegram_config` table

### Data Flow
1. Background task queries active users with Telegram chat IDs
2. For each user, finds subscriptions ready to send based on frequency
3. Formats Telegram HTML message with feed items
4. Sends via Telegram API and updates `last_sent_time`

## Database Schema Design

### New Table: `email_configs`
```sql
CREATE TABLE email_configs (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    user_id INTEGER NOT NULL REFERENCES users(id),
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
```

### Extended Subscription Model
```sql
-- Add delivery method to existing subscriptions table
ALTER TABLE subscriptions ADD COLUMN delivery_method INTEGER NOT NULL DEFAULT 0;

-- Enum values:
-- 0: telegram_only (default for backward compatibility)
-- 1: email_only  
-- 2: both
```

### Migration Strategy
```sql
-- Migration: 001_add_email_delivery.sql
CREATE TABLE email_configs (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    user_id INTEGER NOT NULL REFERENCES users(id),
    smtp_host TEXT NOT NULL,
    smtp_port INTEGER NOT NULL DEFAULT 587,
    smtp_username TEXT NOT NULL,
    smtp_password TEXT NOT NULL,
    smtp_use_tls BOOLEAN NOT NULL DEFAULT true,
    from_email TEXT NOT NULL,
    from_name TEXT,
    is_active BOOLEAN NOT NULL DEFAULT true,
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL,
    UNIQUE(user_id)
);

ALTER TABLE subscriptions ADD COLUMN delivery_method INTEGER NOT NULL DEFAULT 0;
```

## API Design

### New Email Configuration Endpoints

#### Create/Update Email Config
```
POST/PATCH /api/users/{user_id}/email-config
Content-Type: application/x-www-form-urlencoded

smtp_host=smtp.gmail.com
smtp_port=587
smtp_username=user@gmail.com
smtp_password=app_password_here
smtp_use_tls=on
from_email=feeds@domain.com
from_name=MailFeed Updates
```

#### Delete Email Config
```
DELETE /api/users/{user_id}/email-config
```

#### Test Email
```
POST /api/users/{user_id}/test-email
```

### Request/Response Types
```rust
// src/api/email/types.rs
#[derive(Deserialize)]
pub struct EmailConfigForm {
    pub smtp_host: String,
    pub smtp_port: u16,
    pub smtp_username: String,
    pub smtp_password: String,
    pub smtp_use_tls: Option<String>, // HTML checkbox
    pub from_email: String,
    pub from_name: Option<String>,
}

#[derive(Serialize, Deserialize, AsExpression, FromSqlRow, Clone, Copy)]
#[diesel(sql_type = Integer)]
pub enum DeliveryMethod {
    TelegramOnly = 0,
    EmailOnly = 1,
    Both = 2,
}
```

### Updated Subscription Types
```rust
// Extend existing SubscriptionForm in web_ui.rs
#[derive(Deserialize)]
struct SubscriptionForm {
    friendly_name: Option<String>,
    frequency: String,
    max_items: i32,
    is_active: Option<String>,
    delivery_method: Option<String>, // "telegram_only", "email_only", "both"
}
```

## UI/UX Design

### Settings Page Addition
Add email configuration section to `templates/settings.html`:

```html
<!-- Email Delivery Section -->
<section>
    <h2>📧 Email Delivery</h2>
    
    {% if user.email_config %}
    <!-- Email Configured State -->
    <div style="padding: 1rem; background: #d1fae5; border: 1px solid #10b981; border-radius: var(--pico-border-radius); margin-bottom: 1rem;">
        <h4 style="margin: 0 0 0.5rem 0; color: #065f46;">✅ Email Configured</h4>
        <p style="margin: 0; font-size: 0.875rem; color: #047857;">
            <strong>SMTP Host:</strong> {{ user.email_config.smtp_host }}:{{ user.email_config.smtp_port }}<br>
            <strong>From Email:</strong> {{ user.email_config.from_email }}
        </p>
    </div>
    
    <div style="display: flex; gap: 0.5rem; flex-wrap: wrap; margin-bottom: 1rem;">
        <button hx-post="/api/users/{{ user.id }}/test-email"
                hx-target="#settings-messages"
                class="secondary">
            🧪 Send Test Email
        </button>
        
        <button onclick="document.getElementById('email-edit-form').style.display = 'block'; this.style.display = 'none';"
                class="secondary">
            ✏️ Edit Settings
        </button>
        
        <form hx-delete="/api/users/{{ user.id }}/email-config" 
              hx-target="#settings-messages"
              style="display: inline;">
            <button type="submit" class="contrast">
                🔌 Remove Email Config
            </button>
        </form>
    </div>
    {% else %}
    <!-- Email Not Configured State -->
    <div style="padding: 1rem; background: #fef3c7; border: 1px solid #f59e0b; border-radius: var(--pico-border-radius); margin-bottom: 1rem;">
        <h4 style="margin: 0 0 0.5rem 0; color: #92400e;">⚠️ Email Not Configured</h4>
        <p style="margin: 0; color: #92400e;">Configure your SMTP settings to receive feed updates via email.</p>
    </div>
    {% endif %}
    
    <!-- Email Configuration Form -->
    <form id="email-edit-form" 
          hx-patch="/api/users/{{ user.id }}/email-config" 
          hx-target="#settings-messages"
          {% if user.email_config %}style="display: none;"{% endif %}>
        
        <h4>📧 SMTP Configuration</h4>
        
        <div class="grid">
            <label>
                SMTP Host *
                <input type="text" name="smtp_host" 
                       value="{% if user.email_config %}{{ user.email_config.smtp_host }}{% endif %}"
                       placeholder="smtp.gmail.com" required>
                <small>Your email provider's SMTP server</small>
            </label>
            
            <label>
                Port
                <input type="number" name="smtp_port" 
                       value="{% if user.email_config %}{{ user.email_config.smtp_port }}{% else %}587{% endif %}"
                       min="1" max="65535">
                <small>Usually 587 (TLS) or 465 (SSL)</small>
            </label>
        </div>
        
        <div class="grid">
            <label>
                Username *
                <input type="text" name="smtp_username" 
                       value="{% if user.email_config %}{{ user.email_config.smtp_username }}{% endif %}"
                       placeholder="your-email@gmail.com" required>
            </label>
            
            <label>
                Password *
                <input type="password" name="smtp_password" 
                       placeholder="{% if user.email_config %}••••••••{% else %}App password or email password{% endif %}" 
                       {% if not user.email_config %}required{% endif %}>
                <small>Use app-specific passwords for Gmail/Outlook</small>
            </label>
        </div>
        
        <div class="grid">
            <label>
                From Email *
                <input type="email" name="from_email" 
                       value="{% if user.email_config %}{{ user.email_config.from_email }}{% endif %}"
                       placeholder="feeds@yourdomain.com" required>
            </label>
            
            <label>
                From Name
                <input type="text" name="from_name" 
                       value="{% if user.email_config %}{{ user.email_config.from_name }}{% endif %}"
                       placeholder="MailFeed Updates">
            </label>
        </div>
        
        <label>
            <input type="checkbox" name="smtp_use_tls" 
                   {% if not user.email_config or user.email_config.smtp_use_tls %}checked{% endif %}> 
            Use TLS encryption (recommended)
        </label>
        
        <div style="display: flex; gap: 0.5rem; margin-top: 1rem;">
            <button type="submit">💾 Save Email Config</button>
            {% if user.email_config %}
            <button type="button" 
                    onclick="document.getElementById('email-edit-form').style.display = 'none';"
                    class="secondary">
                ✖️ Cancel
            </button>
            {% endif %}
        </div>
    </form>
</section>
```

### Subscription Form Enhancement
Add delivery method selection to dashboard subscription forms:

```html
<!-- Add to subscription form in dashboard.html -->
<div class="form-field">
    <label class="field-label">
        <span class="field-icon">📨</span>
        <span class="field-title">Delivery Method</span>
    </label>
    <select name="delivery_method" class="enhanced-input" required>
        <option value="telegram_only">📱 Telegram Only</option>
        <option value="email_only">📧 Email Only</option>
        <option value="both">📱📧 Both Telegram & Email</option>
    </select>
    <small class="field-help">Choose how you want to receive updates for this feed</small>
</div>
```

## Backend Implementation

### New Models

#### EmailConfig Model
```rust
// src/models/email_config.rs
use crate::schema::email_configs;
use diesel::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Queryable, Identifiable)]
#[diesel(table_name = email_configs)]
pub struct EmailConfig {
    pub id: i32,
    pub user_id: i32,
    pub smtp_host: String,
    pub smtp_port: i32,
    pub smtp_username: String,
    pub smtp_password: String, // Encrypted
    pub smtp_use_tls: bool,
    pub from_email: String,
    pub from_name: Option<String>,
    pub is_active: bool,
    pub created_at: i32,
    pub updated_at: i32,
}

#[derive(Debug, Insertable)]
#[diesel(table_name = email_configs)]
pub struct NewEmailConfig {
    pub user_id: i32,
    pub smtp_host: String,
    pub smtp_port: i32,
    pub smtp_username: String,
    pub smtp_password: String,
    pub smtp_use_tls: bool,
    pub from_email: String,
    pub from_name: Option<String>,
    pub is_active: bool,
    pub created_at: i32,
    pub updated_at: i32,
}
```

#### DeliveryMethod Enum
```rust
// src/models/subscription.rs (extend existing)
#[derive(Debug, Serialize, Deserialize, AsExpression, FromSqlRow, Clone, Copy, PartialEq)]
#[diesel(sql_type = Integer)]
pub enum DeliveryMethod {
    TelegramOnly = 0,
    EmailOnly = 1,
    Both = 2,
}

impl fmt::Display for DeliveryMethod {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DeliveryMethod::TelegramOnly => write!(f, "telegram_only"),
            DeliveryMethod::EmailOnly => write!(f, "email_only"),
            DeliveryMethod::Both => write!(f, "both"),
        }
    }
}

// Add to Subscription struct:
pub delivery_method: DeliveryMethod,
```

### Email Delivery Task

#### Email Sender Module
```rust
// src/tasks/email_sender/mod.rs
pub mod runner;
pub mod types;

// src/tasks/email_sender/types.rs
pub use super::telegram_sender::types::{FeedData, TelegramDeliveryData};

// src/tasks/email_sender/runner.rs
use lettre::{SmtpTransport, Transport, Message, message::header::ContentType};
use lettre::transport::smtp::authentication::Credentials;
use lettre::transport::smtp::client::{Tls, TlsParameters};

pub async fn send_email_for_feed_data(
    email_config: &EmailConfig,
    feed_data: &FeedData,
) -> Result<(), Box<dyn std::error::Error>> {
    let smtp_password = decrypt_password(&email_config.smtp_password)?;
    
    let email_html = format_email_message(feed_data);
    let subject = format!("📰 {} - {} new items", 
                         feed_data.feed_title, 
                         feed_data.new_items.len());
    
    let email = Message::builder()
        .from(format!("{} <{}>", 
              email_config.from_name.as_deref().unwrap_or("MailFeed"), 
              email_config.from_email).parse()?)
        .to(email_config.from_email.parse()?) // User's email for delivery
        .subject(subject)
        .header(ContentType::TEXT_HTML)
        .body(email_html)?;
    
    let creds = Credentials::new(
        email_config.smtp_username.clone(),
        smtp_password,
    );
    
    let mailer = if email_config.smtp_use_tls {
        SmtpTransport::relay(&email_config.smtp_host)?
            .port(email_config.smtp_port as u16)
            .credentials(creds)
            .build()
    } else {
        SmtpTransport::builder_dangerous(&email_config.smtp_host)
            .port(email_config.smtp_port as u16)
            .credentials(creds)
            .build()
    };
    
    mailer.send(&email)?;
    Ok(())
}

fn format_email_message(feed_data: &FeedData) -> String {
    let mut html = format!(
        r#"
        <!DOCTYPE html>
        <html>
        <head>
            <meta charset="UTF-8">
            <title>{}</title>
            <style>
                body {{ font-family: -apple-system, BlinkMacSystemFont, sans-serif; line-height: 1.6; color: #333; }}
                .header {{ background: #f8f9fa; padding: 20px; border-radius: 8px; margin-bottom: 20px; }}
                .feed-title {{ font-size: 24px; font-weight: bold; margin: 0; }}
                .feed-link {{ color: #007bff; text-decoration: none; }}
                .item {{ border-bottom: 1px solid #eee; padding: 20px 0; }}
                .item-title {{ font-size: 18px; font-weight: bold; margin-bottom: 8px; }}
                .item-title a {{ color: #007bff; text-decoration: none; }}
                .item-meta {{ color: #666; font-size: 14px; margin-bottom: 10px; }}
                .item-description {{ color: #555; }}
                .footer {{ margin-top: 30px; padding-top: 20px; border-top: 1px solid #eee; color: #666; font-size: 12px; }}
            </style>
        </head>
        <body>
            <div class="header">
                <h1 class="feed-title">📰 {}</h1>
                <a href="{}" class="feed-link">View Original Feed</a>
            </div>
        "#,
        html_escape(&feed_data.feed_title),
        html_escape(&feed_data.feed_title),
        feed_data.feed_link
    );
    
    for item in &feed_data.new_items {
        let date_time = chrono::Utc.timestamp_opt(item.pub_date as i64, 0).unwrap();
        
        html.push_str(&format!(
            r#"
            <div class="item">
                <h2 class="item-title">
                    <a href="{}">{}</a>
                </h2>
                <div class="item-meta">
                    🕐 {} {}
                </div>
                <div class="item-description">
                    {}
                </div>
            </div>
            "#,
            item.link,
            html_escape(&item.title),
            date_time.format("%Y-%m-%d %H:%M:%S"),
            item.author.as_ref().map(|a| format!("👤 {}", html_escape(a))).unwrap_or_default(),
            item.description.as_deref().unwrap_or("No description provided")
        ));
    }
    
    html.push_str(r#"
            <div class="footer">
                <p>This email was generated by MailFeed. To manage your subscriptions, visit your dashboard.</p>
            </div>
        </body>
        </html>
    "#);
    
    html
}

fn html_escape(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#x27;")
}
```

### Unified Delivery Coordinator
```rust
// Rename src/tasks/telegram_sender/runner.rs to src/tasks/delivery_coordinator/runner.rs
pub async fn start(pool: DbPool) {
    let mut interval = tokio::time::interval(CHECK_INTERVAL);
    loop {
        interval.tick().await;
        
        let mut conn = pool.get()?;
        
        // Get Telegram client (optional)
        let telegram_client = TelegramClient::new(&mut conn).ok();
        
        // Get all active users
        let users = User::get_all(&mut conn);
        let users = users.into_iter().flatten().filter(|user| user.is_active);
        
        for user in users {
            let delivery_data = items_to_send_by_user(&mut conn, user.id);
            
            for feed_data in &delivery_data.feed_data {
                if feed_data.new_items.is_empty() { continue; }
                
                let subscription = &feed_data.subscription;
                let mut sent_successfully = false;
                
                match subscription.delivery_method {
                    DeliveryMethod::TelegramOnly => {
                        if let (Some(ref client), Some(ref chat_id)) = (&telegram_client, &user.telegram_chat_id) {
                            if send_telegram_message(client, chat_id, feed_data).await.is_ok() {
                                sent_successfully = true;
                            }
                        }
                    },
                    DeliveryMethod::EmailOnly => {
                        if let Some(email_config) = EmailConfig::get_by_user_id(&mut conn, user.id) {
                            if send_email_for_feed_data(&email_config, feed_data).await.is_ok() {
                                sent_successfully = true;
                            }
                        }
                    },
                    DeliveryMethod::Both => {
                        let mut telegram_sent = false;
                        let mut email_sent = false;
                        
                        // Try Telegram
                        if let (Some(ref client), Some(ref chat_id)) = (&telegram_client, &user.telegram_chat_id) {
                            telegram_sent = send_telegram_message(client, chat_id, feed_data).await.is_ok();
                        }
                        
                        // Try Email  
                        if let Some(email_config) = EmailConfig::get_by_user_id(&mut conn, user.id) {
                            email_sent = send_email_for_feed_data(&email_config, feed_data).await.is_ok();
                        }
                        
                        sent_successfully = telegram_sent || email_sent;
                    }
                }
                
                if sent_successfully {
                    let update = PartialSubscription {
                        last_sent_time: Some(Utc::now().timestamp() as i32),
                        ..Default::default()
                    };
                    let _ = Subscription::update(&mut conn, feed_data.sub_id, &update);
                }
            }
        }
    }
}
```

## Security Implementation

### Password Encryption
```rust
// src/security/encryption.rs
use ring::aead::{Aad, LessSafeKey, Nonce, UnboundKey, AES_256_GCM, NONCE_LEN};
use ring::rand::{SecureRandom, SystemRandom};

pub struct PasswordEncryption {
    key: LessSafeKey,
    rng: SystemRandom,
}

impl PasswordEncryption {
    pub fn new(key_bytes: &[u8; 32]) -> Result<Self, String> {
        let unbound_key = UnboundKey::new(&AES_256_GCM, key_bytes)
            .map_err(|_| "Invalid encryption key")?;
        let key = LessSafeKey::new(unbound_key);
        
        Ok(Self {
            key,
            rng: SystemRandom::new(),
        })
    }
    
    pub fn encrypt(&self, plaintext: &str) -> Result<String, String> {
        let mut nonce_bytes = [0u8; NONCE_LEN];
        self.rng.fill(&mut nonce_bytes)
            .map_err(|_| "Failed to generate nonce")?;
        
        let nonce = Nonce::assume_unique_for_key(nonce_bytes);
        let mut ciphertext = plaintext.as_bytes().to_vec();
        
        self.key.seal_in_place_append_tag(nonce, Aad::empty(), &mut ciphertext)
            .map_err(|_| "Encryption failed")?;
        
        // Prepend nonce to ciphertext
        let mut result = nonce_bytes.to_vec();
        result.extend_from_slice(&ciphertext);
        
        Ok(base64::encode(result))
    }
    
    pub fn decrypt(&self, encrypted: &str) -> Result<String, String> {
        let data = base64::decode(encrypted)
            .map_err(|_| "Invalid base64")?;
        
        if data.len() < NONCE_LEN {
            return Err("Invalid encrypted data".into());
        }
        
        let (nonce_bytes, ciphertext) = data.split_at(NONCE_LEN);
        let nonce = Nonce::assume_unique_for_key(
            nonce_bytes.try_into().map_err(|_| "Invalid nonce")?
        );
        
        let mut plaintext = ciphertext.to_vec();
        let plaintext_bytes = self.key.open_in_place(nonce, Aad::empty(), &mut plaintext)
            .map_err(|_| "Decryption failed")?;
        
        String::from_utf8(plaintext_bytes.to_vec())
            .map_err(|_| "Invalid UTF-8")
    }
}
```

### Environment Configuration
```bash
# .env additions
MF_ENCRYPTION_KEY="your-32-byte-hex-encoded-encryption-key-here"
MF_EMAIL_RATE_LIMIT=100    # emails per hour per user
MF_SMTP_TIMEOUT=30         # seconds
MF_MAX_EMAIL_SIZE=10485760 # 10MB max email size
```

### Input Validation
```rust
// src/api/email/validation.rs
use regex::Regex;

pub fn validate_email_config(config: &EmailConfigForm) -> Result<(), String> {
    if config.smtp_host.trim().is_empty() {
        return Err("SMTP host is required".into());
    }
    
    if config.smtp_port == 0 || config.smtp_port > 65535 {
        return Err("SMTP port must be between 1 and 65535".into());
    }
    
    if config.smtp_username.trim().is_empty() {
        return Err("SMTP username is required".into());
    }
    
    if config.smtp_password.len() < 4 {
        return Err("SMTP password too short".into());
    }
    
    let email_regex = Regex::new(r"^[^\s@]+@[^\s@]+\.[^\s@]+$").unwrap();
    if !email_regex.is_match(&config.from_email) {
        return Err("Invalid from email format".into());
    }
    
    // Validate SMTP host format
    let host_regex = Regex::new(r"^[a-zA-Z0-9][a-zA-Z0-9.-]*[a-zA-Z0-9]$").unwrap();
    if !host_regex.is_match(&config.smtp_host) {
        return Err("Invalid SMTP host format".into());
    }
    
    Ok(())
}
```

## Dependencies

### Cargo.toml Additions
```toml
[dependencies]
# Email
lettre = "0.10"

# Encryption
ring = "0.16"
base64 = "0.21"

# Validation
regex = "1.0"
```

## Implementation Plan

### Phase 1: Database & Models
1. Create database migration for `email_configs` table and `delivery_method` column
2. Implement `EmailConfig` model with encryption/decryption
3. Extend `Subscription` model with `DeliveryMethod` enum
4. Update schema.rs

### Phase 2: API Endpoints
1. Create `src/api/email/` module with handlers and types
2. Implement CRUD operations for email configurations
3. Add test email functionality
4. Update subscription creation/editing to handle delivery method

### Phase 3: UI Implementation
1. Add email configuration section to settings page
2. Update subscription forms with delivery method selection
3. Add email status indicators to dashboard

### Phase 4: Email Delivery
1. Implement email formatting and sending logic
2. Create unified delivery coordinator
3. Add rate limiting and error handling
4. Update background task registration

### Phase 5: Security & Testing
1. Implement password encryption/decryption
2. Add comprehensive input validation
3. Test email delivery with common providers (Gmail, Outlook, etc.)
4. Security review and penetration testing

## Rollout Strategy

### Backward Compatibility
- Default `delivery_method` is `TelegramOnly` for existing subscriptions
- Existing Telegram functionality remains unchanged
- New users can choose any delivery method

### Migration Path
1. Deploy database migration
2. Deploy backend changes with feature flag
3. Deploy UI changes
4. Enable email delivery feature
5. Monitor for issues and performance impact

### Monitoring
- Track email delivery success/failure rates
- Monitor SMTP connection health
- Alert on encryption/decryption failures
- Log delivery method usage statistics

This design provides a comprehensive foundation for implementing email delivery while maintaining security, usability, and backward compatibility.