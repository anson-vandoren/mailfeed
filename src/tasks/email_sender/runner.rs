use super::delivery::EmailDeliveryService;
use crate::models::{
    subscription::{DeliveryMethod, Subscription},
    email_config::EmailConfig,
    feed_item::FeedItem,
};
use crate::DbPool;
use diesel::prelude::*;
use log::{debug, error, info, warn};
use std::collections::HashMap;
use tokio::time::Duration;

pub async fn start(pool: DbPool) {
    info!("Starting email sender runner");
    let mut delivery_service = EmailDeliveryService::new();

    let mut interval = tokio::time::interval(Duration::from_secs(300)); // 5 minutes
    loop {
        interval.tick().await;
        
        let mut conn = match pool.get() {
            Ok(conn) => conn,
            Err(e) => {
                error!("Error getting DB connection: {e:?}");
                continue;
            }
        };

        if let Err(e) = process_pending_emails(&mut delivery_service, &mut conn).await {
            error!("Error in email sender: {e}");
        }
    }
}

/// Process all pending email deliveries
async fn process_pending_emails(
    delivery_service: &mut EmailDeliveryService,
    conn: &mut diesel::SqliteConnection,
) -> Result<(), Box<dyn std::error::Error>> {
    debug!("Processing pending email deliveries");

    // Get all subscriptions that have email delivery enabled
    let email_subscriptions = get_email_subscriptions(conn)?;
    
    if email_subscriptions.is_empty() {
        debug!("No email subscriptions found");
        return Ok(());
    }

    info!("Found {} email subscriptions to process", email_subscriptions.len());

    // Group subscriptions by user and frequency for efficient processing
    let mut user_subscriptions: HashMap<i32, Vec<Subscription>> = HashMap::new();
    
    for subscription in email_subscriptions {
        user_subscriptions
            .entry(subscription.user_id)
            .or_default()
            .push(subscription);
    }

    // Process each user's subscriptions
    for (user_id, subscriptions) in user_subscriptions {
        if let Err(e) = process_user_emails(delivery_service, conn, user_id, subscriptions).await {
            error!("Failed to process emails for user {user_id}: {e}");
            continue;
        }
    }

    Ok(())
}

/// Process email deliveries for a specific user
async fn process_user_emails(
    delivery_service: &mut EmailDeliveryService,
    conn: &mut diesel::SqliteConnection,
    user_id: i32,
    subscriptions: Vec<Subscription>,
) -> Result<(), Box<dyn std::error::Error>> {
    debug!("Processing emails for user {user_id}");

    // Get user's email configuration
    let email_config = match get_user_email_config(conn, user_id)? {
        Some(config) => config,
        None => {
            warn!("User {user_id} has email subscriptions but no email config");
            return Ok(());
        }
    };

    if !email_config.is_active {
        debug!("Email config is inactive for user {user_id}");
        return Ok(());
    }

    // Group subscriptions by frequency for digest emails
    let mut frequency_groups: HashMap<String, Vec<Subscription>> = HashMap::new();
    
    for subscription in subscriptions {
        if !subscription.is_active {
            continue;
        }
        
        frequency_groups
            .entry(subscription.frequency.to_string())
            .or_default()
            .push(subscription);
    }

    // Process each frequency group
    for (frequency, group_subscriptions) in frequency_groups {
        if let Err(e) = process_frequency_group(
            delivery_service,
            conn,
            &email_config,
            &frequency,
            group_subscriptions
        ).await {
            error!("Failed to process {frequency} emails for user {user_id}: {e}");
        }
    }

    Ok(())
}

/// Process a group of subscriptions with the same frequency
async fn process_frequency_group(
    delivery_service: &mut EmailDeliveryService,
    conn: &mut diesel::SqliteConnection,
    email_config: &EmailConfig,
    frequency: &str,
    subscriptions: Vec<Subscription>,
) -> Result<(), Box<dyn std::error::Error>> {
    debug!("Processing {} {} subscriptions for user {}", 
           subscriptions.len(), frequency, email_config.user_id);

    match frequency {
        "realtime" => {
            // Send individual emails for each new item
            for subscription in subscriptions {
                send_realtime_emails(delivery_service, conn, email_config, &subscription).await?;
            }
        }
        "hourly" | "daily" => {
            // Send digest emails
            send_digest_emails(delivery_service, conn, email_config, frequency, subscriptions).await?;
        }
        _ => {
            warn!("Unknown frequency: {frequency}");
        }
    }

    Ok(())
}

