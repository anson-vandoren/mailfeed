use actix_web::{delete, get, patch, post, web, HttpResponse, Responder};

use super::types::{RqSubId, SubscriptionCreate, SubscriptionResponse, SubscriptionUpdate};
use crate::{
    errors::{AppError, AppResult},
    security::validation,
    session::SessionClaims,
    models::{
        feed::{Feed, NewFeed},
        subscription::{NewSubscription, PartialSubscription, Subscription},
    },
    RqDbPool,
};

#[get("")]
pub async fn get_all_subscriptions(
    pool: RqDbPool,
    claims: SessionClaims,
) -> AppResult<HttpResponse> {
    let user_id = claims.sub;

    let mut conn = pool.get().map_err(|_| AppError::ConnectionPoolError)?;

    let subscriptions = Subscription::get_all_for_user(&mut conn, user_id)
        .map_err(|_| AppError::DatabaseError)?;

    // Enrich subscriptions with feed information
    let mut enriched_subscriptions = Vec::new();
    for subscription in subscriptions {
        if let Some(feed) = Feed::get_by_id(&mut conn, subscription.feed_id) {
            enriched_subscriptions.push(SubscriptionResponse {
                subscription,
                feed,
            });
        }
    }

    Ok(HttpResponse::Ok().json(enriched_subscriptions))
}

#[post("")]
pub async fn create_subscription(
    pool: RqDbPool,
    sub_req: web::Json<SubscriptionCreate>,
    claims: SessionClaims,
) -> AppResult<HttpResponse> {
    let user_id = claims.sub;

    // Enhanced URL validation
    if let Err(e) = validation::validate_url(&sub_req.url) {
        log::warn!("Invalid feed URL submitted by user {}: {}", user_id, e);
        return Err(AppError::invalid_input("url", "Invalid feed URL format"));
    }
    
    // Validate friendly name if provided
    if let Some(ref name) = sub_req.friendly_name {
        if let Err(e) = validation::validate_friendly_name(name) {
            log::warn!("Invalid friendly name submitted by user {}: {}", user_id, e);
            return Err(AppError::invalid_input("friendly_name", "Contains invalid characters"));
        }
    }
    
    // Validate max_items
    if let Some(max_items) = sub_req.max_items {
        if max_items < 1 || max_items > 100 {
            return Err(AppError::invalid_input("max_items", "Must be between 1 and 100"));
        }
    }

    let mut conn = pool.get().map_err(|_| AppError::ConnectionPoolError)?;

    // check for an existing feed to this URL
    let feed = match Feed::get_by_url(&mut conn, &sub_req.url) {
        Some(feed) => feed,
        None => {
            // if no feed exists, create one
            let new_feed = NewFeed {
                url: &sub_req.url,
                ..Default::default()
            };
            new_feed.insert(&mut conn)
                .ok_or(AppError::InternalError)?
        }
    };

    // if the user already has a subscription to this feed, return 400
    let user_subs = Subscription::get_all_for_user(&mut conn, user_id)?;
    if user_subs.iter().any(|s| s.feed_id == feed.id) {
        return Err(AppError::FeedAlreadySubscribed);
    }

    let mut new_sub = NewSubscription {
        user_id,
        feed_id: feed.id,
        frequency: sub_req.frequency,
        ..Default::default()
    };

    if let Some(max_items) = &sub_req.max_items {
        new_sub.max_items = *max_items;
    }

    if let Some(friendly_name) = &sub_req.friendly_name {
        new_sub.friendly_name = friendly_name.clone();
    }

    let subscription = new_sub.insert(&mut conn)
        .ok_or(AppError::InternalError)?;

    let res = SubscriptionResponse { subscription, feed };

    Ok(HttpResponse::Ok().json(res))
}

#[get("/{sub_id}")]
pub async fn get_subscription() -> impl Responder {
    HttpResponse::Ok().body("get_subscription")
}

#[patch("/{sub_id}")]
pub async fn update_subscription(
    pool: RqDbPool,
    sub_path: RqSubId,
    update_req: web::Json<SubscriptionUpdate>,
    claims: SessionClaims,
) -> AppResult<HttpResponse> {
    let user_id = claims.sub;

    let sub_id = sub_path.sub_id.parse::<i32>()
        .map_err(|_| AppError::invalid_input("sub_id", "Invalid subscription ID format"))?;

    let mut conn = pool.get().map_err(|_| AppError::ConnectionPoolError)?;

    // Verify subscription belongs to user
    let subscription = Subscription::get_by_id(&mut conn, sub_id)
        .ok_or(AppError::resource_not_found("Subscription"))?;

    if subscription.user_id != user_id {
        return Err(AppError::Forbidden);
    }

    // Create partial update
    let partial_update = PartialSubscription {
        frequency: update_req.frequency,
        friendly_name: update_req.friendly_name.clone(),
        max_items: update_req.max_items,
        is_active: update_req.is_active,
        ..Default::default()
    };

    // Update subscription
    let updated_subscription = Subscription::update(&mut conn, sub_id, &partial_update)
        .ok_or(AppError::InternalError)?;

    // Get feed information
    let feed = Feed::get_by_id(&mut conn, updated_subscription.feed_id)
        .ok_or(AppError::InternalError)?;

    let response = SubscriptionResponse {
        subscription: updated_subscription,
        feed,
    };

    Ok(HttpResponse::Ok().json(response))
}

#[delete("/{sub_id}")]
pub async fn delete_subscription(
    pool: RqDbPool,
    sub_path: RqSubId,
    claims: SessionClaims,
) -> AppResult<HttpResponse> {
    let user_id = claims.sub;

    let sub_id = sub_path.sub_id.parse::<i32>()
        .map_err(|_| AppError::invalid_input("sub_id", "Invalid subscription ID format"))?;

    let mut conn = pool.get().map_err(|_| AppError::ConnectionPoolError)?;

    let subscription = Subscription::get_by_id(&mut conn, sub_id)
        .ok_or(AppError::resource_not_found("Subscription"))?;

    if subscription.user_id != user_id {
        return Err(AppError::Forbidden);
    }

    let delete_sub_ok = Subscription::delete(&mut conn, sub_id);
    let delete_feed_ok = Feed::delete(&mut conn, subscription.feed_id);

    if !delete_sub_ok || !delete_feed_ok {
        return Err(AppError::InternalError);
    }

    Ok(HttpResponse::Ok().json(serde_json::json!({
        "message": "Subscription deleted successfully"
    })))
}
