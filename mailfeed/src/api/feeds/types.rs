use actix_web::web;
use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct FeedPath {
    pub feed_id: String,
}

pub type RqFeedId = web::Path<FeedPath>;
