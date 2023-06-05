use std::env;

#[derive(Debug)]
pub struct EmailServerCfg {
    pub host: String,
    pub port: u16,
    pub username: String,
    pub password: String,
    pub from_email: String,
}

impl EmailServerCfg {
    pub fn from_env() -> Self {
        let host = env::var("MF_SMTP_HOST").unwrap();
        let port = env::var("MF_SMTP_PORT").unwrap().parse::<u16>().unwrap();
        let username = env::var("MF_SMTP_USERNAME").unwrap();
        let password = env::var("MF_SMTP_PASSWORD").unwrap();
        let from_email = env::var("MF_FROM_EMAIL").unwrap();
        EmailServerCfg {
            host,
            port,
            username,
            password,
            from_email,
        }
    }
}
