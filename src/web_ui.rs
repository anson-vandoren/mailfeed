use actix_web::{delete, get, patch, post, web, HttpResponse, Result};
use askama::Template;
use serde::Deserialize;

use crate::{
    api::subscriptions::types::SubscriptionResponse,
    errors::{AppError, AppResult},
    models::{
        feed::Feed,
        subscription::Subscription,
        user::{User, UserQuery},
    },
    session::SessionClaims,
    RqDbPool,
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
    is_active: Option<String>,       // Checkbox sends value or nothing
    delivery_method: Option<String>, // "telegram_only", "email_only", "both"
}

#[derive(Deserialize)]
struct NewSubscriptionForm {
    url: String,
    friendly_name: Option<String>,
    frequency: String,
    delivery_method: Option<String>, // "telegram_only", "email_only", "both"
}

/// Serve the login page or redirect to dashboard if already logged in
#[get("/")]
pub async fn login_page(pool: RqDbPool, req: actix_web::HttpRequest) -> Result<HttpResponse> {
    // Check if user is already authenticated
    let session_id = match req.cookie("session_id") {
        Some(cookie) => cookie.value().to_string(),
        None => {
            // No session cookie, show login page
            let template = LoginTemplate { error: None };
            return Ok(HttpResponse::Ok()
                .content_type("text/html")
                .body(template.render().unwrap()));
        }
    };

    // Check if session is valid
    let mut conn = match pool.get() {
        Ok(conn) => conn,
        Err(_) => {
            // Database error, show login page
            let template = LoginTemplate { error: None };
            return Ok(HttpResponse::Ok()
                .content_type("text/html")
                .body(template.render().unwrap()));
        }
    };

    use crate::models::session::Session;
    match Session::get_valid(&mut conn, &session_id) {
        Some(_) => {
            // Valid session exists, redirect to dashboard
            Ok(HttpResponse::SeeOther()
                .append_header(("Location", "/dashboard"))
                .finish())
        }
        None => {
            // Invalid/expired session, show login page
            let template = LoginTemplate { error: None };
            Ok(HttpResponse::Ok()
                .content_type("text/html")
                .body(template.render().unwrap()))
        }
    }
}

/// Handle login form submission
#[post("/auth/login")]
pub async fn login_submit(pool: RqDbPool, form: web::Form<LoginForm>) -> AppResult<HttpResponse> {
    use crate::models::user::{User, UserQuery};
    use crate::security::validation;

    // Validate input
    if validation::validate_email(&form.email).is_err() {
        let template = LoginTemplate {
            error: Some("Invalid email format".to_string()),
        };
        return Ok(HttpResponse::Ok()
            .content_type("text/html")
            .body(template.render().unwrap()));
    }

    if form.password.is_empty() {
        let template = LoginTemplate {
            error: Some("Password cannot be empty".to_string()),
        };
        return Ok(HttpResponse::Ok()
            .content_type("text/html")
            .body(template.render().unwrap()));
    }

    let mut conn = pool.get().map_err(|_| AppError::ConnectionPoolError)?;

    // Get user
    let user = match User::get(&mut conn, UserQuery::Email(&form.email)) {
        Some(user) => user,
        None => {
            let template = LoginTemplate {
                error: Some("Invalid email or password".to_string()),
            };
            return Ok(HttpResponse::Ok()
                .content_type("text/html")
                .body(template.render().unwrap()));
        }
    };

    // Check if user is active
    if !user.is_active {
        let template = LoginTemplate {
            error: Some("Account is deactivated".to_string()),
        };
        return Ok(HttpResponse::Ok()
            .content_type("text/html")
            .body(template.render().unwrap()));
    }

    // Verify password
    let is_password_correct = match User::check_password(&user, &form.password) {
        Ok(is_correct) => is_correct,
        Err(_) => {
            let template = LoginTemplate {
                error: Some("Invalid email or password".to_string()),
            };
            return Ok(HttpResponse::Ok()
                .content_type("text/html")
                .body(template.render().unwrap()));
        }
    };

    if !is_password_correct {
        let template = LoginTemplate {
            error: Some("Invalid email or password".to_string()),
        };
        return Ok(HttpResponse::Ok()
            .content_type("text/html")
            .body(template.render().unwrap()));
    }

    // Create session and redirect
    use crate::models::session::Session;

    let session = match Session::create(&mut conn, user.id) {
        Ok(session) => session,
        Err(_) => {
            let template = LoginTemplate {
                error: Some("Login failed. Please try again.".to_string()),
            };
            return Ok(HttpResponse::Ok()
                .content_type("text/html")
                .body(template.render().unwrap()));
        }
    };

    // Set session cookie and redirect to dashboard
    let is_production = !cfg!(debug_assertions);

    log::info!("Web UI login successful: {}", user.login_email);

    Ok(HttpResponse::SeeOther()
        .cookie(
            actix_web::cookie::Cookie::build("session_id", &session.session_id)
                .secure(is_production)
                .http_only(true)
                .same_site(actix_web::cookie::SameSite::Strict)
                .expires(
                    actix_web::cookie::time::OffsetDateTime::from_unix_timestamp(
                        session.expires_at as i64,
                    )
                    .unwrap(),
                )
                .path("/")
                .finish(),
        )
        .append_header(("Location", "/dashboard"))
        .finish())
}

