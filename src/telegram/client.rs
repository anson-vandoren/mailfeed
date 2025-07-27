use super::types::{TelegramConfig, TelegramMessage, TelegramResponse};
use reqwest::Client;
use std::error::Error;
use diesel::SqliteConnection;

pub struct TelegramClient {
    client: Client,
    config: TelegramConfig,
}

impl TelegramClient {
    pub fn new(conn: &mut SqliteConnection) -> Result<Self, Box<dyn Error>> {
        let config = TelegramConfig::from_database(conn)?;
        let client = Client::new();
        
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
            return Err(format!("Telegram API error: {}", error_text).into());
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