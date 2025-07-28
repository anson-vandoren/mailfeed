use super::feed::Feed;
use crate::schema::*;
use diesel::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Queryable, Identifiable, Associations, PartialEq)]
#[diesel(belongs_to(Feed))]
#[diesel(table_name = feed_items)]
pub struct FeedItem {
    pub id: i32,
    pub feed_id: i32,
    pub title: String,
    pub link: String,
    pub pub_date: i32,
    pub description: Option<String>,
    pub author: Option<String>,
}

#[derive(Debug, Default, Serialize, Deserialize, Insertable)]
#[diesel(table_name = feed_items)]
pub struct NewFeedItem<'a> {
    pub feed_id: i32,
    pub title: &'a str, // TODO: make optional
    pub link: &'a str,  // TODO: add link_title
    pub pub_date: i32,
    pub description: Option<&'a str>, // TODO: rename to summary
    pub author: Option<&'a str>,
}

impl<'a> NewFeedItem<'a> {
    pub fn insert(&self, conn: &mut SqliteConnection) -> Option<FeedItem> {
        use crate::schema::feed_items::dsl::*;
        match diesel::insert_into(feed_items)
            .values(self)
            .get_result(conn)
        {
            Ok(item) => Some(item),
            Err(e) => {
                log::warn!("Error inserting feed item: {e:?}");
                None
            }
        }
    }

    pub fn insert_if_not_present(
        &self,
        conn: &mut SqliteConnection,
    ) -> Result<Option<FeedItem>, diesel::result::Error> {
        use crate::schema::feed_items::dsl::*;

        if FeedItem::has(conn, self) {
            return Ok(None);
        }
        match diesel::insert_into(feed_items)
            .values(self)
            .get_result(conn)
        {
            Ok(item) => Ok(Some(item)),
            Err(e) => {
                log::warn!("Error inserting feed item: {e:?}");
                Err(e)
            }
        }
    }
}

impl FeedItem {
    pub fn get_by_id(conn: &mut SqliteConnection, id: i32) -> Option<FeedItem> {
        use crate::schema::feed_items::dsl::feed_items;
        match feed_items.find(id).first::<FeedItem>(conn) {
            Ok(item) => Some(item),
            Err(e) => {
                log::warn!("Error getting feed item: {e:?}");
                None
            }
        }
    }

    pub fn get_all(conn: &mut SqliteConnection) -> Option<Vec<FeedItem>> {
        use crate::schema::feed_items::dsl::feed_items;
        match feed_items.load::<FeedItem>(conn) {
            Ok(items) => match items.len() {
                0 => None,
                _ => Some(items),
            },
            Err(e) => {
                log::warn!("Error getting feed items: {e:?}");
                None
            }
        }
    }

    pub fn get_by_feed(conn: &mut SqliteConnection, feed_id: i32) -> Option<Vec<FeedItem>> {
        use crate::schema::feed_items::dsl::{feed_id as fid, feed_items};
        match feed_items.filter(fid.eq(feed_id)).load::<FeedItem>(conn) {
            Ok(items) => match items.len() {
                0 => None,
                _ => Some(items),
            },
            Err(e) => {
                log::warn!("Error getting feed items: {e:?}");
                None
            }
        }
    }

    pub fn items_after(
        conn: &mut SqliteConnection,
        feed_id: i32,
        time_after: i32,
    ) -> Vec<FeedItem> {
        use crate::schema::feed_items::dsl::{feed_id as fid, feed_items, pub_date};
        match feed_items
            .filter(fid.eq(feed_id))
            .filter(pub_date.gt(time_after))
            .load::<FeedItem>(conn)
        {
            Ok(items) => items,
            Err(e) => {
                log::warn!("Error getting feed items: {e:?}");
                Vec::new()
            }
        }
    }

    pub fn has(conn: &mut SqliteConnection, item: &NewFeedItem) -> bool {
        use crate::schema::feed_items::dsl::{feed_id, feed_items, link, pub_date};
        feed_items
            .filter(feed_id.eq(item.feed_id))
            .filter(link.eq(item.link))
            .filter(pub_date.eq(item.pub_date))
            .first::<FeedItem>(conn)
            .is_ok()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_helpers::get_test_db_connection;

    fn insert_items(conn: &mut SqliteConnection, num_items: i32, feed_id: i32) -> Vec<FeedItem> {
        let mut inserted = Vec::new();
        for i in 0..num_items {
            let item = NewFeedItem {
                feed_id,
                title: &format!("test_title_{i}"),
                link: &format!("http://test.com/{i}"),
                ..Default::default()
            };
            let fi = item.insert(conn);
            match fi {
                Some(fi) => inserted.push(fi),
                None => log::warn!("Error inserting feed item"),
            }
        }
        inserted
    }

    #[test]
    fn test_insert_feed_item() {
        let mut conn = get_test_db_connection();
        let binding = insert_items(&mut conn, 1, 1);
        let item = binding.first().unwrap();
        assert_eq!(item.feed_id, 1);
        assert_eq!(item.title, "test_title_0");
        assert_eq!(item.link, "http://test.com/0");
        assert_eq!(item.pub_date, 0);
        assert_eq!(item.description, None);
        assert_eq!(item.author, None);
    }

    #[test]
    fn test_invalid_id_returns_none() {
        let mut conn = get_test_db_connection();
        let item = FeedItem::get_by_id(&mut conn, 1);
        assert_eq!(item, None);

        insert_items(&mut conn, 3, 1);
        let item = FeedItem::get_by_id(&mut conn, -1);
        assert_eq!(item, None);

        let item = FeedItem::get_by_id(&mut conn, 0);
        assert_eq!(item, None);
    }

    #[test]
    fn test_get_all() {
        let mut conn = get_test_db_connection();
        let items = FeedItem::get_all(&mut conn);
        assert_eq!(items, None);

        insert_items(&mut conn, 3, 1);
        insert_items(&mut conn, 3, 2);
        let items = FeedItem::get_all(&mut conn);
        assert_eq!(items.unwrap().len(), 6);
    }

    #[test]
    fn test_get_by_feed() {
        let mut conn = get_test_db_connection();
        let items = FeedItem::get_by_feed(&mut conn, 1);
        assert_eq!(items, None);

        insert_items(&mut conn, 3, 1);
        insert_items(&mut conn, 3, 2);
        let items = FeedItem::get_by_feed(&mut conn, 1);
        assert_eq!(items.unwrap().len(), 3);
    }
}
