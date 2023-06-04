use crate::{
    claims::Claims,
    models::{feed::Feed, subscription::Subscription},
    RqDbPool,
};

use super::types::RqFeedId;
use actix_web::{delete, get, patch, post, HttpResponse, Responder};

#[get("")]
pub async fn get_all_feeds() -> impl Responder {
    HttpResponse::Ok().body("get_all_feeds")
}

#[post("")]
pub async fn create_feed() -> impl Responder {
    HttpResponse::Ok().body("create_feed")
}

#[get("/{feed_id}")]
pub async fn get_feed(pool: RqDbPool, feed_path: RqFeedId, claims: Claims) -> impl Responder {
    // parse feed_id from feed_path or else return 400
    let feed_id = feed_path.feed_id.parse::<i32>();
    if feed_id.is_err() {
        return HttpResponse::BadRequest().body("Invalid feed_id");
    }
    let feed_id = feed_id.unwrap();

    let mut conn = match pool.get() {
        Ok(conn) => conn,
        Err(err) => {
            log::error!("Failed to get db connection from pool: {}", err);
            return HttpResponse::InternalServerError().body("Error connecting to database");
        }
    };

    let user_id = claims.sub;

    let subscription = Subscription::get_for_user_and_feed(&mut conn, user_id, feed_id);

    if subscription.is_err() {
        return HttpResponse::InternalServerError().body("Error getting feed");
    }

    let subscription = subscription.unwrap();

    if subscription.is_none() {
        return HttpResponse::NotFound().body("Feed not found");
    }

    let subscription = subscription.unwrap();

    let feed = Feed::get_by_id(&mut conn, subscription.feed_id);

    if feed.is_none() {
        return HttpResponse::NotFound().body("Feed not found");
    }

    let feed = feed.unwrap();

    HttpResponse::Ok().json(feed)
}

#[patch("/{feed_id}")]
pub async fn update_feed() -> impl Responder {
    HttpResponse::Ok().body("update_feed")
}

#[delete("/{feed_id}")]
pub async fn delete_feed() -> impl Responder {
    HttpResponse::Ok().body("delete_feed")
}
