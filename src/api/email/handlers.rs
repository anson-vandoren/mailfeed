use actix_web::{web, HttpResponse};
use crate::{
    errors::{AppError, AppResult},
    models::email_config::{EmailConfig, NewEmailConfig, PartialEmailConfig},
    session::SessionClaims,
    RqDbPool,
};
use super::types::{EmailConfigForm, EmailConfigResponse};

/// Create or update email configuration for a user
#[actix_web::post("/api/users/{user_id}/email-config")]
pub async fn create_or_update_email_config(
    pool: RqDbPool,
    path: web::Path<i32>,
    form: web::Form<EmailConfigForm>,
    claims: SessionClaims,
) -> AppResult<HttpResponse> {
    let target_user_id = path.into_inner();
    
    // Check authorization - users can only update their own config
    if claims.sub != target_user_id && claims.role != "admin" {
        return Err(AppError::Forbidden);
    }
    
    // Validate the form
    form.validate()
        .map_err(|e| AppError::invalid_input("email_config", &e))?;
    
    let mut conn = pool.get().map_err(|_| AppError::ConnectionPoolError)?;
    
    // Check if config already exists
    let existing = EmailConfig::get_by_user_id(&mut conn, target_user_id);
    
    let response = if existing.is_some() {
        // Update existing config
        let params = crate::models::email_config::PartialEmailConfigParams {
            smtp_host: Some(form.smtp_host.clone()),
            smtp_port: Some(form.smtp_port as i32),
            smtp_username: Some(form.smtp_username.clone()),
            plain_password: Some(form.smtp_password.clone()),
            smtp_use_tls: Some(form.use_tls()),
            from_email: Some(form.from_email.clone()),
            from_name: form.from_name.clone(),
            is_active: Some(true),
        };
        
        let partial_config = PartialEmailConfig::new_with_password(params)
            .map_err(|_e| AppError::InternalError)?;
        
        EmailConfig::update(&mut conn, target_user_id, &partial_config)
            .map_err(|_| AppError::DatabaseError)?
    } else {
        // Create new config
        let params = crate::models::email_config::EmailConfigParams {
            smtp_host: form.smtp_host.clone(),
            smtp_port: form.smtp_port as i32,
            smtp_username: form.smtp_username.clone(),
            plain_password: form.smtp_password.clone(),
            smtp_use_tls: form.use_tls(),
            from_email: form.from_email.clone(),
            from_name: form.from_name.clone(),
        };
        
        // Validate parameters
        params.validate()
            .map_err(|e| AppError::invalid_input("email_config", &e))?;
        
        let new_config = NewEmailConfig::new(target_user_id, params)
            .map_err(|_e| AppError::InternalError)?;
        
        new_config.insert(&mut conn)
            .map_err(|_| AppError::DatabaseError)?
    };
    
    // Convert to response (without password) - for future use
    let _config_response = EmailConfigResponse {
        id: response.id,
        user_id: response.user_id,
        smtp_host: response.smtp_host,
        smtp_port: response.smtp_port,
        smtp_username: response.smtp_username,
        from_email: response.from_email,
        from_name: response.from_name,
        smtp_use_tls: response.smtp_use_tls,
        is_active: response.is_active,
    };
    
    // Return success message for HTMX
    Ok(HttpResponse::Ok()
        .content_type("text/html")
        .body(r#"<div style="padding: 1rem; background: #d1fae5; border: 1px solid #10b981; border-radius: var(--pico-border-radius); margin-bottom: 1rem;">
            <h4 style="margin: 0 0 0.5rem 0; color: #065f46;">✅ Email Configuration Saved</h4>
            <p style="margin: 0; color: #047857;">Your SMTP settings have been saved successfully. You can now receive feed updates via email.</p>
        </div>"#))
}

/// Update existing email configuration (PATCH)
#[actix_web::patch("/api/users/{user_id}/email-config")]
pub async fn update_email_config(
    pool: RqDbPool,
    path: web::Path<i32>,
    form: web::Form<EmailConfigForm>,
    claims: SessionClaims,
) -> AppResult<HttpResponse> {
    let target_user_id = path.into_inner();
    
    // Check authorization - users can only update their own config
    if claims.sub != target_user_id && claims.role != "admin" {
        return Err(AppError::Forbidden);
    }
    
    // Validate the form
    form.validate()
        .map_err(|e| AppError::invalid_input("email_config", &e))?;
    
    let mut conn = pool.get().map_err(|_| AppError::ConnectionPoolError)?;
    
    // Create partial update
    let params = crate::models::email_config::PartialEmailConfigParams {
        smtp_host: Some(form.smtp_host.clone()),
        smtp_port: Some(form.smtp_port as i32),
        smtp_username: Some(form.smtp_username.clone()),
        plain_password: Some(form.smtp_password.clone()),
        smtp_use_tls: Some(form.use_tls()),
        from_email: Some(form.from_email.clone()),
        from_name: form.from_name.clone(),
        is_active: Some(true),
    };
    
    let partial_config = PartialEmailConfig::new_with_password(params)
        .map_err(|_e| AppError::InternalError)?;
    
    let _response = EmailConfig::update(&mut conn, target_user_id, &partial_config)
        .map_err(|_| AppError::DatabaseError)?;
    
    // Return success message for HTMX
    Ok(HttpResponse::Ok()
        .content_type("text/html")
        .body(r#"<div style="padding: 1rem; background: #d1fae5; border: 1px solid #10b981; border-radius: var(--pico-border-radius); margin-bottom: 1rem;">
            <h4 style="margin: 0 0 0.5rem 0; color: #065f46;">✅ Email Configuration Updated</h4>
            <p style="margin: 0; color: #047857;">Your SMTP settings have been updated successfully.</p>
        </div>"#))
}

/// Delete email configuration
#[actix_web::delete("/api/users/{user_id}/email-config")]
pub async fn delete_email_config(
    pool: RqDbPool,
    path: web::Path<i32>,
    claims: SessionClaims,
) -> AppResult<HttpResponse> {
    let target_user_id = path.into_inner();
    
    // Check authorization
    if claims.sub != target_user_id && claims.role != "admin" {
        return Err(AppError::Forbidden);
    }
    
    let mut conn = pool.get().map_err(|_| AppError::ConnectionPoolError)?;
    
    EmailConfig::delete(&mut conn, target_user_id)
        .map_err(|_| AppError::DatabaseError)?;
    
    // Return success message for HTMX
    Ok(HttpResponse::Ok()
        .content_type("text/html")
        .body(r#"<div style="padding: 1rem; background: #fef3c7; border: 1px solid #f59e0b; border-radius: var(--pico-border-radius); margin-bottom: 1rem;">
            <h4 style="margin: 0 0 0.5rem 0; color: #92400e;">⚠️ Email Configuration Removed</h4>
            <p style="margin: 0; color: #92400e;">Your email configuration has been deleted. You will no longer receive feed updates via email.</p>
        </div>"#))
}

/// Send a test email to verify configuration
#[actix_web::post("/api/users/{user_id}/test-email")]
pub async fn send_test_email(
    pool: RqDbPool,
    path: web::Path<i32>,
    claims: SessionClaims,
) -> AppResult<HttpResponse> {
    let target_user_id = path.into_inner();
    
    // Check authorization
    if claims.sub != target_user_id && claims.role != "admin" {
        return Err(AppError::Forbidden);
    }
    
    let mut conn = pool.get().map_err(|_| AppError::ConnectionPoolError)?;
    
    // Get email config
    let email_config = EmailConfig::get_by_user_id(&mut conn, target_user_id)
        .ok_or_else(|| AppError::resource_not_found("Email configuration"))?;
    
    // Try to send test email
    match send_test_email_impl(&email_config).await {
        Ok(_) => {
            Ok(HttpResponse::Ok()
                .content_type("text/html")
                .body(r#"<div style="padding: 1rem; background: #d1fae5; border: 1px solid #10b981; border-radius: var(--pico-border-radius); margin-bottom: 1rem;">
                    <h4 style="margin: 0 0 0.5rem 0; color: #065f46;">✅ Test Email Sent</h4>
                    <p style="margin: 0; color: #047857;">A test email has been sent to your configured email address. Please check your inbox.</p>
                </div>"#))
        }
        Err(e) => {
            Ok(HttpResponse::Ok()
                .content_type("text/html")
                .body(format!(r#"<div style="padding: 1rem; background: #fee2e2; border: 1px solid #ef4444; border-radius: var(--pico-border-radius); margin-bottom: 1rem;">
                    <h4 style="margin: 0 0 0.5rem 0; color: #991b1b;">❌ Test Email Failed</h4>
                    <p style="margin: 0; color: #991b1b;">Failed to send test email: {}</p>
                    <p style="margin: 0.5rem 0 0 0; font-size: 0.875rem; color: #991b1b;">Please check your SMTP settings and try again.</p>
                </div>"#, html_escape::encode_text(&e))))
        }
    }
}

/// Implementation of test email sending
async fn send_test_email_impl(email_config: &EmailConfig) -> Result<(), String> {
    use lettre::{Message, SmtpTransport, Transport};
    use lettre::transport::smtp::authentication::Credentials;
    use lettre::message::header::ContentType;
    
    // Decrypt password
    let smtp_password = email_config.get_decrypted_password()?;
    
    // Build email
    let email = Message::builder()
        .from(format!("{} <{}>", 
              email_config.from_name.as_deref().unwrap_or("MailFeed"), 
              email_config.from_email).parse().map_err(|e| format!("Invalid from address: {e}"))?)
        .to(email_config.from_email.parse().map_err(|e| format!("Invalid to address: {e}"))?)
        .subject("MailFeed Test Email")
        .header(ContentType::TEXT_HTML)
        .body(r#"
            <!DOCTYPE html>
            <html>
            <head>
                <meta charset="UTF-8">
                <title>MailFeed Test Email</title>
            </head>
            <body style="font-family: -apple-system, BlinkMacSystemFont, sans-serif; line-height: 1.6; color: #333; padding: 20px;">
                <h1 style="color: #10b981;">✅ MailFeed Email Configuration Successful!</h1>
                <p>This is a test email from your MailFeed installation. Your SMTP settings are configured correctly.</p>
                <p>You can now receive RSS/Atom feed updates via email by selecting "Email Only" or "Both" as the delivery method for your subscriptions.</p>
                <hr style="margin: 30px 0; border: none; border-top: 1px solid #eee;">
                <p style="color: #666; font-size: 12px;">This email was sent from MailFeed - your self-hosted RSS to email service.</p>
            </body>
            </html>
        "#.to_string())
        .map_err(|e| format!("Failed to build email: {e}"))?;
    
    // Build SMTP transport
    let creds = Credentials::new(
        email_config.smtp_username.clone(),
        smtp_password,
    );
    
    let mailer = if email_config.smtp_use_tls {
        SmtpTransport::relay(&email_config.smtp_host)
            .map_err(|e| format!("Failed to create SMTP relay: {e}"))?
            .port(email_config.smtp_port as u16)
            .credentials(creds)
            .build()
    } else {
        SmtpTransport::builder_dangerous(&email_config.smtp_host)
            .port(email_config.smtp_port as u16)
            .credentials(creds)
            .build()
    };
    
    // Send email
    mailer.send(&email)
        .map_err(|e| format!("Failed to send email: {e}"))?;
    
    Ok(())
}