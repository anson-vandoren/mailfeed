use super::handlers;
use actix_web::{web, Scope};

pub fn routes() -> Scope {
    web::scope("/feeds/{feed_id}/items")
        .service(handlers::get_items_for_feed)
        .service(handlers::get_feed_item)
}
