use actix_web::{get, post, patch, delete, web, HttpResponse, Result};
use askama::Template;
use serde::Deserialize;

use crate::{
    errors::{AppError, AppResult},
    session::SessionClaims,
    models::{
        user::{User, UserQuery},
        subscription::Subscription,
        feed::Feed,
    },
    RqDbPool,
    api::subscriptions::types::SubscriptionResponse,
};

#[derive(Template)]
#[template(path = "login.html")]
struct LoginTemplate {
    error: Option<String>,
}

#[derive(Template)]
#[template(path = "dashboard.html")]
#[allow(dead_code)]
struct DashboardTemplate {
    user: User,
    subscriptions: Vec<SubscriptionResponse>,
}

#[derive(Template)]
#[template(path = "subscription_item.html")]
struct SubscriptionItemTemplate {
    sub: SubscriptionResponse,
}

#[derive(Template)]
#[template(path = "subscription_edit.html")]
struct SubscriptionEditTemplate {
    sub: SubscriptionResponse,
}

#[derive(Deserialize)]
#[allow(dead_code)]
struct LoginForm {
    email: String,
    password: String,
}

#[derive(Deserialize)]
struct SubscriptionForm {
    friendly_name: Option<String>,
    frequency: String,
    max_items: i32,
    is_active: Option<String>, // Checkbox sends value or nothing
}

#[derive(Deserialize)]
struct NewSubscriptionForm {
    url: String,
    friendly_name: Option<String>,
    frequency: String,
}

/// Serve the login page
#[get("/")]
pub async fn login_page() -> Result<HttpResponse> {
    let template = LoginTemplate { error: None };
    Ok(HttpResponse::Ok()
        .content_type("text/html")
        .body(template.render().unwrap()))
}

/// Handle login form submission
#[post("/auth/login")]
pub async fn login_submit() -> AppResult<HttpResponse> {
    // For now, let's just redirect to dashboard
    // We'll implement proper auth in the next step
    Ok(HttpResponse::SeeOther()
        .append_header(("Location", "/dashboard"))
        .finish())
}

/// Handle logout
#[post("/auth/logout")]
pub async fn logout() -> Result<HttpResponse> {
    Ok(HttpResponse::SeeOther()
        .append_header(("Location", "/"))
        .finish())
}

/// Serve the dashboard page (requires authentication)
#[get("/dashboard")]
pub async fn dashboard(
    pool: RqDbPool,
    claims: SessionClaims,
) -> AppResult<HttpResponse> {
    let mut conn = pool.get().map_err(|_| AppError::ConnectionPoolError)?;
    
    // Get user details
    let user = User::get(&mut conn, UserQuery::Id(claims.sub))
        .ok_or(AppError::resource_not_found("User"))?;
    
    // Get user's subscriptions
    let subscriptions = Subscription::get_all_for_user(&mut conn, claims.sub)
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
    
    let template = DashboardTemplate {
        user,
        subscriptions: enriched_subscriptions,
    };
    
    Ok(HttpResponse::Ok()
        .content_type("text/html")
        .body(template.render().unwrap()))
}

/// Get a single subscription for editing
#[get("/subscriptions/{sub_id}/edit")]
pub async fn subscription_edit_form(
    pool: RqDbPool,
    path: web::Path<i32>,
    claims: SessionClaims,
) -> AppResult<HttpResponse> {
    let sub_id = path.into_inner();
    let user_id = claims.sub;

    let mut conn = pool.get().map_err(|_| AppError::ConnectionPoolError)?;

    // Get the subscription and verify ownership
    let subscription = Subscription::get_by_id(&mut conn, sub_id)
        .ok_or(AppError::resource_not_found("Subscription"))?;

    if subscription.user_id != user_id {
        return Err(AppError::Forbidden);
    }

    // Get feed information
    let feed = Feed::get_by_id(&mut conn, subscription.feed_id)
        .ok_or(AppError::InternalError)?;

    let sub_response = SubscriptionResponse {
        subscription,
        feed,
    };

    let template = SubscriptionEditTemplate {
        sub: sub_response,
    };

    Ok(HttpResponse::Ok()
        .content_type("text/html")
        .body(template.render().unwrap()))
}

/// Get a single subscription (for cancel)
#[get("/subscriptions/{sub_id}")]
pub async fn subscription_view(
    pool: RqDbPool,
    path: web::Path<i32>,
    claims: SessionClaims,
) -> AppResult<HttpResponse> {
    let sub_id = path.into_inner();
    let user_id = claims.sub;

    let mut conn = pool.get().map_err(|_| AppError::ConnectionPoolError)?;

    // Get the subscription and verify ownership
    let subscription = Subscription::get_by_id(&mut conn, sub_id)
        .ok_or(AppError::resource_not_found("Subscription"))?;

    if subscription.user_id != user_id {
        return Err(AppError::Forbidden);
    }

    // Get feed information
    let feed = Feed::get_by_id(&mut conn, subscription.feed_id)
        .ok_or(AppError::InternalError)?;

    let sub_response = SubscriptionResponse {
        subscription,
        feed,
    };

    let template = SubscriptionItemTemplate {
        sub: sub_response,
    };

    Ok(HttpResponse::Ok()
        .content_type("text/html")
        .body(template.render().unwrap()))
}

