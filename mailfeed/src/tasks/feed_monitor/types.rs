use crate::models::feed::{FeedType, PartialFeed};

#[derive(Debug, Default)]
pub(super) struct FeedUpdates<'a> {
    feed_type: Option<FeedType>,
    title: Option<&'a str>,
    last_updated: Option<i32>,
}

impl<'a> From<FeedUpdates<'a>> for PartialFeed<'a> {
    fn from(updates: FeedUpdates<'a>) -> Self {
        PartialFeed {
            feed_type: updates.feed_type,
            title: updates.title,
            last_updated: updates.last_updated,
            ..Default::default()
        }
    }
}

impl<'a> FeedUpdates<'a> {
    fn new() -> Self {
        Self::default()
    }

    /// If existing feed has unknown type, set it based on mapping
    /// from feed_rs::model::FeedType to our FeedType
    fn set_feed_type(
        &mut self,
        parsed: &feed_rs::model::Feed,
        existing: &crate::models::feed::Feed,
    ) -> &mut Self {
        if existing.feed_type == FeedType::Unknown {
            self.feed_type = Some(match parsed.feed_type {
                feed_rs::model::FeedType::Atom => FeedType::Atom,
                // This is mostly for display purposes, so we don't care
                // about the different RSS versions
                feed_rs::model::FeedType::RSS0 => FeedType::Rss,
                feed_rs::model::FeedType::RSS1 => FeedType::Rss,
                feed_rs::model::FeedType::RSS2 => FeedType::Rss,
                feed_rs::model::FeedType::JSON => FeedType::JsonFeed,
            });
        }
        self
    }

    /// If existing feed has no title, set it from parsed feed
    fn set_title(
        &mut self,
        parsed: &'a feed_rs::model::Feed,
        existing: &crate::models::feed::Feed,
    ) -> &mut Self {
        if existing.title.is_empty() {
            if let Some(title) = &parsed.title {
                self.title = Some(&title.content);
            }
        }
        self
    }

    /// Set when this feed was updated, which is the later of the
    /// feed's updated time and the newest item's published time.
    fn set_last_updated(
        &mut self,
        parsed: &feed_rs::model::Feed,
        existing: &crate::models::feed::Feed,
    ) -> &mut Self {
        self.last_updated = parsed
            .updated
            .map(|updated| updated.timestamp() as i32)
            .and_then(|last_updated| {
                // Especially w/ RSS, feed.updated may be when the feed definition
                // was updated, but not when a newer item was added. So we also
                // check the newest item's published time.
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
            feed_type: self.feed_type,
            title: self.title,
            last_updated: self.last_updated,
        }
    }

    pub(super) fn from_feed_rs(
        parsed_feed: &'a feed_rs::model::Feed,
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
