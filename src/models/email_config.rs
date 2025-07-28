use crate::schema::email_configs;
use crate::security::encryption::{encrypt_password, decrypt_password};
use diesel::prelude::*;
use serde::{Deserialize, Serialize};
use chrono::Utc;

#[derive(Debug, Serialize, Deserialize, Queryable, Identifiable)]
#[diesel(table_name = email_configs)]
pub struct EmailConfig {
    pub id: Option<i32>,
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
    pub smtp_password: String, // Will be encrypted before insertion
    pub smtp_use_tls: bool,
    pub from_email: String,
    pub from_name: Option<String>,
    pub is_active: bool,
    pub created_at: i32,
    pub updated_at: i32,
}

#[derive(Debug, AsChangeset)]
#[diesel(table_name = email_configs)]
pub struct PartialEmailConfig {
    pub smtp_host: Option<String>,
    pub smtp_port: Option<i32>,
    pub smtp_username: Option<String>,
    pub smtp_password: Option<String>, // Will be encrypted before update
    pub smtp_use_tls: Option<bool>,
    pub from_email: Option<String>,
    pub from_name: Option<String>,
    pub is_active: Option<bool>,
    pub updated_at: Option<i32>,
}

impl EmailConfig {
    /// Get email config by user ID
    pub fn get_by_user_id(conn: &mut SqliteConnection, target_user_id: i32) -> Option<EmailConfig> {
        use crate::schema::email_configs::dsl::*;
        
        match email_configs
            .filter(user_id.eq(target_user_id))
            .filter(is_active.eq(true))
            .first::<EmailConfig>(conn)
        {
            Ok(config) => Some(config),
            Err(diesel::result::Error::NotFound) => None,
            Err(e) => {
                log::warn!("Error getting email config for user {target_user_id}: {e:?}");
                None
            }
        }
    }
    
    /// Get decrypted SMTP password
    pub fn get_decrypted_password(&self) -> Result<String, String> {
        decrypt_password(&self.smtp_password)
    }
    
    /// Update an existing email config
    pub fn update(
        conn: &mut SqliteConnection,
        target_user_id: i32,
        update: &PartialEmailConfig,
    ) -> Result<EmailConfig, diesel::result::Error> {
        use crate::schema::email_configs::dsl::*;
        
        diesel::update(email_configs.filter(user_id.eq(target_user_id)))
            .set(update)
            .get_result(conn)
    }
    
    /// Delete email config for user
    pub fn delete(conn: &mut SqliteConnection, target_user_id: i32) -> Result<usize, diesel::result::Error> {
        use crate::schema::email_configs::dsl::*;
        
        diesel::delete(email_configs.filter(user_id.eq(target_user_id)))
            .execute(conn)
    }
}

/// Configuration parameters for creating a new EmailConfig
#[derive(Debug)]
pub struct EmailConfigParams {
    pub smtp_host: String,
    pub smtp_port: i32,
    pub smtp_username: String,
    pub plain_password: String,
    pub smtp_use_tls: bool,
    pub from_email: String,
    pub from_name: Option<String>,
}

impl NewEmailConfig {
    /// Create a new email config with encrypted password
    pub fn new(user_id: i32, params: EmailConfigParams) -> Result<Self, String> {
        let encrypted_password = encrypt_password(&params.plain_password)?;
        let now = Utc::now().timestamp() as i32;
        
        Ok(Self {
            user_id,
            smtp_host: params.smtp_host,
            smtp_port: params.smtp_port,
            smtp_username: params.smtp_username,
            smtp_password: encrypted_password,
            smtp_use_tls: params.smtp_use_tls,
            from_email: params.from_email,
            from_name: params.from_name,
            is_active: true,
            created_at: now,
            updated_at: now,
        })
    }
    
    /// Insert the email config into the database
    pub fn insert(self, conn: &mut SqliteConnection) -> Result<EmailConfig, diesel::result::Error> {
        use crate::schema::email_configs::dsl::*;
        
        diesel::insert_into(email_configs)
            .values(&self)
            .get_result(conn)
    }
}

/// Configuration parameters for updating an EmailConfig
#[derive(Debug, Default)]
pub struct PartialEmailConfigParams {
    pub smtp_host: Option<String>,
    pub smtp_port: Option<i32>,
    pub smtp_username: Option<String>,
    pub plain_password: Option<String>,
    pub smtp_use_tls: Option<bool>,
    pub from_email: Option<String>,
    pub from_name: Option<String>,
    pub is_active: Option<bool>,
}

impl PartialEmailConfig {
    /// Create a partial update with encrypted password if provided
    pub fn new_with_password(params: PartialEmailConfigParams) -> Result<Self, String> {
        let encrypted_password = match params.plain_password.as_deref() {
            Some(password) => Some(encrypt_password(password)?),
            None => None,
        };
        
        Ok(Self {
            smtp_host: params.smtp_host,
            smtp_port: params.smtp_port,
            smtp_username: params.smtp_username,
            smtp_password: encrypted_password,
            smtp_use_tls: params.smtp_use_tls,
            from_email: params.from_email,
            from_name: params.from_name,
            is_active: params.is_active,
            updated_at: Some(Utc::now().timestamp() as i32),
        })
    }
}

/// Validation for email configuration parameters
impl EmailConfigParams {
    pub fn validate(&self) -> Result<(), String> {
        if self.smtp_host.trim().is_empty() {
            return Err("SMTP host is required".into());
        }
        
        if self.smtp_port <= 0 || self.smtp_port > 65535 {
            return Err("SMTP port must be between 1 and 65535".into());
        }
        
        if self.smtp_username.trim().is_empty() {
            return Err("SMTP username is required".into());
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
}

/// Validation for email configuration
impl NewEmailConfig {
    pub fn validate(&self) -> Result<(), String> {
        if self.smtp_host.trim().is_empty() {
            return Err("SMTP host is required".into());
        }
        
        if self.smtp_port <= 0 || self.smtp_port > 65535 {
            return Err("SMTP port must be between 1 and 65535".into());
        }
        
        if self.smtp_username.trim().is_empty() {
            return Err("SMTP username is required".into());
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
}