/// Update a subscription (PATCH)
#[patch("/subscriptions/{sub_id}")]
pub async fn subscription_update(
    pool: RqDbPool,
    path: web::Path<i32>,
    form: web::Form<SubscriptionForm>,
    claims: SessionClaims,
) -> AppResult<HttpResponse> {
    let sub_id = path.into_inner();
    let user_id = claims.sub;

    let mut conn = pool.get().map_err(|_| AppError::ConnectionPoolError)?;

    // Get the subscription and verify ownership
    let subscription = Subscription::get_by_id(&mut conn, sub_id)
        .ok_or(AppError::resource_not_found("Subscription"))?;

    if subscription.user_id != user_id {
        return Err(AppError::Forbidden);
    }

    // Create the update object
    use crate::models::subscription::{Frequency, PartialSubscription};
    
    let frequency = match form.frequency.as_str() {
        "realtime" => Frequency::Realtime,
        "hourly" => Frequency::Hourly,
        "daily" => Frequency::Daily,
        _ => return Err(AppError::invalid_input("frequency", "Invalid frequency")),
    };

    let update = PartialSubscription {
        friendly_name: form.friendly_name.clone(),
        frequency: Some(frequency),
        max_items: Some(form.max_items),
        is_active: Some(form.is_active.is_some()),
        last_sent_time: None, // Don't change this
    };

    // Save the updated subscription
    let subscription = Subscription::update(&mut conn, sub_id, &update)
        .ok_or(AppError::DatabaseError)?;

    // Get feed information for the response
    let feed = Feed::get_by_id(&mut conn, subscription.feed_id)
        .ok_or(AppError::InternalError)?;

    let sub_response = SubscriptionResponse {
        subscription,
        feed,
    };

    let template = SubscriptionItemTemplate {
        sub: sub_response,
    };

    Ok(HttpResponse::Ok()
        .content_type("text/html")
        .body(template.render().unwrap()))
}

/// Delete a subscription (DELETE)
#[delete("/subscriptions/{sub_id}")]
pub async fn subscription_delete(
    pool: RqDbPool,
    path: web::Path<i32>,
    claims: SessionClaims,
) -> AppResult<HttpResponse> {
    let sub_id = path.into_inner();
    let user_id = claims.sub;

    let mut conn = pool.get().map_err(|_| AppError::ConnectionPoolError)?;

    // Get the subscription and verify ownership
    let subscription = Subscription::get_by_id(&mut conn, sub_id)
        .ok_or(AppError::resource_not_found("Subscription"))?;

    if subscription.user_id != user_id {
        return Err(AppError::Forbidden);
    }

    // Delete the subscription
    if !Subscription::delete(&mut conn, sub_id) {
        return Err(AppError::DatabaseError);
    }

    // Return empty response (HTMX will remove the element)
    Ok(HttpResponse::Ok()
        .content_type("text/html")
        .body(""))
}

/// Create a new subscription (POST)
#[post("/subscriptions")]
pub async fn subscription_create(
    pool: RqDbPool,
    form: web::Form<NewSubscriptionForm>,
    claims: SessionClaims,
) -> AppResult<HttpResponse> {
    let user_id = claims.sub;

    let mut conn = pool.get().map_err(|_| AppError::ConnectionPoolError)?;

    // Parse frequency
    use crate::models::subscription::{Frequency, NewSubscription};
    use crate::models::feed::{Feed, NewFeed};
    
    let frequency = match form.frequency.as_str() {
        "realtime" => Frequency::Realtime,
        "hourly" => Frequency::Hourly,
        "daily" => Frequency::Daily,
        _ => return Err(AppError::invalid_input("frequency", "Invalid frequency")),
    };

    // Check if feed exists, create if not
    let feed = match Feed::get_by_url(&mut conn, &form.url) {
        Some(existing_feed) => existing_feed,
        None => {
            // Create new feed
            let new_feed = NewFeed {
                url: &form.url,
                feed_type: crate::models::feed::FeedType::Unknown,
                title: "".to_string(), // Will be updated when feed is first fetched
                last_checked: 0,
                last_updated: 0,
                error_time: 0,
                error_message: None,
            };
            
            new_feed.insert(&mut conn)
                .ok_or(AppError::InternalError)?
        }
    };

    // Check if user already has a subscription to this feed
    if let Ok(Some(_existing)) = Subscription::get_for_user_and_feed(&mut conn, user_id, feed.id) {
        return Err(AppError::FeedAlreadySubscribed);
    }

    // Create the subscription
    let new_subscription = NewSubscription {
        user_id,
        friendly_name: form.friendly_name.clone().unwrap_or_default(),
        frequency,
        last_sent_time: 0,
        max_items: 10, // Default to 10 items
        is_active: true,
        feed_id: feed.id,
    };

    let subscription = new_subscription.insert(&mut conn)
        .ok_or(AppError::DatabaseError)?;

    let sub_response = SubscriptionResponse {
        subscription,
        feed,
    };

    let template = SubscriptionItemTemplate {
        sub: sub_response,
    };

    Ok(HttpResponse::Ok()
        .content_type("text/html")
        .body(template.render().unwrap()))
}

pub fn routes() -> actix_web::Scope {
    web::scope("")
        .service(login_page)
        .service(login_submit)
        .service(logout)
        .service(dashboard)
        .service(subscription_create)
        .service(subscription_edit_form)
        .service(subscription_view)
        .service(subscription_update)
        .service(subscription_delete)
}