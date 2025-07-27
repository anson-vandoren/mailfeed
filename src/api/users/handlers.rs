use super::types::{RqPartUser, RqUserId};
use crate::errors::{AppError, AppResult};
use crate::models::user::{NewUser, User, UserQuery, UserTableError};
use crate::RqDbPool;
use actix_web::{delete, get, patch, post, web, HttpResponse, Responder};

use crate::session::SessionClaims;

#[get("")]
pub async fn get_all_users(pool: RqDbPool, claims: SessionClaims) -> AppResult<HttpResponse> {
    let mut conn = pool.get().map_err(|_| AppError::ConnectionPoolError)?;

    if &claims.role != "admin" {
        log::warn!("Unauthorized attempt to get all users by {}", claims.sub);
        return Err(AppError::Forbidden);
    }

    let users = User::get_all(&mut conn)?;
    Ok(HttpResponse::Ok().json(users))
}

#[post("")]
pub async fn create_user(
    pool: RqDbPool,
    new_user: web::Json<NewUser>,
    claims: SessionClaims,
) -> AppResult<HttpResponse> {
    let mut conn = pool.get().map_err(|_| AppError::ConnectionPoolError)?;
    let db_result = User::create(&mut conn, &new_user, claims);

    match db_result {
        Ok(_) => {
            log::info!("created new user: {:?}", new_user.email);
            let user = User::get(&mut conn, UserQuery::Email(&new_user.email))
                .ok_or(AppError::InternalError)?;
            Ok(HttpResponse::Ok().json(user))
        }
        Err(UserTableError::EmailExists) => Err(AppError::duplicate_resource("User with this email")),
        Err(UserTableError::PasswordTooShort) => {
            Err(AppError::invalid_input("password", "Password is too short"))
        }
        Err(_) => Err(AppError::InternalError),
    }
}

#[get("/{user_id}")]
pub async fn get_user(pool: RqDbPool, user_path: RqUserId, claims: SessionClaims) -> impl Responder {
    let id = user_path.user_id.parse::<i32>();

    if id.is_err() {
        return HttpResponse::BadRequest().body("Invalid user ID");
    }
    let id = id.unwrap();

    let mut conn = match pool.get() {
        Ok(conn) => conn,
        Err(err) => {
            log::error!("Failed to get db connection from pool: {}", err);
            return HttpResponse::InternalServerError().body("Error connecting to database");
        }
    };
    let user = match User::get(&mut conn, UserQuery::Id(id)) {
        Some(user) => user,
        None => return HttpResponse::InternalServerError().body("Error getting user"),
    };

    if &claims.role != "admin" && claims.sub != user.id {
        log::warn!("Unauthorized attempt to get user by {}", claims.sub);
        return HttpResponse::Forbidden().body("Forbidden");
    }

    HttpResponse::Ok().json(user)
}

#[patch("/{user_id}")]
pub async fn update_user(
    pool: RqDbPool,
    path: RqUserId,
    updates: RqPartUser,
    claims: SessionClaims,
) -> impl Responder {
    // if none of the fields are set, return a bad request
    if updates.is_empty() {
        return HttpResponse::BadRequest().body("No fields to update");
    }
    let id = match path.user_id.parse::<i32>() {
        Ok(id) => id,
        Err(_) => return HttpResponse::BadRequest().body("Invalid user ID"),
    };

    if id != claims.sub && &claims.role != "admin" {
        log::warn!("Unauthorized attempt to update user by {}", claims.sub);
        return HttpResponse::Forbidden().body("Forbidden");
    }

    // if role is being changed, it should only be changed by an admin
    if updates.role.is_some() && &claims.role != "admin" {
        log::warn!("Unauthorized attempt to change role by {}", claims.sub);
        return HttpResponse::Forbidden().body("Forbidden");
    }
    if updates.is_active.is_some() && &claims.role != "admin" {
        log::warn!("Unauthorized attempt to change is_active by {}", claims.sub);
        return HttpResponse::Forbidden().body("Forbidden");
    }

    let mut conn = match pool.get() {
        Ok(conn) => conn,
        Err(err) => {
            log::error!("Failed to get db connection from pool: {}", err);
            return HttpResponse::InternalServerError().body("Error connecting to database");
        }
    };

    let updated_user = match User::update(&mut conn, id, &updates) {
        Ok(user) => user,
        Err(UserTableError::EmailExists) => return HttpResponse::BadRequest().body("Email exists"),
        Err(_) => return HttpResponse::InternalServerError().body("Error updating user"),
    };

    HttpResponse::Ok().json(updated_user)
}

