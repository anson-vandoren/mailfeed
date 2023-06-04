use super::jwt::{create_access_token, create_refresh_token, verify_and_extract_claims};
use super::types::{LoginRequest, RefreshRequest, TokenResponse};
use crate::claims::Claims;
use crate::models::user::{PartialUser, User, UserQuery};
use actix_web::{post, web, HttpResponse, Responder};

use crate::RqDbPool;

#[post("/login")]
pub async fn login(pool: RqDbPool, login_req: web::Json<LoginRequest>) -> impl Responder {
    let mut conn = match pool.get() {
        Ok(conn) => conn,
        Err(err) => {
            log::error!("Failed to get db connection from pool: {}", err);
            return HttpResponse::InternalServerError().body("Error connecting to database");
        }
    };

    let user = match User::get(&mut conn, UserQuery::Email(&login_req.email)) {
        Some(user) => user,
        None => return HttpResponse::BadRequest().body("Invalid email or password"),
    };

    if !user.is_active {
        return HttpResponse::BadRequest().body("Account is deactivated - contact admin");
    }

    let is_password_correct = match User::check_password(&user, &login_req.password) {
        Ok(is_correct) => is_correct,
        Err(_) => return HttpResponse::BadRequest().body("Invalid email or password"),
    };

    if !is_password_correct {
        return HttpResponse::BadRequest().body("Invalid email or password");
    }

    let refresh_token = match create_refresh_token(&user) {
        Ok(token) => token,
        Err(_) => return HttpResponse::InternalServerError().body("Error creating refresh token"),
    };

    let access_token = match create_access_token(&user) {
        Ok(token) => token,
        Err(_) => return HttpResponse::InternalServerError().body("Error creating access token"),
    };

    let updates = PartialUser {
        refresh_token: Some(refresh_token.clone()),
        ..Default::default()
    };
    // add refresh token to users table
    if let Err(e) = User::update(&mut conn, user.id, &updates) {
        log::error!("Error updating user: {:?}", e);
        return HttpResponse::InternalServerError().body("Error updating user");
    }

    let response = TokenResponse {
        access_token: &access_token,
        refresh_token: &refresh_token,
    };

    HttpResponse::Ok().json(response)
}

#[post("/logout")]
pub async fn logout(pool: RqDbPool, claims: Claims) -> impl Responder {
    log::info!("logout: {:?}", &claims.sub);
    let mut conn = match pool.get() {
        Ok(conn) => conn,
        Err(err) => {
            log::error!("Failed to get db connection from pool: {}", err);
            return HttpResponse::InternalServerError().body("Error connecting to database");
        }
    };

    if let Err(e) = User::clear_refresh_token(&mut conn, UserQuery::Id(claims.sub)) {
        log::error!("Error clearing refresh token: {:?}", e);
        return HttpResponse::InternalServerError().body("Error clearing refresh token");
    }

    HttpResponse::Ok().body("logout successful")
}

#[post("/refresh")]
pub async fn refresh(pool: RqDbPool, refresh_req: web::Json<RefreshRequest>) -> impl Responder {
    let mut conn = match pool.get() {
        Ok(conn) => conn,
        Err(err) => {
            log::error!("Failed to get db connection from pool: {}", err);
            return HttpResponse::InternalServerError().body("Error connecting to database");
        }
    };

    let claims = verify_and_extract_claims(&refresh_req.refresh_token);

    if claims.is_none() {
        return HttpResponse::Unauthorized().body("Invalid refresh token");
    }

    let claims = claims.unwrap();

    let user = match User::get(&mut conn, UserQuery::Id(claims.sub)) {
        Some(user) => user,
        None => return HttpResponse::Unauthorized().body("Invalid refresh token"),
    };

    if !user.is_active {
        if let Err(e) = User::clear_refresh_token(&mut conn, UserQuery::Id(user.id)) {
            log::error!("Error clearing refresh token: {:?}", e);
        }
        return HttpResponse::BadRequest().body("Account is deactivated - contact admin");
    }

    let new_access_token = match create_access_token(&user) {
        Ok(token) => token,
        Err(_) => return HttpResponse::InternalServerError().body("Error creating access token"),
    };

    let response = TokenResponse {
        access_token: &new_access_token,
        refresh_token: &refresh_req.refresh_token,
    };

    HttpResponse::Ok().json(response)
}

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
