use actix_web::{
    dev::{forward_ready, Service, ServiceRequest, ServiceResponse, Transform},
    Error,
};
use futures_util::future::LocalBoxFuture;
use std::{
    future::{ready, Ready},
    rc::Rc,
};

/// Security headers middleware
pub struct SecurityHeaders;

impl<S, B> Transform<S, ServiceRequest> for SecurityHeaders
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error> + 'static,
    S::Future: 'static,
    B: 'static,
{
    type Response = ServiceResponse<B>;
    type Error = Error;
    type InitError = ();
    type Transform = SecurityHeadersMiddleware<S>;
    type Future = Ready<Result<Self::Transform, Self::InitError>>;

    fn new_transform(&self, service: S) -> Self::Future {
        ready(Ok(SecurityHeadersMiddleware {
            service: Rc::new(service),
        }))
    }
}

pub struct SecurityHeadersMiddleware<S> {
    service: Rc<S>,
}

impl<S, B> Service<ServiceRequest> for SecurityHeadersMiddleware<S>
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error> + 'static,
    S::Future: 'static,
    B: 'static,
{
    type Response = ServiceResponse<B>;
    type Error = Error;
    type Future = LocalBoxFuture<'static, Result<Self::Response, Self::Error>>;

    forward_ready!(service);

    fn call(&self, req: ServiceRequest) -> Self::Future {
        let srv = self.service.clone();

        Box::pin(async move {
            let mut res = srv.call(req).await?;

            // Add security headers
            let headers = res.headers_mut();
            
            // Prevent clickjacking
            headers.insert(
                actix_web::http::header::HeaderName::from_static("x-frame-options"),
                actix_web::http::header::HeaderValue::from_static("DENY"),
            );
            
            // Prevent MIME type sniffing
            headers.insert(
                actix_web::http::header::HeaderName::from_static("x-content-type-options"),
                actix_web::http::header::HeaderValue::from_static("nosniff"),
            );
            
            // Enable XSS protection
            headers.insert(
                actix_web::http::header::HeaderName::from_static("x-xss-protection"),
                actix_web::http::header::HeaderValue::from_static("1; mode=block"),
            );
            
            // Referrer policy
            headers.insert(
                actix_web::http::header::HeaderName::from_static("referrer-policy"),
                actix_web::http::header::HeaderValue::from_static("strict-origin-when-cross-origin"),
            );
            
            // Content Security Policy - allows HTMX, Tailwind CDNs, and Google Fonts
            headers.insert(
                actix_web::http::header::HeaderName::from_static("content-security-policy"),
                actix_web::http::header::HeaderValue::from_static(
                    "default-src 'self'; script-src 'self' 'unsafe-inline' https://unpkg.com https://cdn.tailwindcss.com; style-src 'self' 'unsafe-inline' https://fonts.googleapis.com; img-src 'self' data:; connect-src 'self' https://fonts.gstatic.com; font-src 'self' https://fonts.gstatic.com"
                ),
            );

            // Only add HSTS in production (when using HTTPS)
            if cfg!(not(debug_assertions)) {
                headers.insert(
                    actix_web::http::header::HeaderName::from_static("strict-transport-security"),
                    actix_web::http::header::HeaderValue::from_static("max-age=31536000; includeSubDomains"),
                );
            }

            Ok(res)
        })
    }
}

/// Input validation utilities
pub mod validation {
    use regex::Regex;
    use std::sync::OnceLock;
    
    static URL_REGEX: OnceLock<Regex> = OnceLock::new();
    static EMAIL_REGEX: OnceLock<Regex> = OnceLock::new();
    
    /// Validate URL format and scheme
    pub fn validate_url(url: &str) -> Result<(), String> {
        if url.is_empty() {
            return Err("URL cannot be empty".to_string());
        }
        
        if url.len() > 2048 {
            return Err("URL too long (max 2048 characters)".to_string());
        }
        
        let url_regex = URL_REGEX.get_or_init(|| {
            Regex::new(r"^https?://[^\s/$.?#].[^\s]*$").unwrap()
        });
        
        if !url_regex.is_match(url) {
            return Err("Invalid URL format. Must be HTTP or HTTPS".to_string());
        }
        
        // Additional security: block local/private IPs in production
        if cfg!(not(debug_assertions)) && (url.contains("localhost") || 
               url.contains("127.0.0.1") || 
               url.contains("192.168.") || 
               url.contains("10.") ||
               url.contains("172.16.")) {
            return Err("Private/local URLs not allowed in production".to_string());
        }
        
        Ok(())
    }
    
    /// Validate email format
    pub fn validate_email(email: &str) -> Result<(), String> {
        if email.is_empty() {
            return Err("Email cannot be empty".to_string());
        }
        
        if email.len() > 254 {
            return Err("Email too long (max 254 characters)".to_string());
        }
        
        let email_regex = EMAIL_REGEX.get_or_init(|| {
            Regex::new(r"^[a-zA-Z0-9._%+-]+@[a-zA-Z0-9.-]+\.[a-zA-Z]{2,}$").unwrap()
        });
        
        if !email_regex.is_match(email) {
            return Err("Invalid email format".to_string());
        }
        
        Ok(())
    }
    
    /// Validate friendly name
    pub fn validate_friendly_name(name: &str) -> Result<(), String> {
        if name.len() > 100 {
            return Err("Friendly name too long (max 100 characters)".to_string());
        }
        
        // Allow most characters but prevent XSS
        if name.contains('<') || name.contains('>') || name.contains('"') || name.contains('\'') {
            return Err("Friendly name contains invalid characters".to_string());
        }
        
        Ok(())
    }
}

/// Password encryption module for securing SMTP credentials
pub mod encryption;

/// Rate limiting configuration for different endpoints
pub use actix_governor::{GovernorConfigBuilder, GovernorConfig};

pub fn create_rate_limiter() -> GovernorConfig<actix_governor::PeerIpKeyExtractor, actix_governor::governor::middleware::StateInformationMiddleware> {
    // General rate limiting for API endpoints
    GovernorConfigBuilder::default()
        .per_second(10) // Allow 10 requests per second
        .burst_size(20) // Allow bursts of 20 requests
        .use_headers() // Send rate limit info in headers
        .finish()
        .unwrap()
}

pub fn create_auth_rate_limiter() -> GovernorConfig<actix_governor::PeerIpKeyExtractor, actix_governor::governor::middleware::StateInformationMiddleware> {
    // Very restrictive for login attempts - prevent brute force
    GovernorConfigBuilder::default()
        .per_second(1) // Only 1 login attempt per second
        .burst_size(3) // Small burst allowance
        .use_headers()
        .finish()
        .unwrap()
}