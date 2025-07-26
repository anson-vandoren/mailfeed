use super::types::{MessageResult, TelegramConfig, TelegramMessage, TelegramResponse};
use reqwest::Client;
use std::error::Error;

pub struct TelegramClient {
    client: Client,
    config: TelegramConfig,
}

impl TelegramClient {
    pub fn new() -> Result<Self, Box<dyn Error>> {
        let config = TelegramConfig::from_env();
        let client = Client::new();
        
        Ok(Self { client, config })
    }
    
    pub async fn send_message(
        &self,
        chat_id: &str,
        text: &str,
        parse_mode: Option<&str>,
    ) -> Result<MessageResult, Box<dyn Error>> {
        let message = TelegramMessage {
            chat_id: chat_id.to_string(),
            text: text.to_string(),
            parse_mode: parse_mode.map(|s| s.to_string()),
            disable_web_page_preview: Some(true),
        };
        
        let response = self
            .client
            .post(&self.config.send_message_url())
            .json(&message)
            .send()
            .await?;
        
        if !response.status().is_success() {
            let error_text = response.text().await?;
            return Err(format!("Telegram API error: {}", error_text).into());
        }
        
        let telegram_response: TelegramResponse<MessageResult> = response.json().await?;
        
        if !telegram_response.ok {
            let error_msg = telegram_response
                .description
                .unwrap_or_else(|| "Unknown Telegram API error".to_string());
            return Err(error_msg.into());
        }
        
        telegram_response
            .result
            .ok_or_else(|| "No result in Telegram response".into())
    }
    
    pub async fn send_html_message(
        &self,
        chat_id: &str,
        html_text: &str,
    ) -> Result<MessageResult, Box<dyn Error>> {
        self.send_message(chat_id, html_text, Some("HTML")).await
    }
    
    pub async fn send_markdown_message(
        &self,
        chat_id: &str,
        markdown_text: &str,
    ) -> Result<MessageResult, Box<dyn Error>> {
        self.send_message(chat_id, markdown_text, Some("MarkdownV2")).await
    }
}