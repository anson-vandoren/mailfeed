use super::handlers;
use actix_web::{web, Scope};

pub fn routes() -> Scope {
    web::scope("/feeds")
        .service(handlers::get_all_feeds)
        .service(handlers::create_feed)
        .service(handlers::get_feed)
        .service(handlers::update_feed)
        .service(handlers::delete_feed)
}
