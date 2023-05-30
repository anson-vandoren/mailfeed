use std::time::Duration;

use crate::{models::feed::Feed, DbPool};

pub async fn start(pool: DbPool) {
    loop {
        let mut conn = match pool.get() {
            Ok(conn) => conn,
            Err(e) => {
                log::error!("Error getting DB connection: {:?}", e);
                continue;
            }
        };
        let num_feeds = Feed::get_all(&mut conn).unwrap().len();
        log::info!("Found {} feeds", num_feeds);
        tokio::time::sleep(Duration::from_secs(5)).await;
    }
}
