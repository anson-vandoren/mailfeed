use super::{auth, feed_items, feeds, subscriptions, users};
use actix_web::{web, Scope};

pub fn routes() -> Scope {
    web::scope("/api")
        .service(users::routes())
        .service(subscriptions::routes())
        .service(auth::routes())
        .service(feeds::routes())
        .service(feed_items::routes())
}
