use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Serialize, Deserialize)]
pub struct ConfigItem {
    pub key: String,
    pub value: String,
    pub description: Option<String>,
    pub config_type: ConfigType,
    pub category: ConfigCategory,
    pub updated_at: i32,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ConfigUpdate {
    pub value: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ConfigBulkUpdate {
    pub updates: HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ConfigType {
    String,
    Number,
    Boolean,
    Select,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ConfigCategory {
    Telegram,
    FeedMonitoring,
    MessageDelivery,
    Authentication,
    System,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ConfigSchema {
    pub key: String,
    pub display_name: String,
    pub description: String,
    pub config_type: ConfigType,
    pub category: ConfigCategory,
    pub default_value: String,
    pub validation: Option<ConfigValidation>,
    pub options: Option<Vec<ConfigOption>>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ConfigValidation {
    pub min: Option<i32>,
    pub max: Option<i32>,
    pub pattern: Option<String>,
    pub required: bool,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ConfigOption {
    pub value: String,
    pub label: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ConfigResponse {
    pub config: HashMap<String, ConfigItem>,
    pub schema: Vec<ConfigSchema>,
}

// Predefined configuration schemas
pub fn get_config_schemas() -> Vec<ConfigSchema> {
    vec![
        // Telegram Settings
        ConfigSchema {
            key: "telegram_message_description_limit".to_string(),
            display_name: "Description Length Limit".to_string(),
            description: "Maximum length for feed item descriptions in Telegram messages".to_string(),
            config_type: ConfigType::Number,
            category: ConfigCategory::Telegram,
            default_value: "200".to_string(),
            validation: Some(ConfigValidation {
                min: Some(50),
                max: Some(1000),
                pattern: None,
                required: true,
            }),
            options: None,
        },
        ConfigSchema {
            key: "telegram_message_length_limit".to_string(),
            display_name: "Message Length Limit".to_string(),
            description: "Maximum total length for Telegram messages before truncation".to_string(),
            config_type: ConfigType::Number,
            category: ConfigCategory::Telegram,
            default_value: "3900".to_string(),
            validation: Some(ConfigValidation {
                min: Some(1000),
                max: Some(4096),
                pattern: None,
                required: true,
            }),
            options: None,
        },
        ConfigSchema {
            key: "telegram_disable_web_preview".to_string(),
            display_name: "Disable Web Page Preview".to_string(),
            description: "Disable automatic web page previews in Telegram messages".to_string(),
            config_type: ConfigType::Boolean,
            category: ConfigCategory::Telegram,
            default_value: "true".to_string(),
            validation: Some(ConfigValidation {
                min: None,
                max: None,
                pattern: None,
                required: true,
            }),
            options: None,
        },
        ConfigSchema {
            key: "telegram_message_format".to_string(),
            display_name: "Message Format".to_string(),
            description: "Choose between HTML and Markdown formatting for Telegram messages".to_string(),
            config_type: ConfigType::Select,
            category: ConfigCategory::Telegram,
            default_value: "html".to_string(),
            validation: Some(ConfigValidation {
                min: None,
                max: None,
                pattern: None,
                required: true,
            }),
            options: Some(vec![
                ConfigOption {
                    value: "html".to_string(),
                    label: "HTML".to_string(),
                },
                ConfigOption {
                    value: "markdown".to_string(),
                    label: "Markdown".to_string(),
                },
                ConfigOption {
                    value: "plain".to_string(),
                    label: "Plain Text".to_string(),
                },
            ]),
        },
        // Feed Monitoring Settings
        ConfigSchema {
            key: "feed_check_interval_seconds".to_string(),
            display_name: "Feed Check Interval".to_string(),
            description: "How often to check feeds for updates (in seconds)".to_string(),
            config_type: ConfigType::Number,
            category: ConfigCategory::FeedMonitoring,
            default_value: "30".to_string(),
            validation: Some(ConfigValidation {
                min: Some(10),
                max: Some(3600),
                pattern: None,
                required: true,
            }),
            options: None,
        },
        ConfigSchema {
            key: "feed_http_timeout_seconds".to_string(),
            display_name: "HTTP Timeout".to_string(),
            description: "Timeout for HTTP requests when fetching feeds (in seconds)".to_string(),
            config_type: ConfigType::Number,
            category: ConfigCategory::FeedMonitoring,
            default_value: "30".to_string(),
            validation: Some(ConfigValidation {
                min: Some(5),
                max: Some(120),
                pattern: None,
                required: true,
            }),
            options: None,
        },
        ConfigSchema {
            key: "feed_user_agent".to_string(),
            display_name: "User Agent".to_string(),
            description: "User-Agent header sent when fetching feeds".to_string(),
            config_type: ConfigType::String,
            category: ConfigCategory::FeedMonitoring,
            default_value: "Mailfeed (https://github.com/anson-vandoren/mailfeed)".to_string(),
            validation: Some(ConfigValidation {
                min: Some(10),
                max: Some(200),
                pattern: None,
                required: true,
            }),
            options: None,
        },
        // Message Delivery Settings
        ConfigSchema {
            key: "delivery_quiet_hours_start".to_string(),
            display_name: "Quiet Hours Start".to_string(),
            description: "Start of quiet hours (24-hour format, e.g., 23:00)".to_string(),
            config_type: ConfigType::String,
            category: ConfigCategory::MessageDelivery,
            default_value: "".to_string(),
            validation: Some(ConfigValidation {
                min: None,
                max: None,
                pattern: Some("^([01]?[0-9]|2[0-3]):[0-5][0-9]$".to_string()),
                required: false,
            }),
            options: None,
        },
        ConfigSchema {
            key: "delivery_quiet_hours_end".to_string(),
            display_name: "Quiet Hours End".to_string(),
            description: "End of quiet hours (24-hour format, e.g., 07:00)".to_string(),
            config_type: ConfigType::String,
            category: ConfigCategory::MessageDelivery,
            default_value: "".to_string(),
            validation: Some(ConfigValidation {
                min: None,
                max: None,
                pattern: Some("^([01]?[0-9]|2[0-3]):[0-5][0-9]$".to_string()),
                required: false,
            }),
            options: None,
        },
        ConfigSchema {
            key: "delivery_max_items_per_message".to_string(),
            display_name: "Max Items per Message".to_string(),
            description: "Maximum number of feed items to include in a single message".to_string(),
            config_type: ConfigType::Number,
            category: ConfigCategory::MessageDelivery,
            default_value: "5".to_string(),
            validation: Some(ConfigValidation {
                min: Some(1),
                max: Some(20),
                pattern: None,
                required: true,
            }),
            options: None,
        },
        // System Settings
        ConfigSchema {
            key: "telegram_bot_token".to_string(),
            display_name: "Telegram Bot Token".to_string(),
            description: "Bot token for Telegram integration. Get this from @BotFather on Telegram.".to_string(),
            config_type: ConfigType::String,
            category: ConfigCategory::System,
            default_value: "".to_string(),
            validation: Some(ConfigValidation {
                min: None,
                max: Some(100),
                pattern: Some("^[0-9]+:[A-Za-z0-9_-]+$".to_string()),
                required: false,
            }),
            options: None,
        },
        // Authentication Settings
        ConfigSchema {
            key: "jwt_access_token_duration_minutes".to_string(),
            display_name: "Access Token Duration".to_string(),
            description: "How long access tokens remain valid (in minutes)".to_string(),
            config_type: ConfigType::Number,
            category: ConfigCategory::Authentication,
            default_value: "15".to_string(),
            validation: Some(ConfigValidation {
                min: Some(5),
                max: Some(1440),
                pattern: None,
                required: true,
            }),
            options: None,
        },
        ConfigSchema {
            key: "jwt_refresh_token_duration_days".to_string(),
            display_name: "Refresh Token Duration".to_string(),
            description: "How long refresh tokens remain valid (in days)".to_string(),
            config_type: ConfigType::Number,
            category: ConfigCategory::Authentication,
            default_value: "7".to_string(),
            validation: Some(ConfigValidation {
                min: Some(1),
                max: Some(90),
                pattern: None,
                required: true,
            }),
            options: None,
        },
    ]
}