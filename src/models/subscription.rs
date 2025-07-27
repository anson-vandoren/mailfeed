use super::user::User;
use crate::schema::*;
use diesel::{
    backend::Backend,
    deserialize::{self, FromSql, FromSqlRow},
    prelude::*,
    serialize::{self, Output, ToSql},
    sql_types::Integer,
    AsExpression,
};
use serde::{Deserialize, Serialize};
use std::fmt;

#[derive(Debug, Serialize, Deserialize, Queryable, Identifiable, Associations)]
#[diesel(belongs_to(User))]
#[diesel(table_name = subscriptions)]
pub struct Subscription {
    pub id: i32,
    pub user_id: i32,
    pub friendly_name: String,
    /// realtime, hourly, daily
    pub frequency: Frequency,
    /// zero if never sent
    pub last_sent_time: i32,
    /// zero if no limit
    pub max_items: i32,
    pub is_active: bool,
    pub feed_id: i32,
    // TODO: add send_existing option
}

#[repr(i32)]
#[derive(Debug, Serialize, Deserialize, AsExpression, Clone, Copy, FromSqlRow, PartialEq)]
#[diesel(sql_type=Integer)]
#[serde(rename_all = "snake_case")]
pub enum Frequency {
    Realtime = 0,
    Hourly = 1,
    Daily = 2,
}

impl fmt::Display for Frequency {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Frequency::Realtime => write!(f, "realtime"),
            Frequency::Hourly => write!(f, "hourly"),
            Frequency::Daily => write!(f, "daily"),
        }
    }
}

impl PartialEq<&str> for Frequency {
    fn eq(&self, other: &&str) -> bool {
        matches!((self, *other), (Frequency::Realtime, "realtime") | (Frequency::Hourly, "hourly") | (Frequency::Daily, "daily"))
    }
}

impl<DB> FromSql<Integer, DB> for Frequency
where
    DB: Backend,
    i32: FromSql<Integer, DB>,
{
    fn from_sql(bytes: DB::RawValue<'_>) -> deserialize::Result<Self> {
        match i32::from_sql(bytes)? {
            0 => Ok(Frequency::Realtime),
            1 => Ok(Frequency::Hourly),
            2 => Ok(Frequency::Daily),
            _ => Err("Unrecognized enum variant".into()),
        }
    }
}

impl<DB> ToSql<Integer, DB> for Frequency
where
    DB: Backend,
    i32: ToSql<Integer, DB>,
{
    fn to_sql<'b>(&'b self, out: &mut Output<'b, '_, DB>) -> serialize::Result {
        match self {
            Frequency::Realtime => 0.to_sql(out),
            Frequency::Hourly => 1.to_sql(out),
            Frequency::Daily => 2.to_sql(out),
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Insertable)]
#[diesel(table_name = subscriptions)]
pub struct NewSubscription {
    pub user_id: i32,
    pub friendly_name: String,
    /// realtime, hourly, daily
    pub frequency: Frequency,
    /// zero if never sent
    pub last_sent_time: i32,
    /// zero if no limit
    pub max_items: i32,
    pub is_active: bool,
    pub feed_id: i32,
}

impl Default for NewSubscription {
    fn default() -> Self {
        Self {
            user_id: 0,
            friendly_name: "".to_string(),
            frequency: Frequency::Realtime,
            last_sent_time: 0,
            max_items: 0,
            is_active: true,
            feed_id: 0,
        }
    }
}

#[derive(Debug, Default, Serialize, Deserialize, AsChangeset)]
#[diesel(table_name = subscriptions)]
pub struct PartialSubscription {
    pub friendly_name: Option<String>,
    /// realtime, hourly, daily
    pub frequency: Option<Frequency>,
    /// zero if never sent
    pub last_sent_time: Option<i32>,
    /// zero if no limit
    pub max_items: Option<i32>,
    pub is_active: Option<bool>,
}

impl NewSubscription {
    pub fn insert(&self, conn: &mut SqliteConnection) -> Option<Subscription> {
        use crate::schema::subscriptions::dsl::*;
        match diesel::insert_into(subscriptions)
            .values(self)
            .get_result(conn)
        {
            Ok(subscription) => Some(subscription),
            Err(e) => {
                log::warn!("Error inserting subscription: {:?}", e);
                None
            }
        }
    }
}

impl Subscription {
    pub fn get_by_id(conn: &mut SqliteConnection, id: i32) -> Option<Subscription> {
        use crate::schema::subscriptions::dsl::subscriptions;
        match subscriptions.find(id).first::<Subscription>(conn) {
            Ok(subscription) => Some(subscription),
            Err(e) => {
                log::warn!("Error getting subscription: {:?}", e);
                None
            }
        }
    }

    pub fn get_all(conn: &mut SqliteConnection) -> Option<Vec<Subscription>> {
        use crate::schema::subscriptions::dsl::subscriptions;
        match subscriptions.load::<Subscription>(conn) {
            Ok(found) => match found.len() {
                0 => None,
                _ => Some(found),
            },
            Err(e) => {
                log::warn!("Error getting subscriptions: {:?}", e);
                None
            }
        }
    }

    pub fn get_all_for_user(
        conn: &mut SqliteConnection,
        user_id: i32,
    ) -> Result<Vec<Subscription>, diesel::result::Error> {
        use crate::schema::subscriptions::dsl::{subscriptions, user_id as user_id_col};
        match subscriptions
            .filter(user_id_col.eq(user_id))
            .load::<Subscription>(conn)
        {
            Ok(found) => Ok(found),
            Err(e) => {
                log::warn!("Error getting subscriptions: {:?}", e);
                Err(e)
            }
        }
    }

    pub fn get_for_user_and_feed(
        conn: &mut SqliteConnection,
        user_id: i32,
        feed_id: i32,
    ) -> Result<Option<Subscription>, diesel::result::Error> {
        use crate::schema::subscriptions::dsl::{
            feed_id as feed_id_col, subscriptions, user_id as user_id_col,
        };
        match subscriptions
            .filter(user_id_col.eq(user_id))
            .filter(feed_id_col.eq(feed_id))
            .first::<Subscription>(conn)
        {
            Ok(found) => Ok(Some(found)),
            Err(e) => match e {
                diesel::result::Error::NotFound => Ok(None),
                _ => {
                    log::warn!("Error getting subscriptions: {:?}", e);
                    Err(e)
                }
            },
        }
    }

    pub fn update(
        conn: &mut SqliteConnection,
        sub_id: i32,
        update: &PartialSubscription,
    ) -> Option<Subscription> {
        use crate::schema::subscriptions::dsl::{id, subscriptions};
        match diesel::update(subscriptions.filter(id.eq(sub_id)))
            .set(update)
            .get_result(conn)
        {
            Ok(subscription) => Some(subscription),
            Err(e) => {
                log::warn!("Error updating subscription: {:?}", e);
                None
            }
        }
    }

    pub fn delete(conn: &mut SqliteConnection, sub_id: i32) -> bool {
        use crate::schema::subscriptions::dsl::{id, subscriptions};
        match diesel::delete(subscriptions.filter(id.eq(sub_id))).execute(conn) {
            Ok(_) => true,
            Err(e) => {
                log::warn!("Error deleting subscription: {:?}", e);
                false
            }
        }
    }
}
