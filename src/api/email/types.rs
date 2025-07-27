use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize)]
pub struct EmailConfigForm {
    pub smtp_host: String,
    pub smtp_port: u16,
    pub smtp_username: String,
    pub smtp_password: String,
    pub smtp_use_tls: Option<String>, // HTML checkbox sends "on" or nothing
    pub from_email: String,
    pub from_name: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct EmailConfigResponse {
    pub id: Option<i32>,
    pub user_id: i32,
    pub smtp_host: String,
    pub smtp_port: i32,
    pub smtp_username: String,
    pub from_email: String,
    pub from_name: Option<String>,
    pub smtp_use_tls: bool,
    pub is_active: bool,
    // Note: We never return the password in responses
}

#[derive(Debug, Serialize)]
pub struct TestEmailResponse {
    pub success: bool,
    pub message: String,
}

impl EmailConfigForm {
    /// Validate the email configuration form
    pub fn validate(&self) -> Result<(), String> {
        if self.smtp_host.trim().is_empty() {
            return Err("SMTP host is required".into());
        }
        
        if self.smtp_port == 0 {
            return Err("SMTP port must be between 1 and 65535".into());
        }
        
        if self.smtp_username.trim().is_empty() {
            return Err("SMTP username is required".into());
        }
        
        if self.smtp_password.trim().is_empty() {
            return Err("SMTP password is required".into());
        }
        
        if self.from_email.trim().is_empty() {
            return Err("From email is required".into());
        }
        
        // Validate email format using security module
        crate::security::validation::validate_email(&self.from_email)?;
        
        // Validate SMTP host format
        use regex::Regex;
        let host_regex = Regex::new(r"^[a-zA-Z0-9][a-zA-Z0-9.-]*[a-zA-Z0-9]$").unwrap();
        if !host_regex.is_match(&self.smtp_host) {
            return Err("Invalid SMTP host format".into());
        }
        
        Ok(())
    }
    
    /// Check if TLS should be enabled (checkbox value)
    pub fn use_tls(&self) -> bool {
        self.smtp_use_tls.as_deref() == Some("on")
    }
}