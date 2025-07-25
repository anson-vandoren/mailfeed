use super::{auth, config, feed_items, feeds, subscriptions, users};
use actix_web::{web, Scope};

pub fn routes() -> Scope {
    web::scope("/api")
        .service(subscriptions::routes())
        .service(config::routes())
        .service(feed_items::routes())
        .service(users::routes())
        .service(auth::routes())
        .service(feeds::routes())
}
