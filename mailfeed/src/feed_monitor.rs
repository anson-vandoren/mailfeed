use diesel::SqliteConnection;
use feed_rs::parser;
use tokio::{task::spawn_blocking, time::Duration};

use reqwest::Client;

use crate::{
    models::{
        feed::{Feed, FeedType},
        feed_item::{FeedItem, NewFeedItem},
    },
    DbPool,
};

pub async fn start(pool: DbPool) {
    let http_client = Client::new();
    loop {
        let mut conn = match pool.get() {
            Ok(conn) => conn,
            Err(e) => {
                log::error!("Error getting DB connection: {:?}", e);
                continue;
            }
        };
        let feeds: Vec<Feed> =
            spawn_blocking(move || Feed::get_all(&mut conn).unwrap_or(Vec::new()))
                .await
                .unwrap();

        for feed in &feeds {
            let response = http_client.get(&feed.url).send().await;
            match response {
                Ok(response) => {
                    if response.status().is_success() {
                        log::info!("Got response for feed {}", feed.url);
                        let body = response.text().await.unwrap();
                        let (parsed_feed, feed_updates) = parse_feed(&body, &feed);
                        log::info!("Found updates: {:?}", feed_updates);
                        log::info!("Found {} items", parsed_feed.len());
                    } else {
                        log::warn!(
                            "Got non-success response for feed {}: {}",
                            feed.url,
                            response.status()
                        );
                    }
                }
                Err(e) => {
                    // TODO: update error in DB
                    log::warn!("Error getting feed {}: {:?}", feed.url, e);
                }
            }
        }
        let num_feeds = feeds.len();
        log::info!("Found {} feeds", num_feeds);
        tokio::time::sleep(Duration::from_secs(60)).await;
    }
}

#[derive(Debug)]
struct FeedUpdates {
    feed_type: Option<FeedType>,
    title: Option<String>,
    last_updated: Option<i32>,
}

impl Default for FeedUpdates {
    fn default() -> Self {
        FeedUpdates {
            feed_type: None,
            title: None,
            last_updated: None,
        }
    }
}

fn parse_feed(body: &str, feed: &Feed) -> (Vec<NewFeedItem>, FeedUpdates) {
    let mut feed_items = Vec::new();
    let mut feed_updates = FeedUpdates::default();
    let parsed = match feed_rs::parser::parse(body.as_bytes()) {
        Ok(parsed) => parsed,
        Err(e) => {
            log::warn!("Error parsing feed: {:?}", e);
            return (feed_items, feed_updates);
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
        feed_updates.feed_type = Some(FeedType::from(feed_type));
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
        let last_updated = last_updated as i32;
        if feed.last_updated != last_updated {
            feed_updates.last_updated = Some(last_updated);
        }
    }

    // insert new feed items
    for entry in parsed.entries {
        let title = entry.title.or_else(|| entry.summary.clone());
        let title = title
            .map(|t| t.content)
            .unwrap_or_else(|| feed.title.clone());
        let pub_date: i32 = entry.published.map(|p| p.timestamp() as i32).unwrap_or(0);

        // entry.authors may be an empty Vec
        let author = entry.authors.get(0).map(|a| a.name.clone());

        let item = NewFeedItem {
            feed_id: feed.id,
            title: title,
            link: entry.links[0].href.clone(),
            pub_date,
            description: entry.summary.map(|s| s.content),
            author,
        };

        feed_items.push(item);
    }

    (feed_items, feed_updates)
}

fn insert_feed_items_and_update_feed(conn: &mut SqliteConnection, parsed_feed: FeedItem) {
    // Implement this function to insert the new feed items into the `feed_items`
    // table and update the `last_checked` and `last_updated` fields in the `feeds` table
    unimplemented!()
}
