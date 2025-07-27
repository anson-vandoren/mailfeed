#[cfg(test)]
use crate::schema::users;
use diesel::prelude::*;
use diesel::r2d2::{self, ConnectionManager};
use diesel::sqlite::SqliteConnection;
use diesel_migrations::{embed_migrations, EmbeddedMigrations, MigrationHarness};
use tempfile::TempDir;

#[cfg(test)]
use crate::DbPool;

#[cfg(test)]
pub const MIGRATIONS: EmbeddedMigrations = embed_migrations!("src/migrations");


/// Create a test database with a temporary file
#[cfg(test)]
pub fn create_test_db() -> (TempDir, DbPool) {
    let temp_dir = tempfile::tempdir().expect("Failed to create temp directory");
    let db_path = temp_dir.path().join("test.db");
    let database_url = format!("sqlite://{}", db_path.display());

    let manager = ConnectionManager::<SqliteConnection>::new(database_url);
    let pool = r2d2::Pool::builder()
        .max_size(1)
        .build(manager)
        .expect("Failed to create pool");

    // Run migrations
    let mut conn = pool.get().expect("Failed to get connection");
    conn.run_pending_migrations(MIGRATIONS)
        .expect("Failed to run migrations");

    (temp_dir, pool)
}

/// Create an in-memory test database connection (legacy support)
#[cfg(test)]
pub fn get_test_db_connection() -> SqliteConnection {
    let mut conn = SqliteConnection::establish(":memory:")
        .unwrap_or_else(|_| panic!("Error connecting to in-memory SQLite database"));

    conn.run_pending_migrations(MIGRATIONS)
        .expect("Failed to run migrations");
    conn
}

/// Clean up test database
#[cfg(test)]
pub fn cleanup_test_db(conn: &mut SqliteConnection) {
    diesel::sql_query("DELETE FROM subscriptions").execute(conn).ok();
    diesel::sql_query("DELETE FROM feed_items").execute(conn).ok();
    diesel::sql_query("DELETE FROM feeds").execute(conn).ok();
    diesel::sql_query("DELETE FROM users").execute(conn).ok();
    diesel::sql_query("DELETE FROM settings").execute(conn).ok();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_test_db() {
        let (_temp_dir, pool) = create_test_db();
        let mut conn = pool.get().expect("Failed to get connection");
        
        // Verify we can query the database
        let result: i32 = diesel::sql_query("SELECT 1 as test")
            .get_result::<crate::types::TestResult>(&mut conn)
            .map(|r| r.test)
            .expect("Failed to query test database");
        
        assert_eq!(result, 1);
    }

    #[test]
    fn test_cleanup_test_db() {
        let (_temp_dir, pool) = create_test_db();
        let mut conn = pool.get().expect("Failed to get connection");
        
        // Clean up (should not fail on empty database)
        cleanup_test_db(&mut conn);
        
        // Verify cleanup worked by counting users
        let user_count: i64 = users::table.count().first(&mut conn).expect("Failed to count users");
        assert_eq!(user_count, 0);
    }
}
