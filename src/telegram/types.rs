use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct TelegramMessage {
    pub chat_id: String,
    pub text: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parse_mode: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub disable_web_page_preview: Option<bool>,
}

#[derive(Debug, Deserialize)]
pub struct TelegramResponse<T> {
    pub ok: bool,
    pub result: Option<T>,
    pub description: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct MessageResult {
    pub message_id: i64,
    pub date: i64,
}

#[derive(Debug)]
pub struct TelegramConfig {
    pub bot_token: String,
    pub api_base_url: String,
}

impl TelegramConfig {
    pub fn from_env() -> Self {
        let bot_token = std::env::var("TELEGRAM_BOT_TOKEN")
            .expect("TELEGRAM_BOT_TOKEN environment variable must be set");
        
        let api_base_url = std::env::var("TELEGRAM_API_BASE_URL")
            .unwrap_or_else(|_| "https://api.telegram.org".to_string());
        
        Self {
            bot_token,
            api_base_url,
        }
    }
    
    pub fn send_message_url(&self) -> String {
        format!("{}/bot{}/sendMessage", self.api_base_url, self.bot_token)
    }
}