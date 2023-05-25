extern crate diesel;

mod handlers;
mod models;
mod routes;
mod schema;
mod test_helpers;

use crate::routes::configure;
use actix_files::Files;
use actix_web::{middleware, web, App, HttpServer};
use diesel::{prelude::*, r2d2};
use dotenvy::dotenv;
use std::env;

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    dotenv().ok();
    env_logger::init_from_env(env_logger::Env::new().default_filter_or("info"));

    let public_path = env::var("MF_PUBLIC_PATH").expect("MF_PUBLIC_PATH must be set");
    log::info!("Serving static files from {}", public_path);

    let pool = initialize_db_pool();
    log::info!("Starting server at http://localhost:8080");

    HttpServer::new(move || {
        App::new()
            .wrap(middleware::Logger::default())
            .wrap(middleware::Compress::default())
            .wrap(middleware::NormalizePath::new(
                middleware::TrailingSlash::Trim,
            ))
            .app_data(web::Data::new(pool.clone()))
            .configure(configure)
            .service(Files::new("/", &public_path).index_file("index.html"))
    })
    .bind(("127.0.0.1", 8080))?
    .run()
    .await
}

type DbPool = r2d2::Pool<r2d2::ConnectionManager<SqliteConnection>>;
fn initialize_db_pool() -> DbPool {
    dotenv().ok();

    let database_url = env::var("MF_DATABASE_URL").expect("DATABASE_URL must be set");
    let manager = r2d2::ConnectionManager::<SqliteConnection>::new(database_url);
    r2d2::Pool::builder()
        .build(manager)
        .expect("Database URL should be a valid path to SQLite DB file")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_initialize_db_pool() {
        let pool = initialize_db_pool();
        let mut conn = pool.get().unwrap();
        let result = diesel::sql_query("SELECT 1").execute(&mut conn);
        assert_eq!(result, Ok(0));
    }
}
