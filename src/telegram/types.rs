use serde::{Deserialize, Serialize};
use crate::models::settings::Setting;
use diesel::SqliteConnection;

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


#[derive(Debug)]
pub struct TelegramConfig {
    pub bot_token: String,
    pub api_base_url: String,
}

impl TelegramConfig {
    pub fn from_database(conn: &mut SqliteConnection) -> Result<Self, Box<dyn std::error::Error>> {
        let bot_token = Setting::get(conn, "telegram_bot_token", None)?.value;

        let api_base_url = Setting::get(conn, "telegram_api_base_url", None)
            .map(|s| s.value)
            .unwrap_or_else(|_| "https://api.telegram.org".to_string());

        Ok(Self {
            bot_token,
            api_base_url,
        })
    }

    pub fn send_message_url(&self) -> String {
        format!("{}/bot{}/sendMessage", self.api_base_url, self.bot_token)
    }
}
