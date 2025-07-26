use super::{auth, config, feed_items, feeds, subscriptions, users};
use actix_web::{web, Scope};

pub fn routes() -> Scope {
    web::scope("/api")
        .service(config::routes())
        .service(feed_items::routes())
        .service(users::routes())
        .service(subscriptions::routes()) // subscriptions as base route with user_id path param
        .service(auth::routes())
        .service(feeds::routes())
}
