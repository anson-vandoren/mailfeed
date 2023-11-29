use super::types::{RqPartUser, RqUserId};
use crate::models::user::{NewUser, User, UserQuery, UserTableError};
use crate::RqDbPool;
use actix_web::{delete, get, patch, post, web, HttpResponse, Responder};

use crate::claims::Claims;

#[get("")]
pub async fn get_all_users(pool: RqDbPool, claims: Claims) -> impl Responder {
    let mut conn = match pool.get() {
        Ok(conn) => conn,
        Err(err) => {
            log::error!("Failed to get db connection from pool: {}", err);
            return HttpResponse::InternalServerError().body("Error connecting to database");
        }
    };

    if &claims.role != "admin" {
        log::warn!("Unauthorized attempt to get all users by {}", claims.sub);
        return HttpResponse::Forbidden().body("Forbidden");
    }

    let users_result = User::get_all(&mut conn);

    match users_result {
        Ok(users) => HttpResponse::Ok().json(users),
        Err(_) => HttpResponse::InternalServerError().body("Error getting users"),
    }
}

#[post("")]
pub async fn create_user(
    pool: RqDbPool,
    new_user: web::Json<NewUser>,
    claims: Claims,
) -> impl Responder {
    let mut conn = match pool.get() {
        Ok(conn) => conn,
        Err(err) => {
            log::error!("Failed to get db connection from pool: {}", err);
            return HttpResponse::InternalServerError().body("Error connecting to database");
        }
    };
    let db_result = User::create(&mut conn, &new_user, claims);

    match db_result {
        Ok(_) => {
            log::info!("created new user: {:?}", new_user.email);
            let user = User::get(&mut conn, UserQuery::Email(&new_user.email)).unwrap();
            HttpResponse::Ok().json(user)
        }
        Err(UserTableError::EmailExists) => HttpResponse::BadRequest().body("Email exists"),
        Err(UserTableError::PasswordTooShort) => {
            HttpResponse::BadRequest().body("Password too short")
        }
        Err(_) => HttpResponse::InternalServerError().body("Error creating user"),
    }
}

#[get("/{user_id}")]
pub async fn get_user(pool: RqDbPool, user_path: RqUserId, claims: Claims) -> impl Responder {
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
    claims: Claims,
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

#[delete("/{user_id}")]
pub async fn delete_user(pool: RqDbPool, user_path: RqUserId, claims: Claims) -> impl Responder {
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
