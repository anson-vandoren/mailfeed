#[cfg(test)]
pub mod test_helpers {
    use crate::MIGRATIONS;
    use diesel::{Connection, SqliteConnection};
    use diesel_migrations::MigrationHarness;

    pub fn get_test_db_connection() -> SqliteConnection {
        let mut conn = SqliteConnection::establish(":memory:")
            .unwrap_or_else(|_| panic!("Error connecting to in-memory SQLite database"));

        conn.run_pending_migrations(MIGRATIONS)
            .expect("Failed to run migrations");
        conn
    }
}
