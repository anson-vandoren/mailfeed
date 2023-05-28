use super::handlers;
use actix_web::{web, Scope};

pub fn routes() -> Scope {
    web::scope("/users")
        .service(handlers::get_all_users)
        .service(handlers::create_user)
        .service(handlers::get_user)
        .service(handlers::update_user)
        .service(handlers::delete_user)
}
