use super::types::LoginRequest;
use crate::errors::{AppError, AppResult};
use crate::models::user::{User, UserQuery};
use crate::security::validation;
use crate::session::session_manager;
use actix_web::{post, web, HttpRequest, HttpResponse, Responder};

use crate::RqDbPool;

#[post("/login")]
pub async fn login(pool: RqDbPool, login_req: web::Json<LoginRequest>) -> AppResult<HttpResponse> {
    // Input validation
    if let Err(e) = validation::validate_email(&login_req.email) {
        tracing::warn!(
            email = %login_req.email,
            error = %e,
            "Login attempt with invalid email format"
        );
        return Err(AppError::InvalidCredentials);
    }
    
    if login_req.password.is_empty() || login_req.password.len() > 128 {
        tracing::warn!(
            email = %login_req.email,
            password_length = login_req.password.len(),
            "Login attempt with invalid password length"
        );
        return Err(AppError::InvalidCredentials);
    }

    let mut conn = pool.get().map_err(|_| AppError::ConnectionPoolError)?;

    let user = match User::get(&mut conn, UserQuery::Email(&login_req.email)) {
        Some(user) => user,
        None => {
            tracing::warn!(
                email = %login_req.email,
                "Login attempt for non-existent user"
            );
            return Err(AppError::InvalidCredentials);
        }
    };

    if !user.is_active {
        return Err(AppError::AccountDeactivated);
    }

    let is_password_correct = match User::check_password(&user, &login_req.password) {
        Ok(is_correct) => is_correct,
        Err(_) => return Err(AppError::InvalidCredentials),
    };

    if !is_password_correct {
        return Err(AppError::InvalidCredentials);
    }

    // Create session and return response with session cookie
    match session_manager::create_session(&mut conn, &user) {
        Ok(response) => {
            tracing::info!(
                user_id = user.id,
                email = %user.login_email,
                role = %user.role,
                "User login successful"
            );
            Ok(response)
        },
        Err(_) => Err(AppError::InternalError),
    }
}

#[post("/logout")]
pub async fn logout(pool: RqDbPool, req: HttpRequest) -> AppResult<HttpResponse> {
    let mut conn = pool.get().map_err(|_| AppError::ConnectionPoolError)?;

    // Extract session ID from cookie
    let session_id = match req.cookie("session_id") {
        Some(cookie) => cookie.value().to_string(),
        None => {
            tracing::warn!("Logout called without session cookie");
            return Err(AppError::SessionExpired);
        }
    };

    // Clear session and return response with cleared cookie
    match session_manager::clear_session(&mut conn, &session_id) {
        Ok(response) => Ok(response),
        Err(_) => Err(AppError::InternalError),
    }
}

// Refresh endpoint removed - sessions handle authentication automatically

#[post("/password_reset")]
pub async fn password_reset() -> impl Responder {
    HttpResponse::Ok().body("password_reset")
}

#[post("/password_reset/{token}")]
pub async fn password_reset_confirm() -> impl Responder {
    HttpResponse::Ok().body("password_reset_confirm")
}

#[post("/change_password")]
pub async fn change_password() -> impl Responder {
    HttpResponse::Ok().body("change_password")
}
