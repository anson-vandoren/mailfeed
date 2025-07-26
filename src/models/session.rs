use crate::schema::sessions;
use chrono::Utc;
use diesel::{prelude::*, result::Error as DieselError, SqliteConnection};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Queryable, Identifiable, Selectable, Serialize, Deserialize, Debug, Clone)]
#[diesel(table_name = sessions)]
pub struct Session {
    pub id: i32,
    pub session_id: String,
    pub user_id: i32,
    pub expires_at: i32,
    pub created_at: i32,
    pub last_accessed: i32,
}

#[derive(Insertable)]
#[diesel(table_name = sessions)]
pub struct NewSession {
    pub session_id: String,
    pub user_id: i32,
    pub expires_at: i32,
    pub created_at: i32,
    pub last_accessed: i32,
}

#[derive(AsChangeset)]
#[diesel(table_name = sessions)]
pub struct SessionUpdate {
    pub last_accessed: Option<i32>,
}

impl Session {
    /// Create a new session for a user
    pub fn create(conn: &mut SqliteConnection, user_id: i32) -> Result<Self, DieselError> {
        let now = Utc::now().timestamp() as i32;
        let expires_at = now + (30 * 24 * 60 * 60); // 30 days
        let session_id = Uuid::new_v4().to_string();

        let new_session = NewSession {
            session_id,
            user_id,
            expires_at,
            created_at: now,
            last_accessed: now,
        };

        diesel::insert_into(sessions::table)
            .values(&new_session)
            .returning(Session::as_returning())
            .get_result(conn)
    }

    /// Get session by session_id if not expired
    pub fn get_valid(conn: &mut SqliteConnection, session_id: &str) -> Option<Self> {
        let now = Utc::now().timestamp() as i32;
        
        sessions::table
            .filter(sessions::session_id.eq(session_id))
            .filter(sessions::expires_at.gt(now))
            .first(conn)
            .ok()
    }

    /// Update last_accessed timestamp for a session
    pub fn touch(&self, conn: &mut SqliteConnection) -> Result<(), DieselError> {
        let now = Utc::now().timestamp() as i32;
        let update = SessionUpdate {
            last_accessed: Some(now),
        };

        diesel::update(sessions::table.filter(sessions::id.eq(self.id)))
            .set(&update)
            .execute(conn)
            .map(|_| ())
    }

    /// Delete a specific session
    pub fn delete(conn: &mut SqliteConnection, session_id: &str) -> Result<(), DieselError> {
        diesel::delete(sessions::table.filter(sessions::session_id.eq(session_id)))
            .execute(conn)
            .map(|_| ())
    }

    /// Delete all sessions for a user
    pub fn delete_all_for_user(conn: &mut SqliteConnection, user_id: i32) -> Result<(), DieselError> {
        diesel::delete(sessions::table.filter(sessions::user_id.eq(user_id)))
            .execute(conn)
            .map(|_| ())
    }

    /// Clean up expired sessions
    pub fn cleanup_expired(conn: &mut SqliteConnection) -> Result<usize, DieselError> {
        let now = Utc::now().timestamp() as i32;
        
        diesel::delete(sessions::table.filter(sessions::expires_at.le(now)))
            .execute(conn)
    }
}