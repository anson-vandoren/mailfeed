use crate::models::{email_config::EmailConfig, feed_item::FeedItem};
use crate::security::encryption;
use lettre::message::{header::ContentType, Mailbox};
use lettre::transport::smtp::authentication::Credentials;
use lettre::{Message, SmtpTransport, Transport};
use log::{debug, error, info, warn};
use std::collections::HashMap;
use std::str::FromStr;

pub struct EmailDeliveryService {
    // Cache SMTP connections by user_id for efficiency
    connection_cache: HashMap<i32, SmtpTransport>,
}

impl EmailDeliveryService {
    pub fn new() -> Self {
        Self {
            connection_cache: HashMap::new(),
        }
    }

    /// Send a single feed item via email to a user
    pub async fn send_feed_item(
        &mut self,
        email_config: &EmailConfig,
        feed_item: &FeedItem,
        feed_title: &str,
    ) -> Result<(), String> {
        debug!(
            "Sending email for feed item {} to user {}",
            feed_item.id, email_config.user_id
        );

        // Build email content
        let subject = format!("[{}] {}", feed_title, feed_item.title);
        let body = self.build_email_body(feed_item, feed_title);

        // Send the email
        self.send_email(email_config, &subject, &body).await
    }

    /// Send multiple feed items as a digest email
    pub async fn send_digest(
        &mut self,
        email_config: &EmailConfig,
        feed_items: &[(FeedItem, String)], // (item, feed_title)
        frequency: &str,
    ) -> Result<(), String> {
        if feed_items.is_empty() {
            debug!("No feed items to send in digest for user {}", email_config.user_id);
            return Ok(());
        }

        debug!(
            "Sending {} digest with {} items to user {}",
            frequency,
            feed_items.len(),
            email_config.user_id
        );

        let subject = format!("MailFeed {} Digest - {} new items", 
                            frequency.to_ascii_uppercase(), 
                            feed_items.len());
        let body = self.build_digest_body(feed_items, frequency);

        self.send_email(email_config, &subject, &body).await
    }

    /// Send a test email to verify configuration
    pub async fn send_test_email(&mut self, email_config: &EmailConfig) -> Result<(), String> {
        debug!("Sending test email to user {}", email_config.user_id);

        let subject = "MailFeed Test Email".to_string();
        let body = r#"
<html>
<body style="font-family: Arial, sans-serif; max-width: 600px; margin: 0 auto; padding: 20px;">
    <h2 style="color: #10b981;">📧 MailFeed Test Email</h2>
    <p>Hello!</p>
    <p>This is a test email from your MailFeed instance to verify that your email configuration is working correctly.</p>
    <p><strong>Configuration Details:</strong></p>
    <ul>
        <li><strong>SMTP Host:</strong> {}</li>
        <li><strong>Port:</strong> {}</li>
        <li><strong>TLS:</strong> {}</li>
        <li><strong>From Email:</strong> {}</li>
    </ul>
    <p>If you received this email, your email delivery is configured properly! 🎉</p>
    <hr style="margin: 20px 0; border: none; border-top: 1px solid #e5e7eb;">
    <p style="font-size: 12px; color: #6b7280;">
        This email was sent by MailFeed, your self-hosted RSS-to-email service.
    </p>
</body>
</html>
        "#.trim();

        let formatted_body = body
            .replace("{}", &email_config.smtp_host)
            .replace("{}", &email_config.smtp_port.to_string())
            .replace("{}", if email_config.smtp_use_tls { "Enabled" } else { "Disabled" })
            .replace("{}", &email_config.from_email);

        self.send_email(email_config, &subject, &formatted_body).await
    }

    /// Core email sending logic
    async fn send_email(
        &mut self,
        email_config: &EmailConfig,
        subject: &str,
        html_body: &str,
    ) -> Result<(), String> {
        // Decrypt the password
        let smtp_password = encryption::decrypt_password(&email_config.smtp_password)
            .map_err(|e| format!("Failed to decrypt SMTP password: {}", e))?;

        // Build the email message
        let from_mailbox = match email_config.from_name.as_ref() {
            Some(name) => Mailbox::from_str(&format!("{} <{}>", name, email_config.from_email))
                .map_err(|e| format!("Invalid from address: {}", e))?,
            None => Mailbox::from_str(&email_config.from_email)
                .map_err(|e| format!("Invalid from address: {}", e))?,
        };

        // For now, send to the from_email (user's own email)
        // In a real deployment, you might want users to specify a separate "to" email
        let to_mailbox = Mailbox::from_str(&email_config.from_email)
            .map_err(|e| format!("Invalid to address: {}", e))?;

        let email = Message::builder()
            .from(from_mailbox)
            .to(to_mailbox)
            .subject(subject)
            .header(ContentType::TEXT_HTML)
            .body(html_body.to_string())
            .map_err(|e| format!("Failed to build email: {}", e))?;

        // Get or create SMTP transport
        let transport = self.get_smtp_transport(email_config, &smtp_password)?;

        // Send the email
        match transport.send(&email) {
            Ok(_) => {
                info!("Email sent successfully to {}", email_config.from_email);
                Ok(())
            }
            Err(e) => {
                error!("Failed to send email: {}", e);
                // Remove from cache if connection failed
                self.connection_cache.remove(&email_config.user_id);
                Err(format!("Failed to send email: {}", e))
            }
        }
    }