/// Handle logout
#[post("/auth/logout")]
pub async fn logout(pool: RqDbPool, req: actix_web::HttpRequest) -> AppResult<HttpResponse> {
    use crate::session::session_manager;

    let mut conn = pool.get().map_err(|_| AppError::ConnectionPoolError)?;

    // Extract session ID from cookie
    let session_id = match req.cookie("session_id") {
        Some(cookie) => cookie.value().to_string(),
        None => {
            // No session cookie, just redirect to login
            return Ok(HttpResponse::SeeOther()
                .append_header(("Location", "/"))
                .finish());
        }
    };

    // Clear session and redirect
    match session_manager::clear_session(&mut conn, &session_id) {
        Ok(_) => {
            log::info!("Web UI logout successful");
            Ok(HttpResponse::SeeOther()
                .append_header(("Location", "/"))
                .finish())
        }
        Err(_) => {
            // Even if clearing fails, redirect to login
            Ok(HttpResponse::SeeOther()
                .append_header(("Location", "/"))
                .finish())
        }
    }
}

/// Serve the dashboard page (requires authentication)
#[get("/dashboard")]
pub async fn dashboard(pool: RqDbPool, req: actix_web::HttpRequest) -> Result<HttpResponse> {
    // Check authentication and redirect to login if not authenticated
    let claims = match extract_session_claims(&pool, &req).await {
        Some(claims) => claims,
        None => {
            // Redirect to login page
            return Ok(HttpResponse::SeeOther()
                .append_header(("Location", "/"))
                .finish());
        }
    };
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
            enriched_subscriptions.push(SubscriptionResponse { subscription, feed });
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
    let feed = Feed::get_by_id(&mut conn, subscription.feed_id).ok_or(AppError::InternalError)?;

    let sub_response = SubscriptionResponse { subscription, feed };

    let template = SubscriptionEditTemplate { sub: sub_response };

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
    let feed = Feed::get_by_id(&mut conn, subscription.feed_id).ok_or(AppError::InternalError)?;

    let sub_response = SubscriptionResponse { subscription, feed };

    let template = SubscriptionItemTemplate { sub: sub_response };

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
    use crate::models::subscription::{DeliveryMethod, Frequency, PartialSubscription};

    let frequency = match form.frequency.as_str() {
        "realtime" => Frequency::Realtime,
        "hourly" => Frequency::Hourly,
        "daily" => Frequency::Daily,
        _ => return Err(AppError::invalid_input("frequency", "Invalid frequency")),
    };

    let delivery_method = match form.delivery_method.as_deref() {
        Some("telegram_only") => Some(DeliveryMethod::TelegramOnly),
        Some("email_only") => Some(DeliveryMethod::EmailOnly),
        Some("both") => Some(DeliveryMethod::Both),
        _ => None, // Keep existing if not specified
    };

    let update = PartialSubscription {
        friendly_name: form.friendly_name.clone(),
        frequency: Some(frequency),
        max_items: Some(form.max_items),
        is_active: Some(form.is_active.is_some()),
        last_sent_time: None, // Don't change this
        delivery_method,
    };

    // Save the updated subscription
    let subscription =
        Subscription::update(&mut conn, sub_id, &update).ok_or(AppError::DatabaseError)?;

    // Get feed information for the response
    let feed = Feed::get_by_id(&mut conn, subscription.feed_id).ok_or(AppError::InternalError)?;

    let sub_response = SubscriptionResponse { subscription, feed };

    let template = SubscriptionItemTemplate { sub: sub_response };

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
    Ok(HttpResponse::Ok().content_type("text/html").body(""))
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

    // Parse frequency and delivery method
    use crate::models::feed::{Feed, NewFeed};
    use crate::models::subscription::{DeliveryMethod, Frequency, NewSubscription};

    let frequency = match form.frequency.as_str() {
        "realtime" => Frequency::Realtime,
        "hourly" => Frequency::Hourly,
        "daily" => Frequency::Daily,
        _ => return Err(AppError::invalid_input("frequency", "Invalid frequency")),
    };

    let delivery_method = match form.delivery_method.as_deref() {
        Some("telegram_only") => DeliveryMethod::TelegramOnly,
        Some("email_only") => DeliveryMethod::EmailOnly,
        Some("both") => DeliveryMethod::Both,
        _ => DeliveryMethod::TelegramOnly, // Default to Telegram for backward compatibility
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

            new_feed.insert(&mut conn).ok_or(AppError::InternalError)?
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
        delivery_method, // Use the parsed delivery method
    };

    let subscription = new_subscription
        .insert(&mut conn)
        .ok_or(AppError::DatabaseError)?;

    let sub_response = SubscriptionResponse { subscription, feed };

    let template = SubscriptionItemTemplate { sub: sub_response };

    Ok(HttpResponse::Ok()
        .content_type("text/html")
        .body(template.render().unwrap()))
}

