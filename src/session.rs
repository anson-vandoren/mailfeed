use std::future::{ready, Ready};

use crate::{models::session::Session, models::user::User, types::ErrorMessage};
use crate::RqDbPool;
use actix_web::{
    dev::Payload, error::ResponseError, http::StatusCode, web, FromRequest, HttpRequest,
    HttpResponse,
};
use derive_more::Display;
use serde::{Deserialize, Serialize};

#[derive(Debug, Display)]
enum SessionError {
    #[display(fmt = "no_session_cookie")]
    NoSessionCookie,
    #[display(fmt = "invalid_session")]
    InvalidSession,
    #[display(fmt = "session_expired")]
    SessionExpired,
    #[display(fmt = "database_error")]
    DatabaseError,
}

impl ResponseError for SessionError {
    fn error_response(&self) -> HttpResponse {
        match self {
            Self::NoSessionCookie | Self::InvalidSession | Self::SessionExpired => {
                HttpResponse::Unauthorized().json(ErrorMessage {
                    error: Some("unauthorized".to_string()),
                    error_description: Some("Valid session required".to_string()),
                    message: "Please log in".to_string(),
                })
            }
            Self::DatabaseError => HttpResponse::InternalServerError().json(ErrorMessage {
                error: Some("internal_error".to_string()),
                error_description: Some("Database error".to_string()),
                message: "Internal server error".to_string(),
            }),
        }
    }

    fn status_code(&self) -> StatusCode {
        match self {
            Self::NoSessionCookie | Self::InvalidSession | Self::SessionExpired => {
                StatusCode::UNAUTHORIZED
            }
            Self::DatabaseError => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }
}

/// Session claims - similar structure to JWT Claims but for session-based auth
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct SessionClaims {
    pub sub: i32,       // user_id
    pub role: String,   // user role
    pub email: String,  // user email
}

impl FromRequest for SessionClaims {
    type Error = actix_web::Error;
    type Future = Ready<Result<Self, Self::Error>>;

    fn from_request(req: &HttpRequest, _payload: &mut Payload) -> Self::Future {
        // Get the database pool from app data
        let pool = match req.app_data::<RqDbPool>() {
            Some(pool) => pool.get_ref().clone(),
            None => {
                log::error!("Failed to get database pool from app data");
                return ready(Err(SessionError::DatabaseError.into()));
            },
        };

        // Extract session cookie
        let session_id = match extract_session_cookie(req) {
            Some(id) => {
                log::debug!("Found session cookie: {}", id);
                id
            },
            None => {
                log::debug!("No session cookie found");
                return ready(Err(SessionError::NoSessionCookie.into()));
            },
        };

        // Get database connection
        let mut conn = match pool.get() {
            Ok(conn) => conn,
            Err(_) => return ready(Err(SessionError::DatabaseError.into())),
        };

        // Get valid session
        let session = match Session::get_valid(&mut conn, &session_id) {
            Some(session) => {
                log::debug!("Found valid session for user_id: {}", session.user_id);
                session
            },
            None => {
                log::debug!("Invalid or expired session: {}", session_id);
                return ready(Err(SessionError::InvalidSession.into()));
            },
        };

        // Get user details - we'll need this to create claims
        use crate::models::user::UserQuery;
        let user = match User::get(&mut conn, UserQuery::Id(session.user_id)) {
            Some(user) => user,
            None => return ready(Err(SessionError::InvalidSession.into())),
        };

        // Check if user is still active
        if !user.is_active {
            return ready(Err(SessionError::InvalidSession.into()));
        }

        // Update last_accessed timestamp
        if let Err(_) = session.touch(&mut conn) {
            log::warn!("Failed to update session last_accessed time");
        }

        // Create session claims
        let claims = SessionClaims {
            sub: user.id,
            role: user.role,
            email: user.login_email,
        };

        ready(Ok(claims))
    }
}

/// Extract session ID from cookies
fn extract_session_cookie(req: &HttpRequest) -> Option<String> {
    req.cookie("session_id")
        .map(|cookie| cookie.value().to_string())
}

/// Session management functions
pub mod session_manager {
    use super::*;
    use actix_web::{http::header, HttpResponse, ResponseError};
    use chrono::Utc;
    use diesel::SqliteConnection;

    /// Create a new session and set the session cookie
    pub fn create_session(
        conn: &mut SqliteConnection,
        user: &User,
    ) -> Result<HttpResponse, Box<dyn ResponseError>> {
        // Create new session
        let session = Session::create(conn, user.id)
            .map_err(|_| Box::new(SessionError::DatabaseError) as Box<dyn ResponseError>)?;

        // Build response with session cookie
        // Use secure cookies in production, but not in development (localhost)
        let is_production = !cfg!(debug_assertions);
        let response = HttpResponse::Ok()
            .cookie(
                actix_web::cookie::Cookie::build("session_id", &session.session_id)
                    .secure(is_production)
                    .http_only(true)
                    .same_site(actix_web::cookie::SameSite::Strict)
                    .expires(
                        actix_web::cookie::time::OffsetDateTime::from_unix_timestamp(
                            session.expires_at as i64,
                        )
                        .unwrap(),
                    )
                    .path("/")
                    .finish(),
            )
            .json(serde_json::json!({
                "message": "Login successful",
                "user_id": user.id
            }));

        Ok(response)
    }

    /// Clear session and remove session cookie
    pub fn clear_session(
        conn: &mut SqliteConnection,
        session_id: &str,
    ) -> Result<HttpResponse, Box<dyn ResponseError>> {
        // Delete session from database
        Session::delete(conn, session_id)
            .map_err(|_| Box::new(SessionError::DatabaseError) as Box<dyn ResponseError>)?;

        // Build response that clears the session cookie
        let is_production = !cfg!(debug_assertions);
        let response = HttpResponse::Ok()
            .cookie(
                actix_web::cookie::Cookie::build("session_id", "")
                    .secure(is_production)
                    .http_only(true)
                    .same_site(actix_web::cookie::SameSite::Strict)
                    .expires(actix_web::cookie::time::OffsetDateTime::UNIX_EPOCH)
                    .path("/")
                    .finish(),
            )
            .json(serde_json::json!({
                "message": "Logout successful"
            }));

        Ok(response)
    }

    /// Cleanup expired sessions - should be called periodically
    pub fn cleanup_expired_sessions(conn: &mut SqliteConnection) -> Result<usize, SessionError> {
        Session::cleanup_expired(conn).map_err(|_| SessionError::DatabaseError)
    }
}