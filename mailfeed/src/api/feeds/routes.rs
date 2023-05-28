use super::handlers;
use actix_web::{web, Scope};

pub fn routes() -> Scope {
    web::scope("/feeds")
        .route("", web::get().to(handlers::get_all_feeds))
        .route("", web::post().to(handlers::create_feed))
        .route("/{id}", web::get().to(handlers::get_feed))
        .route("/{id}", web::put().to(handlers::update_feed))
        .route("/{id}", web::delete().to(handlers::delete_feed))
}
