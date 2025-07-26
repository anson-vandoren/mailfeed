use super::handlers;
use actix_web::{web, Scope};

pub fn routes() -> Scope {
    web::scope("/auth")
        .service(handlers::login)
        .service(handlers::logout)
        .service(handlers::password_reset)
        .service(handlers::password_reset_confirm)
        .service(handlers::change_password)
}
