use actix_web::web;
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize)]
pub struct FeedPath {
    pub feed_id: String,
}

pub type RqFeedId = web::Path<FeedPath>;

#[derive(Debug, Deserialize)]
pub struct ValidateFeedRequest {
    pub url: String,
}

#[derive(Debug, Serialize)]
pub struct ValidateFeedResponse {
    pub valid: bool,
    pub title: Option<String>,
    pub description: Option<String>,
    pub error: Option<String>,
}
