use std::fmt::Debug;

use chrono::{DateTime, TimeZone, Utc};
use diesel::SqliteConnection;

use crate::{
    feed_monitor::FEED_CHECK_INTERVAL,
    models::{
        feed::Feed,
        feed_item::FeedItem,
        subscription::{Frequency, Subscription},
        user::{User, UserQuery},
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
            let email_text = format_items_to_email(email_data);
            log::info!("Sending email to {}:\n{}", user.send_email, email_text)
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
    feed_title: String,
    feed_link: String,
}

#[derive(Debug)]
struct EmailData {
    user_id: i32,
    user_email: String,
    feed_data: Vec<FeedData>,
}

fn items_to_send_by_user(conn: &mut SqliteConnection, user_id: i32) -> EmailData {
    let subscriptions = Subscription::get_all_for_user(conn, user_id).unwrap();
    let user_email = User::get(conn, UserQuery::Id(user_id)).unwrap().send_email;
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

        let feed = Feed::get_by_id(conn, feed_id).unwrap();

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
            feed_title: feed.title,
            feed_link: feed.url,
        });
    }
    EmailData {
        user_id,
        feed_data,
        user_email,
    }
}

fn format_items_to_email(email_data: EmailData) -> String {
    // create mutable String `result`, initialized w/ EMAIL_TEMPLATE_HEAD
    let mut result = EMAIL_TEMPLATE_HEAD.to_string();
    for feed_data in email_data.feed_data {
        result.push_str(&format!(
            "<h2>{}</h2>
            <a href='{}'>View Feed</a>",
            feed_data.feed_title, feed_data.feed_link
        ));
        for item in feed_data.new_items {
            let date_time = Utc.timestamp_opt(item.pub_date as i64, 0).unwrap();
            result.push_str(&format!(
                "<div class='feed-item'>
                    <h2><a href='{}'>{}</a></h2>
                    <time>{}</time>
                    <p>{}</p>
                    <p class='author'>{}</p>
                </div>",
                item.link,
                item.title,
                item.description
                    .unwrap_or("No description provided".to_string()),
                date_time.format("%Y-%m-%d %H:%M:%S"),
                item.author.unwrap_or("No author provided".to_string())
            ));
        }
        result.push_str("<hr />");
    }
    // push EMAIL_TEMPLATE_FOOT to `result`
    result.push_str(EMAIL_TEMPLATE_FOOT);
    result
}

const EMAIL_TEMPLATE_HEAD: &str = r#"<html>
<head>
  <meta charset='UTF-8' />
  <title>MailFeed Digest</title>
  <style>
    body { font-family: Arial, sans-serif; margin: 0; padding: 0; background-color: #f6f6f6; } .container { width:
    80%; margin: 0 auto; background-color: #ffffff; padding: 20px; } h1 { color: #333333; } .feed { margin-bottom:
    20px; } .feed-item { border-bottom: 1px solid #dddddd; padding: 10px 0; } .feed-item:last-child { border-bottom:
    0; } .feed-item h2 { margin: 0; font-size: 18px; } .feed-item a { color: #007bff; text-decoration: none; }
    .feed-item p { color: #666666; margin: 10px 0; } .feed-item time { color: #999999; font-size: 12px; } .author {
    color: #999999; font-size: 14px; }
  </style>
</head>
<body>
  <div class='container'>
    <h1>MailFeed Digest</h1>
    <div class='feed'>
"#;

const EMAIL_TEMPLATE_FOOT: &str = r#"
        </div>
      </div>
    </div>
  </body>
</html>
"#;
