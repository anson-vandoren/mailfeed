use std::{env, fmt::Debug};

use chrono::{TimeZone, Utc};
use diesel::SqliteConnection;
use lettre::{
    error::Error,
    message::{header::ContentType, MultiPart, SinglePart},
    transport::smtp::authentication::Credentials,
    Message, SmtpTransport, Transport,
};

use crate::{
    feed_monitor::FEED_CHECK_INTERVAL,
    models::{
        feed::Feed,
        feed_item::FeedItem,
        subscription::{Frequency, Subscription},
        user::User,
    },
    DbPool,
};

#[derive(Debug)]
pub struct EmailServerCfg {
    pub host: String,
    pub port: u16,
    pub username: String,
    pub password: String,
    pub from_email: String,
}

fn get_server_config() -> EmailServerCfg {
    let host = env::var("MF_SMTP_HOST").unwrap();
    let port = env::var("MF_SMTP_PORT").unwrap().parse::<u16>().unwrap();
    let username = env::var("MF_SMTP_USERNAME").unwrap();
    let password = env::var("MF_SMTP_PASSWORD").unwrap();
    let from_email = env::var("MF_FROM_EMAIL").unwrap();
    EmailServerCfg {
        host,
        port,
        username,
        password,
        from_email,
    }
}

pub async fn start(pool: DbPool) {
    let cfg = get_server_config();
    let sender = match SmtpTransport::relay(&cfg.host) {
        Ok(sender) => sender,
        Err(e) => {
            log::error!("Error creating email sender: {:?}", e);
            return;
        }
    }
    .credentials(Credentials::new(cfg.username, cfg.password))
    .build();

    loop {
        let mut conn = match pool.get() {
            Ok(conn) => conn,
            Err(e) => {
                log::error!("Error getting DB connection: {:?}", e);
                tokio::time::sleep(FEED_CHECK_INTERVAL).await;
                continue;
            }
        };

        let users = User::get_all(&mut conn);
        // unwrap and get active users
        let users = users.into_iter().flatten().filter(|user| user.is_active);

        for user in users {
            let email_data = items_to_send_by_user(&mut conn, user.id);
            let email_html = to_html_email(&email_data);
            let email_plain = to_plain_email(&email_data);
            let message =
                construct_email(&user.send_email, &cfg.from_email, &email_html, &email_plain);
            let message = match message {
                Ok(message) => message,
                Err(e) => {
                    log::error!("Error constructing email: {:?}", e);
                    tokio::time::sleep(FEED_CHECK_INTERVAL).await;
                    continue;
                }
            };
            let email_result = sender.send(&message);
            match email_result {
                Ok(_) => {
                    log::info!("Email sent to {}", user.send_email);
                }
                Err(e) => {
                    log::error!("Error sending email: {:?}", e);
                }
            }

            // TODO: mark having sent
            panic!("TODO: mark having sent");
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
    new_items: Vec<FeedItem>,
    feed_title: String,
    feed_link: String,
}

#[derive(Debug)]
struct EmailData {
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
            new_items,
            feed_title: feed.title,
            feed_link: feed.url,
        });
    }
    EmailData { feed_data }
}

fn construct_email(
    to_email: &str,
    from_email: &str,
    as_html: &str,
    as_plain: &str,
) -> Result<Message, Error> {
    Message::builder()
        // TODO: update this to env var
        .from(from_email.parse().unwrap())
        .to(to_email.parse().unwrap())
        .subject("MailFeed Digest")
        .multipart(
            MultiPart::alternative()
                .singlepart(
                    SinglePart::builder()
                        .header(ContentType::TEXT_PLAIN)
                        .body(as_plain.to_string()),
                )
                .singlepart(
                    SinglePart::builder()
                        .header(ContentType::TEXT_HTML)
                        .body(as_html.to_string()),
                ),
        )
}

fn to_html_email(email_data: &EmailData) -> String {
    // create mutable String `result`, initialized w/ EMAIL_TEMPLATE_HEAD
    let mut result = EMAIL_TEMPLATE_HEAD.to_string();
    for feed_data in &email_data.feed_data {
        result.push_str(&format!(
            "<h2>{}</h2>
            <a href='{}'>View Feed</a>",
            feed_data.feed_title, feed_data.feed_link
        ));
        for item in &feed_data.new_items {
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
                    .clone()
                    .unwrap_or("No description provided".to_string()),
                date_time.format("%Y-%m-%d %H:%M:%S"),
                item.author
                    .clone()
                    .unwrap_or("No author provided".to_string())
            ));
        }
        result.push_str("<hr />");
    }
    // push EMAIL_TEMPLATE_FOOT to `result`
    result.push_str(EMAIL_TEMPLATE_FOOT);
    result
}

fn to_plain_email(email_data: &EmailData) -> String {
    let mut result = "MailFeed Digest\n\n".to_string();
    for feed_data in &email_data.feed_data {
        result.push_str(&format!(
            "{}\nView Feed: {}\n",
            feed_data.feed_title, feed_data.feed_link
        ));
        for item in &feed_data.new_items {
            let date_time = Utc.timestamp_opt(item.pub_date as i64, 0).unwrap();
            let description = item
                .description
                .clone()
                .unwrap_or("No description provided".to_string());

            result.push_str(&format!(
                "{}\n{}\n{}\n{}\n{}\n----------\n\n",
                item.link,
                item.title,
                html_escape::decode_html_entities(&description),
                date_time.format("%Y-%m-%d %H:%M:%S"),
                item.author
                    .clone()
                    .unwrap_or("No author provided".to_string())
            ));
        }
        result.push('\n');
    }
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
