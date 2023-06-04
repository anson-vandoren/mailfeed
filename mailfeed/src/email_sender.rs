use diesel::SqliteConnection;

use crate::{
    feed_monitor::FEED_CHECK_INTERVAL,
    models::{
        feed_item::FeedItem,
        subscription::{Frequency, Subscription},
        user::User,
    },
    DbPool,
};

pub async fn start(pool: DbPool) {
    loop {
        let mut conn = match pool.get() {
            Ok(conn) => conn,
            Err(e) => {
                log::error!("Error getting DB connection: {:?}", e);
                continue;
            }
        };

        let users = User::get_all(&mut conn);
        // unwrap and get active users
        let users = users.into_iter().flatten().filter(|user| user.is_active);

        for user in users {
            let email_data = items_to_send_by_user(&mut conn, user.id);
            log::info!("Email data: {:?}", email_data);
        }

        // TODO: move to the top
        tokio::time::sleep(FEED_CHECK_INTERVAL).await;
    }
}

fn items_to_send_by_feed(
    conn: &mut SqliteConnection,
    feed_id: i32,
    time_after: i32,
) -> Vec<FeedItem> {
    FeedItem::get_by_feed(conn, feed_id)
        .into_iter()
        .flatten()
        .filter(|item| item.pub_date > time_after)
        .collect()
}

#[derive(Debug)]
struct FeedData {
    feed_id: i32,
    time_after: i32,
    new_items: Vec<FeedItem>,
}

#[derive(Debug)]
struct EmailData {
    user_id: i32,
    feed_data: Vec<FeedData>,
}

fn items_to_send_by_user(conn: &mut SqliteConnection, user_id: i32) -> EmailData {
    let subscriptions = Subscription::get_all_for_user(conn, user_id).unwrap();
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

        let new_items = items_to_send_by_feed(conn, feed_id, last_sent);
        feed_data.push(FeedData {
            feed_id,
            time_after: last_sent,
            new_items,
        });
    }
    EmailData { user_id, feed_data }
}
