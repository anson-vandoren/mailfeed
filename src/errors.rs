use actix_web::{HttpResponse, ResponseError};
use diesel::{r2d2, SqliteConnection};
use serde_json::json;
use std::fmt;

/// Application-wide error types with user-friendly messages
#[derive(Debug)]
pub enum AppError {
    // Authentication & Authorization
    InvalidCredentials,
    AccountDeactivated,
    SessionExpired,
    Forbidden,
    
    // Validation Errors
    InvalidInput { field: String, message: String },
    DuplicateResource { resource: String },
    ResourceNotFound { resource: String },
    
    // Feed-related Errors  
    FeedAlreadySubscribed,
    FeedNotFound,
    FeedParseError,
    
    // Database Errors
    DatabaseError,
    ConnectionPoolError,
    
    // External Service Errors
    NetworkError,
    ServiceUnavailable,
    
    // System Errors
    InternalError,
    ConfigurationError,
}

impl fmt::Display for AppError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            // Authentication & Authorization
            AppError::InvalidCredentials => write!(f, "Invalid email or password"),
            AppError::AccountDeactivated => write!(f, "Account is deactivated - contact admin"),
            AppError::SessionExpired => write!(f, "Session expired - please log in again"),
            AppError::Forbidden => write!(f, "Access denied"),
            
            // Validation Errors
            AppError::InvalidInput { field, message } => write!(f, "Invalid {}: {}", field, message),
            AppError::DuplicateResource { resource } => write!(f, "{} already exists", resource),
            AppError::ResourceNotFound { resource } => write!(f, "{} not found", resource),
            
            // Feed-related Errors
            AppError::FeedAlreadySubscribed => write!(f, "Already subscribed to this feed"),
            AppError::FeedNotFound => write!(f, "Feed not found or inaccessible"),
            AppError::FeedParseError => write!(f, "Unable to parse feed - invalid format"),
            
            // Database Errors
            AppError::DatabaseError => write!(f, "A database error occurred - please try again"),
            AppError::ConnectionPoolError => write!(f, "Service temporarily unavailable - please try again"),
            
            // External Service Errors
            AppError::NetworkError => write!(f, "Network error - please check your connection"),
            AppError::ServiceUnavailable => write!(f, "Service temporarily unavailable - please try again later"),
            
            // System Errors
            AppError::InternalError => write!(f, "An unexpected error occurred - please try again"),
            AppError::ConfigurationError => write!(f, "System configuration error - contact support"),
        }
    }
}

impl ResponseError for AppError {
    fn error_response(&self) -> HttpResponse {
        let (status, error_code, message) = match self {
            // 400 Bad Request
            AppError::InvalidCredentials => (400, "INVALID_CREDENTIALS", self.to_string()),
            AppError::InvalidInput { .. } => (400, "INVALID_INPUT", self.to_string()),
            AppError::DuplicateResource { .. } => (400, "DUPLICATE_RESOURCE", self.to_string()),
            AppError::FeedAlreadySubscribed => (400, "FEED_ALREADY_SUBSCRIBED", self.to_string()),
            AppError::FeedParseError => (400, "FEED_PARSE_ERROR", self.to_string()),
            
            // 401 Unauthorized
            AppError::SessionExpired => (401, "SESSION_EXPIRED", self.to_string()),
            
            // 403 Forbidden
            AppError::Forbidden => (403, "FORBIDDEN", self.to_string()),
            AppError::AccountDeactivated => (403, "ACCOUNT_DEACTIVATED", self.to_string()),
            
            // 404 Not Found
            AppError::ResourceNotFound { .. } => (404, "RESOURCE_NOT_FOUND", self.to_string()),
            AppError::FeedNotFound => (404, "FEED_NOT_FOUND", self.to_string()),
            
            // 500 Internal Server Error
            AppError::DatabaseError => (500, "DATABASE_ERROR", self.to_string()),
            AppError::ConnectionPoolError => (500, "CONNECTION_POOL_ERROR", self.to_string()),
            AppError::InternalError => (500, "INTERNAL_ERROR", self.to_string()),
            AppError::ConfigurationError => (500, "CONFIGURATION_ERROR", self.to_string()),
            
            // 502 Bad Gateway
            AppError::NetworkError => (502, "NETWORK_ERROR", self.to_string()),
            
            // 503 Service Unavailable
            AppError::ServiceUnavailable => (503, "SERVICE_UNAVAILABLE", self.to_string()),
        };

        // Log detailed error information for debugging
        match self {
            AppError::DatabaseError | AppError::ConnectionPoolError | AppError::InternalError => {
                log::error!("Server error: {:?}", self);
            }
            AppError::InvalidCredentials => {
                log::warn!("Authentication failed: {:?}", self);
            }
            _ => {
                log::info!("Client error: {:?}", self);
            }
        }

        HttpResponse::build(actix_web::http::StatusCode::from_u16(status).unwrap())
            .json(json!({
                "error": {
                    "code": error_code,
                    "message": message
                }
            }))
    }
}

/// Result type alias for application errors
pub type AppResult<T> = Result<T, AppError>;

/// Helper functions for common error conversions
impl AppError {
    pub fn invalid_input(field: &str, message: &str) -> Self {
        AppError::InvalidInput {
            field: field.to_string(),
            message: message.to_string(),
        }
    }
    
    pub fn duplicate_resource(resource: &str) -> Self {
        AppError::DuplicateResource {
            resource: resource.to_string(),
        }
    }
    
    pub fn resource_not_found(resource: &str) -> Self {
        AppError::ResourceNotFound {
            resource: resource.to_string(),
        }
    }
}

/// Convert database connection pool errors
impl From<r2d2::Error> for AppError {
    fn from(err: r2d2::Error) -> Self {
        log::error!("Database connection pool error: {}", err);
        AppError::ConnectionPoolError
    }
}

/// Convert diesel database errors
impl From<diesel::result::Error> for AppError {
    fn from(err: diesel::result::Error) -> Self {
        use diesel::result::Error as DieselError;
        
        match err {
            DieselError::NotFound => AppError::ResourceNotFound {
                resource: "Record".to_string()
            },
            _ => {
                log::error!("Database error: {}", err);
                AppError::DatabaseError
            }
        }
    }
}

/// Convert network/reqwest errors
impl From<reqwest::Error> for AppError {
    fn from(err: reqwest::Error) -> Self {
        log::error!("Network error: {}", err);
        if err.is_timeout() || err.is_connect() {
            AppError::NetworkError
        } else {
            AppError::ServiceUnavailable
        }
    }
}

/// Convert feed parsing errors
impl From<feed_rs::parser::ParseFeedError> for AppError {
    fn from(err: feed_rs::parser::ParseFeedError) -> Self {
        log::warn!("Feed parse error: {}", err);
        AppError::FeedParseError
    }
}

/// Convert user table errors
impl From<crate::models::user::UserTableError> for AppError {
    fn from(err: crate::models::user::UserTableError) -> Self {
        use crate::models::user::UserTableError;
        
        match err {
            UserTableError::EmailExists => AppError::duplicate_resource("User with this email"),
            UserTableError::PasswordTooShort => AppError::invalid_input("password", "Password is too short"),
            UserTableError::UserNotFound => AppError::resource_not_found("User"),
            UserTableError::PasswordHashError => AppError::InternalError,
            UserTableError::DatabaseError => AppError::DatabaseError,
            UserTableError::Unauthorized => AppError::Forbidden,
        }
    }
}