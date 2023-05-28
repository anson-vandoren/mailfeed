use super::handlers;
use actix_web::{web, Scope};

pub fn routes() -> Scope {
    web::scope("/feeds/{feed_id}/items")
        .route("", web::get().to(handlers::get_all_feed_items))
        .route("/{id}", web::get().to(handlers::get_feed_item))
}
