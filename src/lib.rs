pub mod api;
pub mod claims;
pub mod errors;
pub mod global;
pub mod models;
pub mod schema;
pub mod security;
pub mod session;
pub mod tasks;
pub mod telegram_client;
#[cfg(test)]
pub mod test_helpers;
pub mod types;
pub mod web_ui;

// Type definitions
use actix_web::web;
use diesel::r2d2::{self, ConnectionManager};
use diesel::sqlite::SqliteConnection;

pub type DbPool = r2d2::Pool<ConnectionManager<SqliteConnection>>;
pub type RqDbPool = web::Data<DbPool>;

// Re-export test helpers for integration tests
#[cfg(test)]
pub use test_helpers::*;
