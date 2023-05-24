use serde::{Serialize, Deserialize};

use crate::schema::*;

#[derive(Debug, Serialize, Deserialize, Queryable, Insertable)]
#[diesel(table_name = feed_items)]
pub struct FeedItem {
    pub id: i32,
    pub feed_id: i32,
    pub title: String,
    pub link: String,
    pub pub_date: i32,
    pub description: Option<String>,
    pub author: Option<String>,
    pub categories: Option<String>,
}


#[derive(Debug, Serialize, Deserialize, Queryable, Insertable)]
#[diesel(table_name = feeds)]
pub struct Feed {
    pub id: i32,
    pub url: String,
    pub feed_type: String,
    pub title: String,
    pub last_checked: i32, // zero if never checked
    pub last_updated: i32,
    pub error_time: i32, // zero if no error
    pub error_message: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Queryable, Insertable)]
#[diesel(table_name = subscriptions)]
pub struct Subscription {
    pub id: i32,
    pub user_id: i32,
    pub friendly_name: String,
    pub frequency: String, // realtime, hourly, daily
    pub last_sent_time: i32, // zero if never sent
    pub max_items: i32, // zero if no limit
    pub is_active: bool,
    pub feed_id: i32,
}