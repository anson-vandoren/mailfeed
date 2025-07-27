use super::handlers;
use actix_web::{web, Scope};

pub fn routes() -> Scope {
    web::scope("")
        .service(handlers::create_or_update_email_config)
        .service(handlers::update_email_config)
        .service(handlers::delete_email_config)
        .service(handlers::send_test_email)
}