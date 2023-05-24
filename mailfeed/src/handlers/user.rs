use crate::models::user::{NewUser, User, UserQuery};
use crate::DbPool;
use actix_web::{web, HttpResponse, Responder};

pub async fn get_all_users() -> impl Responder {
    HttpResponse::Ok().body("get_all_users")
}

pub async fn create_user(pool: web::Data<DbPool>, new_user: web::Json<NewUser>) -> impl Responder {
    let mut conn = pool.get().expect("couldn't get db connection from pool");
    let db_result = User::create(&mut conn, &new_user);

    match db_result {
        Ok(_) => {
            let user = User::get(&mut conn, UserQuery::Email(new_user.email.clone())).unwrap();
            let user_json = serde_json::to_string(&user).unwrap();
            HttpResponse::Ok().body(user_json)
        }
        Err(_) => HttpResponse::BadRequest().body("Failed to create user"),
    }
}

pub async fn get_user() -> impl Responder {
    HttpResponse::Ok().body("get_user")
}

pub async fn update_user() -> impl Responder {
    HttpResponse::Ok().body("update_user")
}

pub async fn delete_user() -> impl Responder {
    HttpResponse::Ok().body("delete_user")
}