/// Send realtime emails for a subscription
async fn send_realtime_emails(
    delivery_service: &mut EmailDeliveryService,
    conn: &mut diesel::SqliteConnection,
    email_config: &EmailConfig,
    subscription: &Subscription,
) -> Result<(), Box<dyn std::error::Error>> {
    // Get new feed items that haven't been sent yet
    let new_items = get_new_feed_items(conn, subscription)?;
    
    if new_items.is_empty() {
        return Ok(());
    }

    debug!("Sending {} realtime emails for subscription {}", 
           new_items.len(), subscription.id);

    // Get feed title for context
    let feed_title = get_feed_title(conn, subscription.feed_id)?;

    // Send each item individually for realtime
    for item in &new_items {
        if let Err(e) = delivery_service
            .send_feed_item(email_config, item, &feed_title)
            .await
        {
            error!("Failed to send realtime email for item {}: {}", item.id, e);
            continue;
        }
    }

    // Mark items as sent
    mark_items_as_sent(conn, &new_items)?;

    Ok(())
}

/// Send digest emails for a group of subscriptions
async fn send_digest_emails(
    delivery_service: &mut EmailDeliveryService,
    conn: &mut diesel::SqliteConnection,
    email_config: &EmailConfig,
    frequency: &str,
    subscriptions: Vec<Subscription>,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut all_items = Vec::new();

    // Collect all new items from all subscriptions
    for subscription in &subscriptions {
        let items = get_new_feed_items(conn, subscription)?;
        let feed_title = get_feed_title(conn, subscription.feed_id)?;
        
        for item in items {
            all_items.push((item, feed_title.clone()));
        }
    }

    if all_items.is_empty() {
        debug!("No new items for {} digest for user {}", frequency, email_config.user_id);
        return Ok(());
    }

    debug!("Sending {} digest with {} items for user {}", 
           frequency, all_items.len(), email_config.user_id);

    // Send the digest
    if let Err(e) = delivery_service
        .send_digest(email_config, &all_items, frequency)
        .await
    {
        error!("Failed to send {} digest for user {}: {}", frequency, email_config.user_id, e);
        return Err(e.into());
    }

    // Mark all items as sent
    let items_only: Vec<FeedItem> = all_items.into_iter().map(|(item, _)| item).collect();
    mark_items_as_sent(conn, &items_only)?;

    Ok(())
}

/// Get all subscriptions that have email delivery enabled
fn get_email_subscriptions(conn: &mut diesel::SqliteConnection) -> Result<Vec<Subscription>, diesel::result::Error> {
    use crate::schema::subscriptions::dsl::*;
    
    subscriptions
        .filter(is_active.eq(true))
        .filter(delivery_method.ne(DeliveryMethod::TelegramOnly as i32))
        .load::<Subscription>(conn)
}

/// Get email configuration for a user
fn get_user_email_config(conn: &mut diesel::SqliteConnection, target_user_id: i32) -> Result<Option<EmailConfig>, diesel::result::Error> {
    use crate::schema::email_configs::dsl::*;
    
    email_configs
        .filter(user_id.eq(target_user_id))
        .first::<EmailConfig>(conn)
        .optional()
}

/// Get new feed items for a subscription that haven't been sent yet
fn get_new_feed_items(conn: &mut diesel::SqliteConnection, subscription: &Subscription) -> Result<Vec<FeedItem>, diesel::result::Error> {
    use crate::schema::feed_items::dsl::*;
    
    // For now, we'll use a simple approach: get items from the last hour for hourly,
    // last day for daily, and last 5 minutes for realtime
    let cutoff_time = match subscription.frequency.to_string().as_str() {
        "realtime" => chrono::Utc::now() - chrono::Duration::minutes(5),
        "hourly" => chrono::Utc::now() - chrono::Duration::hours(1),
        "daily" => chrono::Utc::now() - chrono::Duration::days(1),
        _ => chrono::Utc::now() - chrono::Duration::hours(1),
    };

    feed_items
        .filter(feed_id.eq(subscription.feed_id))
        .filter(pub_date.gt(cutoff_time.timestamp() as i32))
        .order(pub_date.desc())
        .limit(subscription.max_items as i64)
        .load::<FeedItem>(conn)
}

/// Get the title of a feed
fn get_feed_title(conn: &mut diesel::SqliteConnection, feed_id_val: i32) -> Result<String, diesel::result::Error> {
    use crate::schema::feeds::dsl::*;
    
    feeds
        .find(feed_id_val)
        .select(title)
        .first::<String>(conn)
}

/// Mark feed items as sent (placeholder - we might need to add a sent_at column)
fn mark_items_as_sent(_conn: &mut diesel::SqliteConnection, _items: &[FeedItem]) -> Result<(), Box<dyn std::error::Error>> {
    // TODO: In a production system, we'd want to track which items have been sent
    // to avoid sending duplicates. For now, we rely on the time-based cutoffs.
    // This could be implemented by adding a `sent_notifications` table or
    // `email_sent_at` column to track delivery status per user.
    
    debug!("Marked {} items as sent (placeholder)", _items.len());
    Ok(())
}