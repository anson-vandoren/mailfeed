use crate::schema::*;
use diesel::{
    backend::Backend,
    deserialize::{self, FromSql},
    prelude::*,
    serialize::{self, Output, ToSql},
    sql_types::Integer,
    AsExpression, FromSqlRow,
};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Queryable, Identifiable, PartialEq)]
#[diesel(table_name = feeds)]
pub struct Feed {
    pub id: i32,
    pub url: String,
    pub feed_type: FeedType,
    pub title: String,
    // TODO: update vv or remove
    pub last_checked: i32, // zero if never checked
    // TODO: is vv actually used
    pub last_updated: i32,
    // TODO: update vv
    pub error_time: i32, // zero if no error
    // TODO: update vv
    pub error_message: Option<String>,
}

#[repr(i32)]
#[derive(Debug, Serialize, Deserialize, PartialEq, Clone, Copy, AsExpression, FromSqlRow)]
#[diesel(sql_type=Integer)]
#[serde(rename_all = "snake_case")]
pub enum FeedType {
    Unknown,
    Atom,
    Rss,
    JsonFeed,
}

impl<DB> FromSql<Integer, DB> for FeedType
where
    DB: Backend,
    i32: FromSql<Integer, DB>,
{
    fn from_sql(bytes: DB::RawValue<'_>) -> deserialize::Result<Self> {
        match i32::from_sql(bytes)? {
            0 => Ok(FeedType::Unknown),
            1 => Ok(FeedType::Atom),
            2 => Ok(FeedType::Rss),
            3 => Ok(FeedType::JsonFeed),
            _ => Err("Unrecognized enum variant".into()),
        }
    }
}

impl<DB> ToSql<Integer, DB> for FeedType
where
    DB: Backend,
    i32: ToSql<Integer, DB>,
{
    fn to_sql<'b>(&'b self, out: &mut Output<'b, '_, DB>) -> serialize::Result {
        match self {
            FeedType::Unknown => 0.to_sql(out),
            FeedType::Atom => 1.to_sql(out),
            FeedType::Rss => 2.to_sql(out),
            FeedType::JsonFeed => 3.to_sql(out),
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Insertable)]
#[diesel(table_name = feeds)]
pub struct NewFeed<'a> {
    pub url: &'a str,
    pub feed_type: FeedType,
    pub title: String,
    /// zero if never checked
    pub last_checked: i32,
    pub last_updated: i32,
    /// zero if no error
    pub error_time: i32,
    pub error_message: Option<String>,
}

impl<'a> Default for NewFeed<'a> {
    fn default() -> Self {
        NewFeed {
            url: "",
            feed_type: FeedType::Unknown,
            title: String::new(),
            last_checked: 0,
            last_updated: 0,
            error_time: 0,
            error_message: None,
        }
    }
}

#[derive(Debug, Default, Serialize, Deserialize, AsChangeset)]
#[diesel(table_name = feeds)]
pub struct PartialFeed<'a> {
    pub url: Option<String>,
    pub feed_type: Option<FeedType>,
    pub title: Option<&'a str>,
    pub last_checked: Option<i32>,
    pub last_updated: Option<i32>,
    pub error_time: Option<i32>,
    pub error_message: Option<String>,
}

impl<'a> NewFeed<'a> {
    pub fn insert(&self, conn: &mut SqliteConnection) -> Option<Feed> {
        use crate::schema::feeds::dsl::*;
        match diesel::insert_into(feeds).values(self).get_result(conn) {
            Ok(feed) => Some(feed),
            Err(e) => {
                log::warn!("Error inserting feed: {:?}", e);
                None
            }
        }
    }
}

impl Feed {
    pub fn get_by_id(conn: &mut SqliteConnection, id: i32) -> Option<Feed> {
        use crate::schema::feeds::dsl::feeds;
        match feeds.find(id).first::<Feed>(conn) {
            Ok(feed) => Some(feed),
            Err(e) => {
                log::warn!("Error getting feed: {:?}", e);
                None
            }
        }
    }

    pub fn get_by_url(conn: &mut SqliteConnection, url: &str) -> Option<Feed> {
        use crate::schema::feeds::dsl::{feeds, url as url_col};
        match feeds.filter(url_col.eq(url)).first::<Feed>(conn) {
            Ok(feed) => Some(feed),
            Err(e) => {
                log::info!("Requested feed w/ URL '{}' not found: {:?}", url, e);
                None
            }
        }
    }

    pub fn get_all(conn: &mut SqliteConnection) -> Option<Vec<Feed>> {
        use crate::schema::feeds::dsl::feeds;
        match feeds.load::<Feed>(conn) {
            Ok(found) => Some(found), // Return empty vec instead of None
            Err(e) => {
                log::warn!("Error getting feeds: {:?}", e);
                None
            }
        }
    }

    pub fn update(conn: &mut SqliteConnection, feed_id: i32, update: &PartialFeed) -> Option<Feed> {
        use crate::schema::feeds::dsl::{feeds, id};
        match diesel::update(feeds.filter(id.eq(feed_id)))
            .set(update)
            .get_result(conn)
        {
            Ok(feed) => Some(feed),
            Err(e) => {
                log::warn!("Error updating feed: {:?}", e);
                None
            }
        }
    }

    pub fn delete(conn: &mut SqliteConnection, feed_id: i32) -> bool {
        use crate::schema::feeds::dsl::{feeds, id};
        match diesel::delete(feeds.filter(id.eq(feed_id))).execute(conn) {
            Ok(_) => true,
            Err(e) => {
                log::warn!("Error deleting feed: {:?}", e);
                false
            }
        }
    }
}
