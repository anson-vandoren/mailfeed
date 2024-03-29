use std::env;

use crate::models::feed_item::FeedItem;
use lettre::{transport::smtp::authentication::Credentials, SmtpTransport};

#[derive(Debug)]
pub struct EmailServerCfg {
    pub host: String,
    pub port: u16,
    pub username: String,
    pub password: String,
    pub from_email: String,
    pub email_subject: String,
}

impl EmailServerCfg {
    pub fn from_env() -> Self {
        let host = env::var("MF_SMTP_HOST").unwrap();
        let port = env::var("MF_SMTP_PORT").unwrap().parse::<u16>().unwrap();
        let username = env::var("MF_SMTP_USERNAME").unwrap();
        let password = env::var("MF_SMTP_PASSWORD").unwrap();
        let from_email = env::var("MF_FROM_EMAIL").unwrap();
        let email_subject = env::var("MF_EMAIL_SUBJECT").unwrap_or("MailFeed Digest".to_string());
        EmailServerCfg {
            host,
            port,
            username,
            password,
            from_email,
            email_subject,
        }
    }

    pub fn to_transport(&self) -> Result<SmtpTransport, lettre::transport::smtp::Error> {
        SmtpTransport::relay(&self.host)
            .map(|sender| {
                sender.credentials(Credentials::new(
                    self.username.clone(),
                    self.password.clone(),
                ))
            })
            .map(|sender| sender.build())
    }
}

#[derive(Debug)]
pub struct FeedData {
    pub sub_id: i32,
    pub new_items: Vec<FeedItem>,
    pub feed_title: String,
    pub feed_link: String,
}

#[derive(Debug)]
pub struct EmailData {
    pub feed_data: Vec<FeedData>,
}

pub type ToEmail<'a> = &'a str;
pub type FromEmail<'a> = &'a str;

pub struct MultiPartEmailContent<'a> {
    pub as_html: &'a str,
    pub as_plain: &'a str,
}
