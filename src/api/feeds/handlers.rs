use crate::{
    session::SessionClaims,
    models::{feed::Feed, subscription::Subscription},
    RqDbPool,
};
use serde_json;

use super::types::{RqFeedId, ValidateFeedRequest, ValidateFeedResponse};
use actix_web::{get, post, web, HttpResponse, Responder};
use reqwest;
use feed_rs::parser;

#[post("/validate")]
pub async fn validate_feed(req: web::Json<ValidateFeedRequest>, _claims: SessionClaims) -> impl Responder {
    let url = &req.url;
    
    // Validate URL format
    if url.is_empty() {
        return HttpResponse::Ok().json(ValidateFeedResponse {
            valid: false,
            title: None,
            description: None,
            error: Some("URL cannot be empty".to_string()),
        });
    }

    // Try to fetch and parse the feed
    let client = reqwest::Client::builder()
        .user_agent("Mailfeed/1.0")
        .timeout(std::time::Duration::from_secs(10))
        .build();

    let client = match client {
        Ok(c) => c,
        Err(e) => {
            log::error!("Failed to create HTTP client: {}", e);
            return HttpResponse::Ok().json(ValidateFeedResponse {
                valid: false,
                title: None,
                description: None,
                error: Some("Failed to create HTTP client".to_string()),
            });
        }
    };

    // Fetch the feed
    let response = match client.get(url).send().await {
        Ok(resp) => resp,
        Err(e) => {
            return HttpResponse::Ok().json(ValidateFeedResponse {
                valid: false,
                title: None,
                description: None,
                error: Some(format!("Failed to fetch feed: {}", e)),
            });
        }
    };

    if !response.status().is_success() {
        return HttpResponse::Ok().json(ValidateFeedResponse {
            valid: false,
            title: None,
            description: None,
            error: Some(format!("HTTP error: {}", response.status())),
        });
    }

    // Get response body
    let body = match response.bytes().await {
        Ok(bytes) => bytes,
        Err(e) => {
            return HttpResponse::Ok().json(ValidateFeedResponse {
                valid: false,
                title: None,
                description: None,
                error: Some(format!("Failed to read response: {}", e)),
            });
        }
    };

    // Parse the feed
    match parser::parse(&body[..]) {
        Ok(feed) => {
            HttpResponse::Ok().json(ValidateFeedResponse {
                valid: true,
                title: Some(feed.title.map(|t| t.content).unwrap_or_else(|| "Untitled Feed".to_string())),
                description: feed.description.map(|d| d.content),
                error: None,
            })
        }
        Err(e) => {
            HttpResponse::Ok().json(ValidateFeedResponse {
                valid: false,
                title: None,
                description: None,
                error: Some(format!("Failed to parse feed: {}", e)),
            })
        }
    }
}

#[get("")]
pub async fn get_all_feeds(pool: RqDbPool, claims: SessionClaims) -> impl Responder {
    // Only admins can view all feeds
    if claims.role != "admin" {
        return HttpResponse::Forbidden().json(serde_json::json!({
            "error": {
                "code": "FORBIDDEN", 
                "message": "Admin access required"
            }
        }));
    }

    let mut conn = match pool.get() {
        Ok(conn) => conn,
        Err(_) => return HttpResponse::InternalServerError().json(serde_json::json!({
            "error": {
                "code": "DATABASE_ERROR",
                "message": "Database connection failed"
            }
        }))
    };

    match Feed::get_all(&mut conn) {
        Some(feeds) => HttpResponse::Ok().json(feeds),
        None => HttpResponse::InternalServerError().json(serde_json::json!({
            "error": {
                "code": "DATABASE_ERROR", 
                "message": "Failed to retrieve feeds"
            }
        }))
    }
}


#[get("/{feed_id}")]
pub async fn get_feed(pool: RqDbPool, feed_path: RqFeedId, claims: SessionClaims) -> impl Responder {
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

    // Admins can view any feed, regular users need a subscription
    if claims.role == "admin" {
        let feed = Feed::get_by_id(&mut conn, feed_id);
        match feed {
            Some(f) => HttpResponse::Ok().json(f),
            None => HttpResponse::NotFound().body("Feed not found")
        }
    } else {
        // Regular users can only view feeds they're subscribed to
        let user_id = claims.sub;
        let subscription = Subscription::get_for_user_and_feed(&mut conn, user_id, feed_id);

        if subscription.is_err() {
            return HttpResponse::InternalServerError().body("Error getting feed");
        }

        let subscription = subscription.unwrap();

        if subscription.is_none() {
            return HttpResponse::NotFound().body("Feed not found");
        }

        let feed = Feed::get_by_id(&mut conn, feed_id);

        if feed.is_none() {
            return HttpResponse::NotFound().body("Feed not found");
        }

        let feed = feed.unwrap();

        HttpResponse::Ok().json(feed)
    }
}

