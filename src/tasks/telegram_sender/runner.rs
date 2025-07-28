use super::types::{FeedData, TelegramDeliveryData};
use crate::{
    models::{
        feed::Feed,
        feed_item::FeedItem,
        subscription::{Frequency, PartialSubscription, Subscription},
        user::User,
    },
    tasks::types::CHECK_INTERVAL,
    telegram::client::TelegramClient,
    DbPool,
};
use chrono::{TimeZone, Utc};
use diesel::SqliteConnection;

pub async fn start(pool: DbPool) {
    let mut interval = tokio::time::interval(CHECK_INTERVAL);
    loop {
        interval.tick().await;
        
        let mut conn = match pool.get() {
            Ok(conn) => conn,
            Err(e) => {
                log::error!("Error getting DB connection: {e:?}");
                continue;
            }
        };
        
        // Try to initialize Telegram client each time - allows for runtime configuration
        let telegram_client = match TelegramClient::new(&mut conn) {
            Ok(client) => client,
            Err(e) => {
                log::debug!("Telegram client not available (no token configured?): {e:?}");
                continue; // Skip this iteration, try again next time
            }
        };

        let users = User::get_all(&mut conn);
        // unwrap and get active users with Telegram chat IDs
        let users = users
            .into_iter()
            .flatten()
            .filter(|user| user.is_active && user.telegram_chat_id.is_some());

        for user in users {
            let telegram_chat_id = match &user.telegram_chat_id {
                Some(chat_id) => chat_id,
                None => continue, // Skip users without Telegram
            };

            let delivery_data = items_to_send_by_user(&mut conn, user.id);
            for feed_data in &delivery_data.feed_data {
                if feed_data.new_items.is_empty() {
                    log::debug!("No new items for sub_id={}", feed_data.sub_id);
                    continue;
                }

                let message_text = format_telegram_message(feed_data);
                
                match telegram_client
                    .send_html_message(telegram_chat_id, &message_text)
                    .await
                {
                    Ok(_) => {
                        log::info!(
                            "Telegram message sent to {} for sub_id={}",
                            telegram_chat_id,
                            feed_data.sub_id
                        );
                    }
                    Err(e) => {
                        log::error!("Error sending Telegram message: {e:?}");
                        continue;
                    }
                }

                let update = PartialSubscription {
                    last_sent_time: Some(Utc::now().timestamp() as i32),
                    ..Default::default()
                };
                let _ = Subscription::update(&mut conn, feed_data.sub_id, &update);
            }
        }
    }
}

fn items_to_send_by_user(conn: &mut SqliteConnection, user_id: i32) -> TelegramDeliveryData {
    let subscriptions = Subscription::get_all_for_user(conn, user_id).unwrap_or_default();
    let mut feed_data = Vec::new();
    
    for sub in subscriptions {
        let feed_id = sub.feed_id;
        let last_sent = sub.last_sent_time;

        // if last_sent + frequency is > now, skip
        let now = chrono::Utc::now().timestamp() as i32;
        let should_send = match sub.frequency {
            Frequency::Realtime => true,
            Frequency::Hourly => now - last_sent > 3600,
            Frequency::Daily => now - last_sent > 86400,
        };

        if !should_send {
            log::info!(
                "Not enough time elapsed to send again for {:?} with frequency={:?}",
                sub.friendly_name,
                sub.frequency,
            );
            continue;
        }

        let feed = match Feed::get_by_id(conn, feed_id) {
            Some(feed) => feed,
            None => continue,
        };

        let new_items = FeedItem::items_after(conn, feed_id, last_sent);
        if !new_items.is_empty() {
            feed_data.push(FeedData {
                sub_id: sub.id,
                new_items,
                feed_title: feed.title,
                feed_link: feed.url,
            });
        }
    }
    
    TelegramDeliveryData { feed_data }
}

fn escape_html_text(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

fn format_telegram_message(feed_data: &FeedData) -> String {
    let mut message = format!(
        "<b>📰 {}</b>\n<a href=\"{}\">View Feed</a>\n\n",
        escape_html_text(&feed_data.feed_title),
        feed_data.feed_link  // URLs in href attributes should not be HTML escaped
    );

    for item in &feed_data.new_items {
        let date_time = Utc.timestamp_opt(item.pub_date as i64, 0).unwrap();
        let description = item
            .description
            .as_deref()
            .unwrap_or("No description provided");
        
        // For Telegram HTML: escape text content but not URLs in href attributes
        let clean_title = escape_html_text(&item.title);
        
        // Telegram HTML formatting - URLs should not be HTML escaped in href
        message.push_str(&format!(
            "📄 <a href=\"{}\">{}</a>\n",
            item.link,  // Don't escape URLs in href attributes
            clean_title
        ));
        
        message.push_str(&format!(
            "🕐 {}\n",
            date_time.format("%Y-%m-%d %H:%M:%S")
        ));
        
        if let Some(author) = &item.author {
            message.push_str(&format!(
                "👤 {}\n",
                escape_html_text(author)
            ));
        }
        
        // Clean description and limit length for Telegram
        let clean_description = escape_html_text(description);
        let truncated_desc = if clean_description.len() > 200 {
            // Be careful with UTF-8 boundaries when truncating
            let mut truncate_at = 197;
            while truncate_at > 0 && !clean_description.is_char_boundary(truncate_at) {
                truncate_at -= 1;
            }
            format!("{}...", &clean_description[..truncate_at])
        } else {
            clean_description
        };
        
        message.push_str(&format!(
            "{truncated_desc}\n"
        ));
        
        message.push_str("─────────────\n");
    }

    // Telegram message length limit is 4096 characters
    // Be more conservative to avoid cutting off HTML tags
    if message.len() > 3900 {
        // Find the last complete feed item boundary (─────────────) to avoid cutting HTML
        let separator = "─────────────\n";
        let mut safe_truncate_pos = 3900;
        
        // Find UTF-8 boundary first
        while safe_truncate_pos > 0 && !message.is_char_boundary(safe_truncate_pos) {
            safe_truncate_pos -= 1;
        }
        
        // Now find the last separator before this position
        if let Some(last_separator) = message[..safe_truncate_pos].rfind(separator) {
            message.truncate(last_separator + separator.len());
            message.push_str("<i>... more items truncated ...</i>");
        } else {
            // Fallback: find last complete newline
            while safe_truncate_pos > 0 && message.chars().nth(safe_truncate_pos) != Some('\n') {
                safe_truncate_pos -= 1;
            }
            if safe_truncate_pos > 0 {
                message.truncate(safe_truncate_pos);
                message.push_str("\n\n<i>... message truncated ...</i>");
            }
        }
    }

    message
}