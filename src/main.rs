extern crate diesel;

mod api;
mod claims;
mod errors;
mod global;
mod models;
mod schema;
mod security;
mod session;
mod tasks;
mod telegram;
#[cfg(test)]
mod test_helpers;
mod types;
mod web_ui;

use crate::session::SessionClaims;
use crate::global::init_jwt_secret;
use crate::models::user::{NewUser, PartialUser, User};
use actix_cors::Cors;
use actix_files::Files;
use actix_governor::Governor;
use actix_web::{middleware, web, App, HttpServer};
use clap::Parser;
use diesel::{
    prelude::*,
    r2d2::{self},
};
use diesel_migrations::MigrationHarness;
use diesel_migrations::{embed_migrations, EmbeddedMigrations};
use dotenvy::dotenv;
use std::env;

pub const MIGRATIONS: EmbeddedMigrations = embed_migrations!("src/migrations");

/// CLI options
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Create a new user
    #[clap(long)]
    create_admin: bool,
}

fn main() -> std::io::Result<()> {
    dotenv().ok();
    
    // Initialize structured logging
    // Initialize simple logging
    env_logger::init();

    let config = load_config();

    let db_pool = initialize_db_pool(config.db_path);
    tracing::info!("Running database migrations");
    let mut conn = db_pool.get().expect("Failed to get database connection");
    conn.run_pending_migrations(MIGRATIONS)
        .expect("Failed to run migrations");
    init_jwt_secret(&mut conn);

    let args = Args::parse();
    if args.create_admin {
        cli_create_user(&mut conn);
        return Ok(());
    }

    run_server(config.public_path, db_pool, config.port)
}

fn cli_create_user(db: &mut SqliteConnection) {
    println!("\nEnter user login email:");
    let mut email = String::new();
    std::io::stdin()
        .read_line(&mut email)
        .expect("Failed to read email");
    let email = email.trim();

    println!("Enter password:");
    let password = rpassword::read_password().expect("Failed to read password");
    let password = password.trim();

    println!("Enter password again:");
    let password2 = rpassword::read_password().expect("Failed to read password");
    let password2 = password2.trim();

    if password != password2 {
        println!("Passwords do not match");
        return;
    }

    let new_user = NewUser {
        email: email.to_string(),
        password: password.to_string(),
    };

    let claims = SessionClaims {
        sub: 0,
        email: "system@mailfeed".to_string(),
        role: "admin".to_string(),
    };

    let user = match User::create(db, &new_user, claims) {
        Ok(user) => user,
        Err(e) => {
            println!("Failed to create user: {:?}", e);
            return;
        }
    };

    let updates = PartialUser {
        role: Some("admin".to_string()),
        ..Default::default()
    };

    match User::update(db, user.id, &updates) {
        Ok(user) => {
            println!("User created successfully");
            // print the user to stdout
            println!("{:?}", user);
        }
        Err(e) => {
            println!("Failed to update user: {:?}", e);
        }
    }
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
            path.push("mailfeed-ui/build");
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
async fn run_server(public_path: String, db_pool: DbPool, port: u16) -> std::io::Result<()> {
    tracing::info!("Serving static files from {}", public_path);
    tracing::info!("Starting server at http://127.0.0.1:{}", port);
    
    // Initialize metrics
    // Removed metrics - keeping simple

    tokio::spawn(tasks::feed_monitor::runner::start(db_pool.clone()));
    tokio::spawn(tasks::telegram_sender::runner::start(db_pool.clone()));

    HttpServer::new(move || {
        let cors = Cors::default()
            .allow_any_origin()
            .allow_any_method()
            .allow_any_header()
            .supports_credentials()
            .max_age(3600);
            
        // Create rate limiters
        let general_rate_limiter = security::create_rate_limiter();
        let auth_rate_limiter = security::create_auth_rate_limiter();
            
        App::new()
            .wrap(tracing_actix_web::TracingLogger::default())
            .wrap(middleware::Compress::default())
            .wrap(middleware::NormalizePath::new(
                middleware::TrailingSlash::Trim,
            ))
            .wrap(security::SecurityHeaders) // Add security headers
            .wrap(cors)
            .app_data(web::Data::new(db_pool.clone()))
            .service(
                web::scope("/api/auth")
                    .wrap(Governor::new(&auth_rate_limiter)) // Strict rate limiting for auth
                    .service(api::auth::handlers::login)
                    .service(api::auth::handlers::logout)
                    .service(api::auth::handlers::password_reset)
                    .service(api::auth::handlers::password_reset_confirm)
                    .service(api::auth::handlers::change_password)
            )
            .service(api::health::routes()) // Health checks (no rate limiting)
            .service(
                web::scope("/api")
                    .wrap(Governor::new(&general_rate_limiter)) // General rate limiting for non-auth endpoints
                    .service(api::users::routes())
                    .service(api::subscriptions::routes())
                    .service(api::feeds::routes())
                    .service(api::config::routes())
                    .service(api::feed_items::routes())
            )
            .service(web_ui::routes()) // Web UI routes
            .service(Files::new("/static", &public_path))
    })
    .workers(1)
    .bind(("127.0.0.1", port))?
    .run()
    .await
}

type DbPool = r2d2::Pool<r2d2::ConnectionManager<SqliteConnection>>;
pub type RqDbPool = web::Data<DbPool>;
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