#[get("/{user_id}/test-telegram")]
pub async fn test_telegram(pool: RqDbPool, user_path: RqUserId, claims: SessionClaims) -> impl Responder {
    let id = match user_path.user_id.parse::<i32>() {
        Ok(id) => id,
        Err(_) => return HttpResponse::BadRequest().body("Invalid user ID"),
    };

    if id != claims.sub && &claims.role != "admin" {
        return HttpResponse::Forbidden().body("Forbidden");
    }

    let mut conn = match pool.get() {
        Ok(conn) => conn,
        Err(err) => {
            log::error!("Failed to get db connection from pool: {}", err);
            return HttpResponse::InternalServerError().body("Error connecting to database");
        }
    };

    let user = match crate::models::user::User::get(&mut conn, crate::models::user::UserQuery::Id(id)) {
        Some(user) => user,
        None => return HttpResponse::NotFound().body("User not found"),
    };

    if let Some(chat_id) = &user.telegram_chat_id {
        // Try to send a test message
        match crate::telegram::client::TelegramClient::new(&mut conn) {
            Ok(client) => {
                let test_message = format!(
                    "<b>🧪 Mailfeed Test Message</b>\n\nHello! This is a test message from your Mailfeed bot.\n\n📊 <b>Your Settings:</b>\n• Chat ID: <code>{}</code>\n• Username: {}\n\nIf you received this, your Telegram integration is working! 🎉",
                    chat_id,
                    user.telegram_username.as_deref().unwrap_or("Not set")
                );
                
                match client.send_html_message(chat_id, &test_message).await {
                    Ok(_) => {
                        log::info!("Test message sent successfully to chat_id: {}", chat_id);
                        HttpResponse::Ok().json(serde_json::json!({
                            "success": true,
                            "message": "Test message sent successfully!",
                            "chat_id": chat_id
                        }))
                    }
                    Err(e) => {
                        log::error!("Failed to send test message: {:?}", e);
                        HttpResponse::Ok().json(serde_json::json!({
                            "success": false,
                            "error": format!("Failed to send message: {}", e),
                            "chat_id": chat_id
                        }))
                    }
                }
            }
            Err(e) => {
                log::error!("Failed to create Telegram client: {:?}", e);
                HttpResponse::InternalServerError().json(serde_json::json!({
                    "success": false,
                    "error": format!("Failed to create Telegram client: {}", e)
                }))
            }
        }
    } else {
        HttpResponse::BadRequest().json(serde_json::json!({
            "success": false,
            "error": "No Telegram chat ID configured"
        }))
    }
}

#[delete("/{user_id}")]
pub async fn delete_user(pool: RqDbPool, user_path: RqUserId, claims: SessionClaims) -> impl Responder {
    let id = match user_path.user_id.parse::<i32>() {
        Ok(id) => id,
        Err(_) => return HttpResponse::BadRequest().body("Invalid user ID"),
    };

    let mut conn = match pool.get() {
        Ok(conn) => conn,
        Err(err) => {
            log::error!("Failed to get db connection from pool: {}", err);
            return HttpResponse::InternalServerError().body("Error connecting to database");
        }
    };

    let delete_result = User::delete(&mut conn, id, claims);

    match delete_result {
        Ok(_) => {
            log::info!("Deleted user with ID {}", id);
            HttpResponse::Ok().body("User deleted")
        }
        Err(err) => {
            log::error!("Error deleting user: {:?}", err);
            if let UserTableError::UserNotFound = err {
                return HttpResponse::NotFound().body("User not found");
            }
            HttpResponse::InternalServerError().body("Error deleting user")
        }
    }
}
