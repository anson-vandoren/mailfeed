use diesel::SqliteConnection;
use tokio::{task::spawn_blocking, time::Duration};

use reqwest::Client;

use crate::{
    models::{
        feed::{Feed, FeedType, PartialFeed},
        feed_item::NewFeedItem,
    },
    DbPool,
};

pub async fn start(pool: DbPool) {
    let http_client = Client::new();
    loop {
        tokio::time::sleep(Duration::from_secs(60)).await;
        let mut conn = match pool.get() {
            Ok(conn) => conn,
            Err(e) => {
                log::error!("Error getting DB connection: {:?}", e);
                continue;
            }
        };
        let mut update_conn = match pool.get() {
            Ok(conn) => conn,
            Err(e) => {
                log::error!("Error getting DB connection: {:?}", e);
                continue;
            }
        };
        let feeds: Vec<Feed> = match Feed::get_all(&mut conn) {
            Some(feeds) => feeds,
            None => {
                log::info!("No feeds found");
                continue;
            }
        };

        for feed in &feeds {
            let response = http_client.get(&feed.url).send().await;
            match response {
                Ok(response) => {
                    if response.status().is_success() {
                        log::info!("Got response for feed {}", feed.url);
                        let body = response.text().await.unwrap();
                        parse_and_insert(&mut update_conn, &body, feed);
                    } else {
                        let error_update = PartialFeed {
                            error_time: Some(chrono::Utc::now().timestamp() as i32),
                            error_message: Some(response.status().to_string()),
                            ..Default::default()
                        };
                        Feed::update(&mut update_conn, feed.id, &error_update);
                        log::warn!(
                            "Got non-success response for feed {}: {}",
                            feed.url,
                            response.status()
                        );
                    }
                }
                Err(e) => {
                    let error_update = PartialFeed {
                        error_time: Some(chrono::Utc::now().timestamp() as i32),
                        error_message: Some(e.to_string()),
                        ..Default::default()
                    };
                    Feed::update(&mut update_conn, feed.id, &error_update);
                    log::warn!("Error getting feed {}: {:?}", feed.url, e);
                }
            }
        }
        let num_feeds = feeds.len();
        log::info!("Found {} feeds", num_feeds);
    }
}

#[derive(Debug, Default)]
struct FeedUpdates {
    feed_type: Option<FeedType>,
    title: Option<String>,
    last_updated: Option<i32>,
}

impl From<FeedUpdates> for PartialFeed {
    fn from(updates: FeedUpdates) -> Self {
        PartialFeed {
            feed_type: updates.feed_type,
            title: updates.title,
            last_updated: updates.last_updated,
            ..Default::default()
        }
    }
}

fn parse_and_insert(conn: &mut SqliteConnection, body: &str, feed: &Feed) {
    let mut feed_updates = FeedUpdates::default();
    let parsed = match feed_rs::parser::parse(body.as_bytes()) {
        Ok(parsed) => parsed,
        Err(e) => {
            log::warn!("Error parsing feed: {:?}", e);
            return;
        }
    };
    // update feed.feed_type if it is FeedType::Unknown
    if feed.feed_type == FeedType::Unknown {
        let feed_type = match parsed.feed_type {
            feed_rs::model::FeedType::Atom => FeedType::Atom,
            feed_rs::model::FeedType::RSS0 => FeedType::Rss,
            feed_rs::model::FeedType::RSS1 => FeedType::Rss,
            feed_rs::model::FeedType::RSS2 => FeedType::Rss,
            feed_rs::model::FeedType::JSON => FeedType::JsonFeed,
        };
        feed_updates.feed_type = Some(feed_type);
    }

    // update feed.title if it is an empty string
    if feed.title.is_empty() {
        if let Some(title) = parsed.title {
            feed_updates.title = Some(title.content);
        }
    }

    // update feed.last_updated if parsed.updated is Some
    if let Some(updated) = parsed.updated {
        let last_updated = updated.timestamp() as i32;
        let last_updated = last_updated;
        if feed.last_updated != last_updated {
            feed_updates.last_updated = Some(last_updated);
        }
    }
    // only update the feed if there are some Some
    // values in the updates
    if feed_updates.feed_type.is_some()
        || feed_updates.title.is_some()
        || feed_updates.last_updated.is_some()
    {
        log::info!("Found updates: {:?}, updating feed", feed_updates);
        Feed::update(conn, feed.id, &feed_updates.into());
    }

    log::info!("Found {} items", parsed.entries.len());
    let mut num_added = 0;

    // insert new feed items
    for entry in parsed.entries {
        let title = entry.title.or_else(|| entry.summary.clone());
        let title = title
            .map(|t| t.content)
            .unwrap_or_else(|| feed.title.clone());
        let pub_date: i32 = entry.published.map(|p| p.timestamp() as i32).unwrap_or(0);

        // entry.authors may be an empty Vec
        let author = entry.authors.get(0).map(|a| a.name.as_str());
        let description = entry.summary.map(|s| s.content);

        let item = NewFeedItem {
            feed_id: feed.id,
            title: &title,
            link: &entry.links[0].href,
            pub_date,
            description: description.as_deref(),
            author,
        };
        let result = item.insert_if_not_present(conn);
        match result {
            Ok(Some(_)) => {
                num_added += 1;
            }
            Ok(None) => {
                log::debug!("Item already exists: {:?}", item.link);
            }
            Err(e) => {
                log::warn!("Error inserting item: {:?}", e);
            }
        }
    }

    log::info!("Added {} items", num_added);
}
