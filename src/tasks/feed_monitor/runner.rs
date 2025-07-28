use diesel::SqliteConnection;
use reqwest::Client;

use super::types::FeedUpdates;
use crate::{
    models::{
        feed::{Feed, PartialFeed},
        feed_item::NewFeedItem,
    },
    tasks::types::CHECK_INTERVAL,
    DbPool,
};

pub async fn start(pool: DbPool) {
    let http_client = Client::new();
    loop {
        let mut conn = match pool.get() {
            Ok(conn) => conn,
            Err(e) => {
                log::error!("Error getting DB connection: {e:?}");
                continue;
            }
        };
        let feeds: Vec<Feed> = match Feed::get_all(&mut conn) {
            Some(feeds) => feeds,
            None => {
                log::info!("No feeds found");
                tokio::time::sleep(CHECK_INTERVAL).await;
                continue;
            }
        };

        for feed in &feeds {
            let response = http_client.get(&feed.url)
                // See: https://stackoverflow.com/a/7001617/5155484
                .header(
                    "Accept",
                    "application/rss+xml, application/rdf+xml, application/atom+xml, application/feed+json, application/xml;q=0.9, text/xml;q=0.8"
                )
                .header(
                    "User-Agent",
                    "Mailfeed (https://github.com/anson-vandoren/mailfeed)"
                )
                .send().await;
            match response {
                Ok(response) => {
                    // Update last_checked timestamp regardless of success/failure
                    let last_checked_update = PartialFeed {
                        last_checked: Some(chrono::Utc::now().timestamp() as i32),
                        ..Default::default()
                    };
                    Feed::update(&mut conn, feed.id, &last_checked_update);
                    
                    if response.status().is_success() {
                        log::info!("Got response for feed {}", feed.url);
                        let body = response.text().await.unwrap();
                        parse_and_insert(&mut conn, &body, feed);
                        
                        // Clear any previous errors on successful fetch
                        let clear_error_update = PartialFeed {
                            error_time: Some(0),
                            error_message: Some("".to_string()),
                            ..Default::default()
                        };
                        Feed::update(&mut conn, feed.id, &clear_error_update);
                    } else {
                        let error_update = PartialFeed {
                            error_time: Some(chrono::Utc::now().timestamp() as i32),
                            error_message: Some(response.status().to_string()),
                            ..Default::default()
                        };
                        Feed::update(&mut conn, feed.id, &error_update);
                        log::warn!(
                            "Got non-success response for feed {}: {}",
                            feed.url,
                            response.status()
                        );
                    }
                }
                Err(e) => {
                    // Update last_checked and error info
                    let error_update = PartialFeed {
                        last_checked: Some(chrono::Utc::now().timestamp() as i32),
                        error_time: Some(chrono::Utc::now().timestamp() as i32),
                        error_message: Some(e.to_string()),
                        ..Default::default()
                    };
                    Feed::update(&mut conn, feed.id, &error_update);
                    log::warn!("Error getting feed {}: {:?}", feed.url, e);
                }
            }
        }
        let num_feeds = feeds.len();
        log::info!("Found {num_feeds} feeds");
        tokio::time::sleep(CHECK_INTERVAL).await;
    }
}

fn parse_and_insert(conn: &mut SqliteConnection, body: &str, feed: &Feed) {
    let parsed = match feed_rs::parser::parse(body.as_bytes()) {
        Ok(parsed) => parsed,
        Err(e) => {
            log::warn!("Error parsing feed: {e:?}");
            return;
        }
    };

    // Update feed if necessary
    let feed_updates = FeedUpdates::from_feed_rs(&parsed, feed);
    if feed_updates.is_some() {
        log::info!("Found updates: {feed_updates:?}, updating feed");
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
        let author = entry.authors.first().map(|a| a.name.as_str());
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
                log::warn!("Error inserting item: {e:?}");
            }
        }
    }

    log::info!("Added {num_added} items");
}
