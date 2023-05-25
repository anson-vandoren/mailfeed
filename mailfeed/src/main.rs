extern crate diesel;

mod handlers;
mod models;
mod routes;
mod schema;
mod test_helpers;

use crate::routes::configure;
use actix_files::Files;
use actix_web::{middleware, web, App, HttpServer};
use diesel::{
    prelude::*,
    r2d2::{self, ConnectionManager, Pool},
};
use dotenvy::dotenv;
use std::env;

fn main() -> std::io::Result<()> {
    dotenv().ok();
    env_logger::init_from_env(env_logger::Env::new().default_filter_or("info"));

    let config = load_config();

    let db_pool = initialize_db_pool(config.db_path);
    // TODO: Run migrations
    // TODO: if no user, prompt to create one

    run_server(config.public_path, db_pool, config.port)
}

struct AppConfig {
    public_path: String,
    db_path: String,
    port: u16,
}

fn load_config() -> AppConfig {
    let public_path = match env::var("MF_PUBLIC_PATH") {
        Ok(path) => {
            log::info!("Using public path from MF_PUBLIC_PATH: {}", path);
            path
        }
        Err(_) => {
            let mut path = env::current_dir().expect("Failed to get current directory");
            path.push("public");
            let res = path.to_str().unwrap().to_string();
            log::info!("Using default public path: {}", res);
            res
        }
    };
    let db_path = match env::var("MF_DATABASE_URL") {
        Ok(path) => {
            log::info!("Using database path from MF_DATABASE_URL: {}", path);
            path
        }
        Err(_) => {
            let mut path = env::current_dir().expect("Failed to get current directory");
            path.push("mailfeed.db");
            let res = path.to_str().unwrap().to_string();
            log::info!("Using default database path: {}", res);
            res
        }
    };
    let port = match env::var("MF_PORT") {
        Ok(port) => {
            log::info!("Using port from MF_PORT: {}", port);
            port.parse::<u16>().expect("Failed to parse MF_PORT")
        }
        Err(_) => {
            log::info!("Using default port: 8080");
            8080
        }
    };

    AppConfig {
        public_path,
        db_path,
        port,
    }
}

#[actix_web::main]
async fn run_server(
    public_path: String,
    db_pool: Pool<ConnectionManager<SqliteConnection>>,
    port: u16,
) -> std::io::Result<()> {
    log::info!("Serving static files from {}", public_path);
    log::info!("Starting server at http://127.0.0.1:{}", port);

    HttpServer::new(move || {
        App::new()
            .wrap(middleware::Logger::default())
            .wrap(middleware::Compress::default())
            .wrap(middleware::NormalizePath::new(
                middleware::TrailingSlash::Trim,
            ))
            .app_data(web::Data::new(db_pool.clone()))
            .configure(configure)
            .service(Files::new("/", &public_path).index_file("index.html"))
    })
    .workers(1)
    .bind(("127.0.0.1", port))?
    .run()
    .await
}

type DbPool = r2d2::Pool<r2d2::ConnectionManager<SqliteConnection>>;
fn initialize_db_pool(db_path: String) -> DbPool {
    dotenv().ok();

    let manager = r2d2::ConnectionManager::<SqliteConnection>::new(db_path);
    r2d2::Pool::builder()
        .build(manager)
        .expect("Database URL should be a valid path to SQLite DB file")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_initialize_db_pool() {
        let pool = initialize_db_pool(":memory:".to_string());
        let mut conn = pool.get().unwrap();
        let result = diesel::sql_query("SELECT 1").execute(&mut conn);
        assert_eq!(result, Ok(0));
    }
}
