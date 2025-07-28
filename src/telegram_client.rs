use crate::models::settings::Setting;
use diesel::SqliteConnection;
use serde::{Deserialize, Serialize};
use std::error::Error;

pub struct TelegramClient {
    client: reqwest::Client,
    config: TelegramConfig,
}

impl TelegramClient {
    pub fn new(conn: &mut SqliteConnection) -> Result<Self, Box<dyn Error>> {
        let config = TelegramConfig::from_database(conn)?;
        let client = reqwest::Client::new();

        Ok(Self { client, config })
    }

    pub async fn send_message(
        &self,
        chat_id: &str,
        text: &str,
        parse_mode: Option<&str>,
    ) -> Result<(), Box<dyn Error>> {
        let message = TelegramMessage {
            chat_id: chat_id.to_string(),
            text: text.to_string(),
            parse_mode: parse_mode.map(|s| s.to_string()),
            disable_web_page_preview: Some(true),
        };

        let response = self
            .client
            .post(self.config.send_message_url())
            .json(&message)
            .send()
            .await?;

        if !response.status().is_success() {
            let error_text = response.text().await?;
            return Err(format!("Telegram API error: {error_text}").into());
        }

        let telegram_response: TelegramResponse<serde_json::Value> = response.json().await?;

        if !telegram_response.ok {
            let error_msg = telegram_response
                .description
                .unwrap_or_else(|| "Unknown Telegram API error".to_string());
            return Err(error_msg.into());
        }

        if telegram_response.result.is_some() {
            Ok(())
        } else {
            Err("No result in Telegram response".into())
        }
    }

    pub async fn send_html_message(
        &self,
        chat_id: &str,
        html_text: &str,
    ) -> Result<(), Box<dyn Error>> {
        self.send_message(chat_id, html_text, Some("HTML")).await
    }
}

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
