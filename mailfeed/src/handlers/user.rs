use crate::models::user::{NewUser, PartialUser, User, UserQuery, UserTableError};
use crate::DbPool;
use actix_web::{web, HttpResponse, Responder};
use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct UserPath {
    id: String,
}

pub async fn get_all_users(pool: web::Data<DbPool>) -> impl Responder {
    let mut conn = pool.get().expect("couldn't get db connection from pool");
    let users_result = User::get_all(&mut conn);

    match users_result {
        Ok(users) => {
            let users_json = serde_json::to_string(&users).unwrap();
            HttpResponse::Ok().body(users_json)
        }
        Err(_) => HttpResponse::InternalServerError().body("Error getting users"),
    }
}

pub async fn create_user(pool: web::Data<DbPool>, new_user: web::Json<NewUser>) -> impl Responder {
    let mut conn = pool.get().expect("couldn't get db connection from pool");
    let db_result = User::create(&mut conn, &new_user);

    match db_result {
        Ok(_) => {
            log::info!("created new user: {:?}", new_user.email);
            let user = User::get(&mut conn, UserQuery::Email(new_user.email.clone())).unwrap();
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

pub async fn get_user(pool: web::Data<DbPool>, path: web::Path<UserPath>) -> impl Responder {
    let id = path.id.parse::<i32>();

    if let Err(_) = id {
        return HttpResponse::BadRequest().body("Invalid user ID");
    }
    let id = id.unwrap();

    let mut conn = pool.get().expect("couldn't get db connection from pool");
    let user = User::get(&mut conn, UserQuery::Id(id));

    match user {
        Some(user) => {
            let user_json = serde_json::to_string(&user).unwrap();
            HttpResponse::Ok().body(user_json)
        }
        None => {
            log::warn!("Requested non-existent user with ID {}", id);
            HttpResponse::NotFound().body("User not found")
        }
    }
}

pub async fn update_user(
    pool: web::Data<DbPool>,
    path: web::Path<UserPath>,
    updates: web::Json<PartialUser>,
) -> impl Responder {
    // if none of the fields are set, return a bad request
    if updates.is_empty() {
        return HttpResponse::BadRequest().body("No fields to update");
    }
    let id = path.id.parse::<i32>();

    match id {
        Ok(id) => {
            let mut conn = pool.get().expect("couldn't get db connection from pool");

            let updated_user = match User::update(&mut conn, id, &updates) {
                Ok(_) => (),
                Err(UserTableError::EmailExists) => {
                    return HttpResponse::BadRequest().body("Email exists")
                }
                Err(UserTableError::PasswordTooShort) => {
                    return HttpResponse::BadRequest().body("Password too short")
                }
                Err(_) => return HttpResponse::InternalServerError().body("Error updating user"),
            };

            let user_json = serde_json::to_string(&updated_user).unwrap();
            HttpResponse::Ok().body(user_json)
        }
        Err(_) => HttpResponse::BadRequest().body("Invalid user ID"),
    }
}

pub async fn delete_user(pool: web::Data<DbPool>, path: web::Path<UserPath>) -> impl Responder {
    let id = path.id.parse::<i32>();

    match id {
        Ok(id) => {
            let mut conn = pool.get().expect("couldn't get db connection from pool");

            let delete_result = User::delete(&mut conn, id);

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
        Err(_) => HttpResponse::BadRequest().body("Invalid user ID"),
    }
}