    /// Get or create an SMTP transport for the user
    fn get_smtp_transport(
        &mut self,
        email_config: &EmailConfig,
        smtp_password: &str,
    ) -> Result<&SmtpTransport, String> {
        // Check if we have a cached connection
        if !self.connection_cache.contains_key(&email_config.user_id) {
            debug!("Creating new SMTP connection for user {}", email_config.user_id);

            let credentials = Credentials::new(
                email_config.smtp_username.clone(),
                smtp_password.to_string(),
            );

            let transport_builder = SmtpTransport::relay(&email_config.smtp_host)
                .map_err(|e| format!("Failed to create SMTP relay: {}", e))?
                .credentials(credentials)
                .port(email_config.smtp_port as u16);

            // Configure TLS - using the default which enables TLS for most providers
            if !email_config.smtp_use_tls {
                warn!("TLS disabled for SMTP connection for user {}", email_config.user_id);
                // Note: lettre 0.10 handles TLS automatically for most cases
            }

            let transport = transport_builder
                .build();

            self.connection_cache.insert(email_config.user_id, transport);
        }

        Ok(self.connection_cache.get(&email_config.user_id).unwrap())
    }

    /// Build HTML email body for a single feed item
    fn build_email_body(&self, feed_item: &FeedItem, feed_title: &str) -> String {
        let content = feed_item.description.as_deref()
            .unwrap_or("No content available");

        format!(
            r#"
<html>
<body style="font-family: Arial, sans-serif; max-width: 600px; margin: 0 auto; padding: 20px; line-height: 1.6;">
    <div style="background: #f8fafc; padding: 20px; border-radius: 8px; margin-bottom: 20px;">
        <h2 style="color: #1f2937; margin: 0 0 10px 0;">{}</h2>
        <p style="color: #6b7280; margin: 0; font-size: 14px;">From: {}</p>
        <p style="color: #6b7280; margin: 0; font-size: 14px;">Published: {}</p>
    </div>
    
    <div style="margin-bottom: 20px;">
        <h1 style="color: #1f2937; margin: 0 0 15px 0; font-size: 24px;">{}</h1>
        <div style="color: #374151; font-size: 16px;">
            {}
        </div>
    </div>
    
    <div style="margin: 20px 0; padding: 15px; background: #eff6ff; border-left: 4px solid #3b82f6; border-radius: 4px;">
        <p style="margin: 0; font-weight: 500;">
            📖 <a href="{}" style="color: #3b82f6; text-decoration: none;">Read the full article</a>
        </p>
    </div>
    
    <hr style="margin: 30px 0; border: none; border-top: 1px solid #e5e7eb;">
    <p style="font-size: 12px; color: #6b7280; margin: 0;">
        This email was sent by MailFeed, your self-hosted RSS-to-email service.
        <br>Delivered at: {}
    </p>
</body>
</html>
            "#,
            feed_title,
            feed_title,
            chrono::DateTime::from_timestamp(feed_item.pub_date as i64, 0)
                .unwrap_or_default()
                .format("%Y-%m-%d %H:%M UTC"),
            feed_item.title,
            content,
            &feed_item.link,
            chrono::Utc::now().format("%Y-%m-%d %H:%M UTC")
        )
    }

    /// Build HTML email body for a digest of multiple feed items
    fn build_digest_body(&self, feed_items: &[(FeedItem, String)], frequency: &str) -> String {
        let mut items_html = String::new();
        
        for (item, feed_title) in feed_items {
            let summary = item.description.as_deref()
                .map(|s| {
                    // Truncate to first 200 characters for digest
                    if s.len() > 200 {
                        format!("{}...", &s[..200])
                    } else {
                        s.to_string()
                    }
                })
                .unwrap_or_else(|| "No summary available".to_string());

            items_html.push_str(&format!(
                r#"
    <div style="margin-bottom: 30px; border-bottom: 1px solid #e5e7eb; padding-bottom: 20px;">
        <h3 style="margin: 0 0 10px 0; color: #1f2937;">
            <a href="{}" style="color: #1f2937; text-decoration: none;">{}</a>
        </h3>
        <p style="margin: 0 0 10px 0; color: #6b7280; font-size: 14px;">
            📰 {} • 📅 {}
        </p>
        <p style="margin: 0; color: #374151; line-height: 1.5;">
            {}
        </p>
    </div>
                "#,
                &item.link,
                item.title,
                feed_title,
                chrono::DateTime::from_timestamp(item.pub_date as i64, 0)
                    .unwrap_or_default()
                    .format("%Y-%m-%d %H:%M"),
                summary
            ));
        }

        format!(
            r#"
<html>
<body style="font-family: Arial, sans-serif; max-width: 600px; margin: 0 auto; padding: 20px; line-height: 1.6;">
    <div style="background: #f8fafc; padding: 20px; border-radius: 8px; margin-bottom: 30px; text-align: center;">
        <h1 style="color: #1f2937; margin: 0 0 10px 0;">📧 MailFeed {} Digest</h1>
        <p style="color: #6b7280; margin: 0; font-size: 16px;">{} new articles from your subscriptions</p>
        <p style="color: #6b7280; margin: 5px 0 0 0; font-size: 14px;">Generated on {}</p>
    </div>
    
    <div>
        {}
    </div>
    
    <hr style="margin: 30px 0; border: none; border-top: 1px solid #e5e7eb;">
    <p style="font-size: 12px; color: #6b7280; margin: 0; text-align: center;">
        This digest was sent by MailFeed, your self-hosted RSS-to-email service.
        <br>You can manage your subscriptions and email settings in your MailFeed dashboard.
    </p>
</body>
</html>
            "#,
            frequency.to_ascii_uppercase(),
            feed_items.len(),
            chrono::Utc::now().format("%Y-%m-%d %H:%M UTC"),
            items_html
        )
    }

    /// Clear the connection cache (useful for cleanup)
    pub fn clear_cache(&mut self) {
        self.connection_cache.clear();
    }
}

impl Default for EmailDeliveryService {
    fn default() -> Self {
        Self::new()
    }
}