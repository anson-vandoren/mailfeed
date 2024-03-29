use super::types::{
    EmailData, EmailServerCfg, FeedData, FromEmail, MultiPartEmailContent, ToEmail,
};
use crate::{
    models::{
        feed::Feed,
        feed_item::FeedItem,
        subscription::{Frequency, PartialSubscription, Subscription},
        user::User,
    },
    tasks::types::CHECK_INTERVAL,
    DbPool,
};
use chrono::{TimeZone, Utc};
use diesel::SqliteConnection;
use lettre::{
    error::Error,
    message::{header::ContentType, MultiPart, SinglePart},
    Message, Transport,
};

pub async fn start(pool: DbPool) {
    let cfg = EmailServerCfg::from_env();
    // return early if we can't create the sender
    let sender = match cfg.to_transport() {
        Ok(sender) => sender,
        Err(e) => {
            log::error!("Error creating email sender: {:?}", e);
            return;
        }
    };

    let mut interval = tokio::time::interval(CHECK_INTERVAL);
    loop {
        interval.tick().await;
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
            for feed_data in &email_data.feed_data {
                if feed_data.new_items.is_empty() {
                    log::debug!("No new items for sub_id={}", feed_data.sub_id);
                    continue;
                }
                let as_plain = to_plain_email(feed_data);
                let as_html = to_html_email(feed_data);
                let content = MultiPartEmailContent {
                    as_plain: &as_plain,
                    as_html: &as_html,
                };

                let subject = &cfg.email_subject
                    .replace("{feed_title}", &feed_data.feed_title)
                    .replace("{feed_link}", &feed_data.feed_link)
                    .replace("{sub_id}", &feed_data.sub_id.to_string())
                    .replace("{new_items_count}", &feed_data.new_items.len().to_string());
                let message = construct_email(subject, &user.send_email, &cfg.from_email, content);
                let message = match message {
                    Ok(message) => message,
                    Err(e) => {
                        log::error!("Error constructing email: {:?}", e);
                        continue;
                    }
                };
                let email_result = sender.send(&message);
                match email_result {
                    Ok(_) => {
                        log::info!(
                            "Email sent to {} for sub_id={}",
                            user.send_email,
                            feed_data.sub_id
                        );
                    }
                    Err(e) => {
                        log::error!("Error sending email: {:?}", e);
                        continue;
                    }
                }

                let update = PartialSubscription {
                    last_sent_time: Some(Utc::now().timestamp() as i32),
                    ..Default::default()
                };
                Subscription::update(&mut conn, feed_data.sub_id, &update);
            }
        }
    }
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

        let new_items = FeedItem::items_after(conn, feed_id, last_sent);
        feed_data.push(FeedData {
            sub_id: sub.id,
            new_items,
            feed_title: feed.title,
            feed_link: feed.url,
        });
    }
    EmailData { feed_data }
}

fn construct_email(
    subject: &str,
    to_email: ToEmail,
    from_email: FromEmail,
    content: MultiPartEmailContent,
) -> Result<Message, Error> {
    // TODO: settings entries for SMTP server
    // TODO: settings entry for updating From Name and From Email
    Message::builder()
        .from(from_email.parse().unwrap())
        .to(to_email.parse().unwrap())
        .subject(subject)
        .multipart(
            MultiPart::alternative()
                .singlepart(
                    SinglePart::builder()
                        .header(ContentType::TEXT_PLAIN)
                        .body(content.as_plain.to_string()),
                )
                .singlepart(
                    SinglePart::builder()
                        .header(ContentType::TEXT_HTML)
                        .body(content.as_html.to_string()),
                ),
        )
}

fn to_html_email(feed_data: &FeedData) -> String {
    let mut result = EMAIL_TEMPLATE_HEAD.to_string();
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
                .as_deref()
                .unwrap_or("No description provided"),
            date_time.format("%Y-%m-%d %H:%M:%S"),
            item.author.as_deref().unwrap_or("No author provided")
        ));
    }
    result.push_str("<hr />");
    result.push_str(EMAIL_TEMPLATE_FOOT);
    result
}

fn to_plain_email(feed_data: &FeedData) -> String {
    let mut result = "MailFeed Digest\n\n".to_string();
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
