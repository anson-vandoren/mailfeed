use actix_web::{dev::ServiceRequest, HttpMessage};
use diesel::prelude::*;
use std::time::SystemTime;
use tracing::{info, warn, error};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};
use uuid::Uuid;

/// Initialize structured logging and tracing
pub fn init_logging() {
    let log_level = std::env::var("LOG_LEVEL").unwrap_or_else(|_| "info".to_string());
    let log_format = std::env::var("LOG_FORMAT").unwrap_or_else(|_| "pretty".to_string());
    
    let env_filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new(&log_level));

    if log_format == "json" {
        // JSON structured logging for production
        tracing_subscriber::registry()
            .with(env_filter)
            .with(
                tracing_subscriber::fmt::layer()
                    .json()
                    .with_target(true)
                    .with_thread_ids(true)
                    .with_current_span(false)
                    .with_span_list(false)
            )
            .init();
    } else {
        // Pretty logging for development
        tracing_subscriber::registry()
            .with(env_filter)
            .with(
                tracing_subscriber::fmt::layer()
                    .pretty()
                    .with_target(true)
                    .with_thread_ids(false)
            )
            .init();
    }
    
    info!(
        service = "mailfeed",
        version = env!("CARGO_PKG_VERSION"),
        log_level = %log_level,
        log_format = %log_format,
        "Logging initialized"
    );
}

/// Request tracing middleware
pub struct RequestTracing;

impl RequestTracing {
    pub fn generate_request_id() -> String {
        Uuid::new_v4().to_string()
    }
    
    pub fn extract_request_id(req: &ServiceRequest) -> Option<String> {
        req.headers()
            .get("x-request-id")
            .and_then(|h| h.to_str().ok())
            .map(|s| s.to_string())
            .or_else(|| {
                req.extensions()
                    .get::<String>()
                    .cloned()
            })
    }
}

/// Metrics collection
pub struct Metrics {
    pub start_time: SystemTime,
}

impl Metrics {
    pub fn new() -> Self {
        Self {
            start_time: SystemTime::now(),
        }
    }
    
    pub fn uptime_seconds(&self) -> u64 {
        self.start_time
            .elapsed()
            .map(|d| d.as_secs())
            .unwrap_or(0)
    }
    
    pub fn log_request_metrics(
        &self,
        method: &str,
        path: &str,
        status: u16,
        duration_ms: u64,
        request_id: Option<&str>,
    ) {
        info!(
            request_id = request_id,
            method = method,
            path = path,
            status = status,
            duration_ms = duration_ms,
            "Request completed"
        );
    }
    
    pub fn log_database_metrics(&self, operation: &str, duration_ms: u64, success: bool) {
        if success {
            info!(
                operation = operation,
                duration_ms = duration_ms,
                "Database operation completed"
            );
        } else {
            warn!(
                operation = operation,
                duration_ms = duration_ms,
                "Database operation failed"
            );
        }
    }
    
    pub fn log_feed_processing(&self, feed_url: &str, items_processed: usize, duration_ms: u64) {
        info!(
            feed_url = feed_url,
            items_processed = items_processed,
            duration_ms = duration_ms,
            "Feed processing completed"
        );
    }
    
    pub fn log_email_metrics(&self, recipient_count: usize, success_count: usize, duration_ms: u64) {
        info!(
            recipient_count = recipient_count,
            success_count = success_count,
            failure_count = recipient_count - success_count,
            duration_ms = duration_ms,
            "Email batch completed"
        );
    }
    
    pub fn log_security_event(&self, event_type: &str, user_id: Option<i32>, details: serde_json::Value) {
        warn!(
            event_type = event_type,
            user_id = user_id,
            details = %details,
            "Security event detected"
        );
    }
    
    pub fn log_error(&self, error: &str, context: serde_json::Value, request_id: Option<&str>) {
        error!(
            request_id = request_id,
            error = error,
            context = %context,
            "Application error occurred"
        );
    }
}

impl Default for Metrics {
    fn default() -> Self {
        Self::new()
    }
}

/// Application health status
#[derive(Clone, Debug, serde::Serialize)]
pub struct HealthStatus {
    pub status: String,
    pub version: String,
    pub uptime_seconds: u64,
    pub checks: HealthChecks,
}

#[derive(Clone, Debug, serde::Serialize)]
pub struct HealthChecks {
    pub database: String,
    pub storage: String,
}

impl HealthStatus {
    pub async fn check(metrics: &Metrics, pool: &crate::RqDbPool) -> Self {
        let database_status = check_database_health(pool).await;
        let storage_status = check_storage_health();
        
        let overall_status = if database_status == "healthy" && storage_status == "healthy" {
            "healthy"
        } else {
            "unhealthy"
        };
        
        Self {
            status: overall_status.to_string(),
            version: env!("CARGO_PKG_VERSION").to_string(),
            uptime_seconds: metrics.uptime_seconds(),
            checks: HealthChecks {
                database: database_status,
                storage: storage_status,
            },
        }
    }
}

/// Check database connectivity
async fn check_database_health(pool: &crate::RqDbPool) -> String {
    match pool.get() {
        Ok(mut conn) => {
            match diesel::sql_query("SELECT 1").execute(&mut conn) {
                Ok(_) => "healthy".to_string(),
                Err(_) => "unhealthy".to_string(),
            }
        }
        Err(_) => "unhealthy".to_string(),
    }
}

/// Check storage/filesystem health
fn check_storage_health() -> String {
    // Check if we can write to a temp file
    match std::fs::write("/tmp/mailfeed_health_check", "test") {
        Ok(_) => {
            let _ = std::fs::remove_file("/tmp/mailfeed_health_check");
            "healthy".to_string()
        }
        Err(_) => "unhealthy".to_string(),
    }
}

/// Structured logging macros for consistent log format
#[macro_export]
macro_rules! log_user_action {
    ($user_id:expr, $action:expr, $details:expr) => {
        tracing::info!(
            user_id = $user_id,
            action = $action,
            details = %serde_json::json!($details),
            "User action performed"
        );
    };
}

#[macro_export]
macro_rules! log_security_event {
    ($event_type:expr, $user_id:expr, $details:expr) => {
        tracing::warn!(
            event_type = $event_type,
            user_id = $user_id,
            details = %serde_json::json!($details),
            "Security event detected"
        );
    };
}

#[macro_export]
macro_rules! log_error_with_context {
    ($error:expr, $context:expr) => {
        tracing::error!(
            error = %$error,
            context = %serde_json::json!($context),
            "Application error occurred"
        );
    };
}

/// Environment configuration
pub struct ObservabilityConfig {
    pub log_level: String,
    pub log_format: String,
    pub enable_request_tracing: bool,
    pub health_check_interval: u64,
}

impl ObservabilityConfig {
    pub fn from_env() -> Self {
        Self {
            log_level: std::env::var("LOG_LEVEL").unwrap_or_else(|_| "info".to_string()),
            log_format: std::env::var("LOG_FORMAT").unwrap_or_else(|_| "pretty".to_string()),
            enable_request_tracing: std::env::var("ENABLE_REQUEST_TRACING")
                .unwrap_or_else(|_| "true".to_string()) == "true",
            health_check_interval: std::env::var("HEALTH_CHECK_INTERVAL")
                .unwrap_or_else(|_| "30".to_string())
                .parse()
                .unwrap_or(30),
        }
    }
}