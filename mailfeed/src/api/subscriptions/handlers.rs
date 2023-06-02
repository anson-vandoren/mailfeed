use actix_web::{delete, get, patch, post, web, HttpResponse, Responder};

use super::types::{RqSubId, SubscriptionCreate, SubscriptionResponse};
use crate::{
    api::users::RqUserId,
    claims::Claims,
    models::{
        feed::{Feed, NewFeed},
        subscription::{NewSubscription, Subscription},
    },
    RqDbPool,
};

#[get("")]
pub async fn get_all_subscriptions(
    pool: RqDbPool,
    path: RqUserId,
    claims: Claims,
) -> impl Responder {
    let user_id = match path.user_id.parse::<i32>() {
        Ok(id) => id,
        Err(_) => return HttpResponse::BadRequest().body("Invalid user ID"),
    };

    if claims.sub != user_id {
        return HttpResponse::Forbidden().body("Forbidden");
    }

    let mut conn = match pool.get() {
        Ok(conn) => conn,
        Err(err) => {
            log::error!("Failed to get db connection from pool: {}", err);
            return HttpResponse::InternalServerError().body("Error connecting to database");
        }
    };

    let subscriptions = match Subscription::get_all_for_user(&mut conn, user_id) {
        Ok(subscriptions) => subscriptions,
        Err(_) => return HttpResponse::InternalServerError().body("Error getting subscriptions"),
    };

    let subscriptions_json = serde_json::to_string(&subscriptions).unwrap();
    HttpResponse::Ok().body(subscriptions_json)
}

#[post("")]
pub async fn create_subscription(
    pool: RqDbPool,
    path: RqUserId,
    sub_req: web::Json<SubscriptionCreate>,
    claims: Claims,
) -> impl Responder {
    let user_id = match path.user_id.parse::<i32>() {
        Ok(id) => id,
        Err(_) => return HttpResponse::BadRequest().body("Invalid user ID"),
    };

    if claims.sub != user_id {
        return HttpResponse::Forbidden().body("Forbidden");
    }

    // if sub_req.url isn't a valid URL, return 400
    if let Err(_) = url::Url::parse(&sub_req.url) {
        return HttpResponse::BadRequest().body("Invalid feed URL");
    }

    let mut conn = match pool.get() {
        Ok(conn) => conn,
        Err(err) => {
            log::error!("Failed to get db connection from pool: {}", err);
            return HttpResponse::InternalServerError().body("Error connecting to database");
        }
    };

    // check for an existing feed to this URL
    let feed = match Feed::get_by_url(&mut conn, &sub_req.url) {
        Some(feed) => feed,
        None => {
            // if no feed exists, create one
            let new_feed = NewFeed {
                url: &sub_req.url,
                ..Default::default()
            };
            let new_feed = new_feed.insert(&mut conn);
            match new_feed {
                Some(feed) => feed,
                None => {
                    return HttpResponse::InternalServerError().body("Error creating feed");
                }
            }
        }
    };

    // if the user already has a subscription to this feed, return 400
    let user_subs = match Subscription::get_all_for_user(&mut conn, user_id) {
        Ok(subs) => subs,
        Err(_) => return HttpResponse::InternalServerError().body("Error getting subscriptions"),
    };
    if let Some(_) = user_subs.iter().find(|s| s.feed_id == feed.id) {
        return HttpResponse::BadRequest().body("User already subscribed to this feed");
    }

    let mut new_sub = NewSubscription {
        user_id,
        feed_id: feed.id,
        frequency: sub_req.frequency.clone(),
        ..Default::default()
    };

    if let Some(max_items) = &sub_req.max_items {
        new_sub.max_items = *max_items;
    }

    if let Some(friendly_name) = &sub_req.friendly_name {
        new_sub.friendly_name = friendly_name.clone();
    }

    let subscription = match new_sub.insert(&mut conn) {
        Some(subscription) => subscription,
        None => {
            return HttpResponse::InternalServerError().body("Error creating subscription");
        }
    };

    let res = SubscriptionResponse { subscription, feed };

    let res_json = serde_json::to_string(&res).unwrap();
    HttpResponse::Ok().body(res_json)
}

#[get("/{sub_id}")]
pub async fn get_subscription() -> impl Responder {
    HttpResponse::Ok().body("get_subscription")
}

#[patch("/{sub_id}")]
pub async fn update_subscription() -> impl Responder {
    HttpResponse::Ok().body("update_subscription")
}

#[delete("/{sub_id}")]
pub async fn delete_subscription(
    pool: RqDbPool,
    user_path: RqUserId,
    sub_path: RqSubId,
    claims: Claims,
) -> impl Responder {
    let user_id = match user_path.user_id.parse::<i32>() {
        Ok(id) => id,
        Err(_) => return HttpResponse::BadRequest().body("Invalid user ID"),
    };

    if claims.sub != user_id {
        return HttpResponse::Forbidden().body("Forbidden");
    }

    let sub_id = match sub_path.sub_id.parse::<i32>() {
        Ok(id) => id,
        Err(_) => return HttpResponse::BadRequest().body("Invalid subscription ID"),
    };

    let mut conn = match pool.get() {
        Ok(conn) => conn,
        Err(err) => {
            log::error!("Failed to get db connection from pool: {}", err);
            return HttpResponse::InternalServerError().body("Error connecting to database");
        }
    };

    let subscription = match Subscription::get_by_id(&mut conn, sub_id) {
        Some(subscription) => subscription,
        None => return HttpResponse::NotFound().body("Subscription not found"),
    };

    if subscription.user_id != user_id {
        return HttpResponse::Forbidden().body("Forbidden");
    }

    let delete_sub_ok = Subscription::delete(&mut conn, sub_id);
    let delete_feed_ok = Feed::delete(&mut conn, subscription.feed_id);

    if !delete_sub_ok || !delete_feed_ok {
        return HttpResponse::InternalServerError().body("Error deleting subscription");
    }

    HttpResponse::Ok().body("Subscription deleted")
}