#[derive(Template)]
#[template(path = "settings.html")]
struct SettingsTemplate {
    user: User,
    email_config: Option<crate::models::email_config::EmailConfig>,
}

/// Serve the settings page
#[get("/settings")]
pub async fn settings_page(pool: RqDbPool, req: actix_web::HttpRequest) -> Result<HttpResponse> {
    // Check authentication and redirect to login if not authenticated
    let claims = match extract_session_claims(&pool, &req).await {
        Some(claims) => claims,
        None => {
            // Redirect to login page
            return Ok(HttpResponse::SeeOther()
                .append_header(("Location", "/"))
                .finish());
        }
    };
    let mut conn = pool.get().map_err(|_| AppError::ConnectionPoolError)?;

    // Get user details
    let user = User::get(&mut conn, UserQuery::Id(claims.sub))
        .ok_or(AppError::resource_not_found("User"))?;

    // Get email config if exists
    let email_config =
        crate::models::email_config::EmailConfig::get_by_user_id(&mut conn, claims.sub);

    let template = SettingsTemplate { user, email_config };

    Ok(HttpResponse::Ok()
        .content_type("text/html")
        .body(template.render().unwrap()))
}

/// Helper function to extract session claims for web UI routes
async fn extract_session_claims(
    pool: &RqDbPool,
    req: &actix_web::HttpRequest,
) -> Option<SessionClaims> {
    use crate::models::session::Session;
    use crate::models::user::{User, UserQuery};
    use crate::session::extract_session_cookie;

    // Extract session cookie
    let session_id = extract_session_cookie(req)?;

    // Get database connection
    let mut conn = pool.get().ok()?;

    // Get valid session
    let session = Session::get_valid(&mut conn, &session_id)?;

    // Get user details
    let user = User::get(&mut conn, UserQuery::Id(session.user_id))?;

    Some(SessionClaims {
        sub: user.id,
        role: user.role,
        email: user.login_email,
    })
}

pub fn routes() -> actix_web::Scope {
    web::scope("")
        .service(login_page)
        .service(login_submit)
        .service(logout)
        .service(dashboard)
        .service(settings_page)
        .service(subscription_create)
        .service(subscription_edit_form)
        .service(subscription_view)
        .service(subscription_update)
        .service(subscription_delete)
}
