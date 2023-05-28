use super::handlers;
use actix_web::{web, Scope};

pub fn routes() -> Scope {
    web::scope("/users/{user_id}/subscriptions")
        .route("", web::get().to(handlers::get_all_subscriptions))
        .route("", web::post().to(handlers::create_subscription))
        .route("/{id}", web::get().to(handlers::get_subscription))
        .route("/{id}", web::put().to(handlers::update_subscription))
        .route("/{id}", web::delete().to(handlers::delete_subscription))
}
