use crate::models::feed::{FeedType, PartialFeed};

#[derive(Debug, Default)]
pub(super) struct FeedUpdates {
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

impl FeedUpdates {
    fn new() -> Self {
        Self::default()
    }

    fn set_feed_type(
        &mut self,
        parsed: &feed_rs::model::Feed,
        existing: &crate::models::feed::Feed,
    ) -> &mut Self {
        if existing.feed_type == FeedType::Unknown {
            self.feed_type = Some(match parsed.feed_type {
                feed_rs::model::FeedType::Atom => FeedType::Atom,
                feed_rs::model::FeedType::RSS0 => FeedType::Rss,
                feed_rs::model::FeedType::RSS1 => FeedType::Rss,
                feed_rs::model::FeedType::RSS2 => FeedType::Rss,
                feed_rs::model::FeedType::JSON => FeedType::JsonFeed,
            });
        }
        self
    }

    fn set_title(
        &mut self,
        parsed: &feed_rs::model::Feed,
        existing: &crate::models::feed::Feed,
    ) -> &mut Self {
        if existing.title.is_empty() {
            if let Some(title) = &parsed.title {
                self.title = Some(title.content.clone());
            }
        }
        self
    }

    fn set_last_updated(
        &mut self,
        parsed: &feed_rs::model::Feed,
        existing: &crate::models::feed::Feed,
    ) -> &mut Self {
        self.last_updated = parsed
            .updated
            .map(|updated| updated.timestamp() as i32)
            .and_then(|last_updated| {
                let newest_item_ts = parsed
                    .entries
                    .first()
                    .and_then(|i| i.published.map(|p| p.timestamp() as i32));

                let last_updated = newest_item_ts.map_or(last_updated, |newest_item_ts| {
                    if newest_item_ts > last_updated {
                        newest_item_ts
                    } else {
                        last_updated
                    }
                });

                if existing.last_updated != last_updated {
                    Some(last_updated)
                } else {
                    None
                }
            });
        self
    }

    fn build(&mut self) -> Self {
        Self {
            feed_type: self.feed_type.take(),
            title: self.title.take(),
            last_updated: self.last_updated.take(),
        }
    }

    pub(super) fn from_feed_rs(
        parsed_feed: &feed_rs::model::Feed,
        existing_feed: &crate::models::feed::Feed,
    ) -> Self {
        FeedUpdates::new()
            .set_feed_type(parsed_feed, existing_feed)
            .set_title(parsed_feed, existing_feed)
            .set_last_updated(parsed_feed, existing_feed)
            .build()
    }

    pub(super) fn is_some(&self) -> bool {
        self.feed_type.is_some() || self.title.is_some() || self.last_updated.is_some()
    }
}
