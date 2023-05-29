use serde::{Deserialize, Serialize};

use crate::models::{
    feed::Feed,
    subscription::{Frequency, Subscription},
};

#[derive(Debug, Deserialize)]
pub struct SubscriptionCreate {
    // items from Subscription
    pub frequency: Frequency,
    pub friendly_name: Option<String>,
    pub max_items: Option<i32>,
    // items from Feed
    pub url: String,
}

#[derive(Debug, Serialize)]
pub struct SubscriptionResponse {
    pub subscription: Subscription,
    pub feed: Feed,
}
