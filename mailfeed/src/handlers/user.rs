use crate::models::user::{NewUser, PartialUser, User, UserCreationError, UserQuery};
use crate::DbPool;
use actix_web::{web, HttpResponse, Responder};
use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct UserPath {
    id: String,
}

pub async fn get_all_users() -> impl Responder {
    HttpResponse::Ok().body("get_all_users")
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
        Err(UserCreationError::EmailExists) => HttpResponse::BadRequest().body("Email exists"),
        Err(UserCreationError::PasswordTooShort) => {
            HttpResponse::BadRequest().body("Password too short")
        }
        Err(_) => HttpResponse::InternalServerError().body("Error creating user"),
    }
}

pub async fn get_user(pool: web::Data<DbPool>, path: web::Path<UserPath>) -> impl Responder {
    let id = path.id.parse::<i32>();

    match id {
        Ok(id) => {
            let mut conn = pool.get().expect("couldn't get db connection from pool");
            let user = User::get(&mut conn, UserQuery::Id(id)).unwrap();
            let user_json = serde_json::to_string(&user).unwrap();
            HttpResponse::Ok().body(user_json)
        }
        Err(_) => HttpResponse::BadRequest().body("Invalid user ID"),
    }
}

pub async fn update_user(
    pool: web::Data<DbPool>,
    path: web::Path<UserPath>,
    updates: web::Json<PartialUser>,
) -> impl Responder {
    let id = path.id.parse::<i32>();

    match id {
        Ok(id) => {
            let mut conn = pool.get().expect("couldn't get db connection from pool");

            let updated_user = User::update(&mut conn, id, &updates).unwrap();
            let user_json = serde_json::to_string(&updated_user).unwrap();
            HttpResponse::Ok().body(user_json)
        }
        Err(_) => HttpResponse::BadRequest().body("Invalid user ID"),
    }
}

pub async fn delete_user() -> impl Responder {
    HttpResponse::Ok().body("delete_user")
}
