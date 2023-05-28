use crate::models::user::{NewUser, PartialUser, User, UserQuery, UserTableError};
use crate::DbPool;
use actix_web::{web, HttpResponse, Responder};
use serde::Deserialize;

use crate::claims::Claims;

#[derive(Debug, Deserialize)]
pub struct UserPath {
    id: String,
}

pub async fn get_all_users(pool: web::Data<DbPool>, claims: Claims) -> impl Responder {
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
        Ok(users) => {
            let users_json = serde_json::to_string(&users).unwrap();
            HttpResponse::Ok().body(users_json)
        }
        Err(_) => HttpResponse::InternalServerError().body("Error getting users"),
    }
}

pub async fn create_user(
    pool: web::Data<DbPool>,
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
            let user_json = serde_json::to_string(&user).unwrap();
            HttpResponse::Ok().body(user_json)
        }
        Err(UserTableError::EmailExists) => HttpResponse::BadRequest().body("Email exists"),
        Err(UserTableError::PasswordTooShort) => {
            HttpResponse::BadRequest().body("Password too short")
        }
        Err(_) => HttpResponse::InternalServerError().body("Error creating user"),
    }
}

pub async fn get_user(
    pool: web::Data<DbPool>,
    path: web::Path<UserPath>,
    claims: Claims,
) -> impl Responder {
    let id = path.id.parse::<i32>();

    if let Err(_) = id {
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

    if &claims.role != "admin" && claims.sub != user.id.unwrap_or(-1) {
        log::warn!("Unauthorized attempt to get user by {}", claims.sub);
        return HttpResponse::Forbidden().body("Forbidden");
    }

    let user_json = serde_json::to_string(&user).unwrap();
    HttpResponse::Ok().body(user_json)
}

pub async fn update_user(
    pool: web::Data<DbPool>,
    path: web::Path<UserPath>,
    updates: web::Json<PartialUser>,
    claims: Claims,
) -> impl Responder {
    // if none of the fields are set, return a bad request
    if updates.is_empty() {
        return HttpResponse::BadRequest().body("No fields to update");
    }
    let id = match path.id.parse::<i32>() {
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
        Ok(_) => (),
        Err(UserTableError::EmailExists) => return HttpResponse::BadRequest().body("Email exists"),
        Err(_) => return HttpResponse::InternalServerError().body("Error updating user"),
    };

    let user_json = serde_json::to_string(&updated_user).unwrap();
    HttpResponse::Ok().body(user_json)
}

pub async fn delete_user(
    pool: web::Data<DbPool>,
    path: web::Path<UserPath>,
    claims: Claims,
) -> impl Responder {
    let id = match path.id.parse::<i32>() {
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
