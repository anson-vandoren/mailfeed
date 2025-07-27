use super::handlers;
use actix_web::{web, Scope};

pub fn routes() -> Scope {
    web::scope("/config")
        .service(handlers::get_user_config)
        .service(handlers::update_user_config)
        .service(handlers::bulk_update_user_config)
}