use super::handlers;
use actix_web::{web, Scope};

pub fn routes() -> Scope {
    web::scope("/subscriptions")
        .service(handlers::get_all_subscriptions)
        .service(handlers::create_subscription)
        .service(handlers::get_subscription)
        .service(handlers::update_subscription)
        .service(handlers::delete_subscription)
}
