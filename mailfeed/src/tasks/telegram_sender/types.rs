use crate::models::feed_item::FeedItem;

#[derive(Debug)]
pub struct FeedData {
    pub sub_id: i32,
    pub new_items: Vec<FeedItem>,
    pub feed_title: String,
    pub feed_link: String,
}

#[derive(Debug)]
pub struct TelegramDeliveryData {
    pub feed_data: Vec<FeedData>,
}