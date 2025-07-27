use actix_web::{test, web, App};
use mailfeed::{
    api::{auth, feeds, subscriptions, users, health},
    models::user::{NewUser, User, PartialUser},
    security::SecurityHeaders,
    session::SessionClaims,
    DbPool,
};
use diesel::r2d2::{self, ConnectionManager};
use diesel::sqlite::SqliteConnection;
use diesel_migrations::{embed_migrations, EmbeddedMigrations, MigrationHarness};
use tempfile::TempDir;

const MIGRATIONS: EmbeddedMigrations = embed_migrations!("src/migrations");

fn create_test_db() -> (TempDir, DbPool) {
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
use serde_json::{json, Value};

fn create_test_app_with_pool(
    pool: DbPool,
) -> App<
    impl actix_web::dev::ServiceFactory<
        actix_web::dev::ServiceRequest,
        Response = actix_web::dev::ServiceResponse,
        Error = actix_web::Error,
        Config = (),
        InitError = (),
    >,
> {
    App::new()
        .app_data(web::Data::new(pool))
        // Removed metrics for simplicity
        .wrap(SecurityHeaders)
        .service(health::routes())
        .service(
            web::scope("/api/auth")
                .service(auth::handlers::login)
                .service(auth::handlers::logout)
        )
        .service(
            web::scope("/api")
                .service(users::routes())
                .service(feeds::routes())
                .service(subscriptions::routes()),
        )
}

async fn create_admin_user(pool: &DbPool) -> (String, String) {
    let mut conn = pool.get().expect("Failed to get connection");
    
    let admin_email = "admin@example.com";
    let admin_password = "admin123";
    
    let admin_user = NewUser {
        email: admin_email.to_string(),
        password: admin_password.to_string(),
    };
    
    // Create admin with system admin claims
    let system_claims = SessionClaims {
        sub: 0,
        email: "system".to_string(),
        role: "admin".to_string(),
    };
    
    let created_admin = User::create(&mut conn, &admin_user, system_claims)
        .expect("Failed to create admin user");
    
    // Update the admin user to have admin role
    let admin_update = PartialUser {
        role: Some("admin".to_string()),
        login_email: None,
        send_email: None,
        is_active: None,
        daily_send_time: None,
        refresh_token: None,
        telegram_chat_id: None,
        telegram_username: None,
    };
    
    User::update(&mut conn, created_admin.id, &admin_update)
        .expect("Failed to update admin role");
    
    (admin_email.to_string(), admin_password.to_string())
}

// Helper macro to extract session from login response
macro_rules! extract_session_from_login {
    ($app:expr, $email:expr, $password:expr) => {{
        let login_data = json!({
            "email": $email,
            "password": $password
        });
        
        let req = test::TestRequest::post()
            .uri("/api/auth/login")
            .set_json(&login_data)
            .to_request();
        
        let resp = test::call_service($app, req).await;
        
        if resp.status().is_success() {
            // Extract session_id from Set-Cookie header
            if let Some(cookie_header) = resp.headers().get("set-cookie") {
                if let Ok(cookie_str) = cookie_header.to_str() {
                    // Parse session_id from cookie string like "session_id=abc123; ..."
                    if let Some(start) = cookie_str.find("session_id=") {
                        let start = start + "session_id=".len();
                        if let Some(end) = cookie_str[start..].find(';') {
                            Some(cookie_str[start..start + end].to_string())
                        } else {
                            // Cookie value extends to end of string
                            Some(cookie_str[start..].to_string())
                        }
                    } else {
                        None
                    }
                } else {
                    None
                }
            } else {
                None
            }
        } else {
            None
        }
    }};
}

#[actix_web::test]
async fn test_complete_user_workflow() {
    let (_temp_dir, pool) = create_test_db();
    let app = test::init_service(create_test_app_with_pool(pool.clone())).await;
    
    // Create admin user
    let (admin_email, admin_password) = create_admin_user(&pool).await;
    
    // 1. Admin logs in
    let admin_session = extract_session_from_login!(&app, &admin_email, &admin_password);
    assert!(admin_session.is_some(), "Admin should be able to log in");
    let admin_session = admin_session.unwrap();
    
    // 2. Admin creates a regular user
    let user_data = json!({
        "email": "user@example.com",
        "password": "userpass123"
    });
    
    let req = test::TestRequest::post()
        .uri("/api/users")
        .set_json(&user_data)
        .cookie(actix_web::cookie::Cookie::new("session_id", &admin_session))
        .to_request();
    
    let resp = test::call_service(&app, req).await;
    assert!(resp.status().is_success(), "Admin should be able to create users");
    
    // 3. Regular user logs in
    let user_session = extract_session_from_login!(&app, "user@example.com", "userpass123");
    assert!(user_session.is_some(), "User should be able to log in");
    let user_session = user_session.unwrap();
    
    // 4. User tries to access users list (should fail - not admin)
    let req = test::TestRequest::get()
        .uri("/api/users")
        .cookie(actix_web::cookie::Cookie::new("session_id", &user_session))
        .to_request();
    
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 403, "Regular user should not access users list");
    
    // 5. Admin can access users list
    let req = test::TestRequest::get()
        .uri("/api/users")
        .cookie(actix_web::cookie::Cookie::new("session_id", &admin_session))
        .to_request();
    
    let resp = test::call_service(&app, req).await;
    assert!(resp.status().is_success(), "Admin should access users list");
    
    // 6. User logs out
    let req = test::TestRequest::post()
        .uri("/api/auth/logout")
        .cookie(actix_web::cookie::Cookie::new("session_id", &user_session))
        .to_request();
    
    let resp = test::call_service(&app, req).await;
    assert!(resp.status().is_success(), "User should be able to log out");
    
    // 7. Try to use session after logout (should fail)
    let req = test::TestRequest::get()
        .uri("/api/subscriptions")
        .cookie(actix_web::cookie::Cookie::new("session_id", &user_session))
        .to_request();
    
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 401, "Session should be invalid after logout");
}

#[actix_web::test]
async fn test_subscription_management_workflow() {
    let (_temp_dir, pool) = create_test_db();
    let app = test::init_service(create_test_app_with_pool(pool.clone())).await;
    
    // Setup: Create admin and user
    let (admin_email, admin_password) = create_admin_user(&pool).await;
    let admin_session = extract_session_from_login!(&app, &admin_email, &admin_password).unwrap();
    
    // Create regular user
    let user_data = json!({
        "email": "user@example.com",
        "password": "userpass123"
    });
    
    let req = test::TestRequest::post()
        .uri("/api/users")
        .set_json(&user_data)
        .cookie(actix_web::cookie::Cookie::new("session_id", &admin_session))
        .to_request();
    
    test::call_service(&app, req).await;
    let user_session = extract_session_from_login!(&app, "user@example.com", "userpass123").unwrap();
    
    // 1. User creates a subscription (which automatically creates feed)
    let subscription_data = json!({
        "url": "https://example.com/rss.xml",
        "friendly_name": "My Example Feed",
        "frequency": "daily",
        "max_items": 10,
        "is_active": true
    });
    
    let req = test::TestRequest::post()
        .uri("/api/subscriptions")
        .set_json(&subscription_data)
        .cookie(actix_web::cookie::Cookie::new("session_id", &user_session))
        .to_request();
    
    let resp = test::call_service(&app, req).await;
    assert!(resp.status().is_success(), "User should be able to create subscriptions");
    
    let body = test::read_body(resp).await;
    let subscription_response: Value = serde_json::from_slice(&body).expect("Failed to parse subscription response");
    let subscription_id = subscription_response["subscription"]["id"].as_i64().expect("Subscription should have ID");
    
    // 2. User lists their subscriptions
    let req = test::TestRequest::get()
        .uri("/api/subscriptions")
        .cookie(actix_web::cookie::Cookie::new("session_id", &user_session))
        .to_request();
    
    let resp = test::call_service(&app, req).await;
    assert!(resp.status().is_success(), "User should be able to list subscriptions");
    
    let body = test::read_body(resp).await;
    let subscriptions: Value = serde_json::from_slice(&body).expect("Failed to parse subscriptions response");
    assert!(subscriptions.is_array(), "Should return array of subscriptions");
    assert_eq!(subscriptions.as_array().unwrap().len(), 1, "Should have one subscription");
    
    // 3. User updates their subscription
    let update_data = json!({
        "friendly_name": "Updated Feed Name",
        "frequency": "hourly",
        "max_items": 5,
        "is_active": false
    });
    
    let req = test::TestRequest::patch()
        .uri(&format!("/api/subscriptions/{}", subscription_id))
        .set_json(&update_data)
        .cookie(actix_web::cookie::Cookie::new("session_id", &user_session))
        .to_request();
    
    let resp = test::call_service(&app, req).await;
    assert!(resp.status().is_success(), "User should be able to update subscriptions");
    
    // Note: Individual subscription GET is not needed since list endpoint returns all data
    
    // 4. User deletes their subscription
    let req = test::TestRequest::delete()
        .uri(&format!("/api/subscriptions/{}", subscription_id))
        .cookie(actix_web::cookie::Cookie::new("session_id", &user_session))
        .to_request();
    
    let resp = test::call_service(&app, req).await;
    assert!(resp.status().is_success(), "User should be able to delete subscriptions");
    
    // 5. Verify subscription is deleted
    let req = test::TestRequest::get()
        .uri("/api/subscriptions")
        .cookie(actix_web::cookie::Cookie::new("session_id", &user_session))
        .to_request();
    
    let resp = test::call_service(&app, req).await;
    let body = test::read_body(resp).await;
    let subscriptions: Value = serde_json::from_slice(&body).expect("Failed to parse subscriptions response");
    assert_eq!(subscriptions.as_array().unwrap().len(), 0, "Should have no subscriptions after deletion");
}

#[actix_web::test]
async fn test_feed_management_workflow() {
    let (_temp_dir, pool) = create_test_db();
    let app = test::init_service(create_test_app_with_pool(pool.clone())).await;
    
    // Setup: Create admin
    let (admin_email, admin_password) = create_admin_user(&pool).await;
    let admin_session = extract_session_from_login!(&app, &admin_email, &admin_password).unwrap();
    
    // 1. First create a regular user who will subscribe to feeds
    let user_data = json!({
        "email": "feeduser@example.com",
        "password": "userpass123"
    });
    
    let req = test::TestRequest::post()
        .uri("/api/users")
        .set_json(&user_data)
        .cookie(actix_web::cookie::Cookie::new("session_id", &admin_session))
        .to_request();
    
    test::call_service(&app, req).await;
    let user_session = extract_session_from_login!(&app, "feeduser@example.com", "userpass123").unwrap();
    
    // 2. User creates subscriptions (which automatically creates feeds)
    let subscription_data = vec![
        json!({
            "url": "https://example.com/rss1.xml",
            "friendly_name": "Feed 1",
            "frequency": "daily",
            "max_items": 10
        }),
        json!({
            "url": "https://example.com/rss2.xml",
            "friendly_name": "Feed 2", 
            "frequency": "daily",
            "max_items": 10
        }),
    ];
    
    for sub_data in subscription_data {
        let req = test::TestRequest::post()
            .uri("/api/subscriptions")
            .set_json(&sub_data)
            .cookie(actix_web::cookie::Cookie::new("session_id", &user_session))
            .to_request();
        
        let resp = test::call_service(&app, req).await;
        assert!(resp.status().is_success(), "User should be able to create subscriptions");
    }
    
    // 3. Admin lists all feeds
    let req = test::TestRequest::get()
        .uri("/api/feeds")
        .cookie(actix_web::cookie::Cookie::new("session_id", &admin_session))
        .to_request();
    
    let resp = test::call_service(&app, req).await;
    assert!(resp.status().is_success(), "Admin should be able to list feeds");
    
    let body = test::read_body(resp).await;
    let feeds: Value = serde_json::from_slice(&body).expect("Failed to parse feeds response");
    assert!(feeds.is_array(), "Should return array of feeds");
    assert_eq!(feeds.as_array().unwrap().len(), 2, "Should have two feeds");
    
    // 4. Admin gets specific feed
    let feed_id = feeds[0]["id"].as_i64().expect("Feed should have ID");
    let req = test::TestRequest::get()
        .uri(&format!("/api/feeds/{}", feed_id))
        .cookie(actix_web::cookie::Cookie::new("session_id", &admin_session))
        .to_request();
    
    let resp = test::call_service(&app, req).await;
    assert!(resp.status().is_success(), "Admin should be able to get specific feed");
    
    // Feeds are managed automatically through subscriptions, so no update/delete operations
}

#[actix_web::test]
async fn test_unauthorized_access_scenarios() {
    let (_temp_dir, pool) = create_test_db();
    let app = test::init_service(create_test_app_with_pool(pool.clone())).await;
    
    // Setup: Create admin and regular user
    let (admin_email, admin_password) = create_admin_user(&pool).await;
    let admin_session = extract_session_from_login!(&app, &admin_email, &admin_password).unwrap();
    
    let user_data = json!({
        "email": "user@example.com",
        "password": "userpass123"
    });
    
    let req = test::TestRequest::post()
        .uri("/api/users")
        .set_json(&user_data)
        .cookie(actix_web::cookie::Cookie::new("session_id", &admin_session))
        .to_request();
    
    test::call_service(&app, req).await;
    let user_session = extract_session_from_login!(&app, "user@example.com", "userpass123").unwrap();
    
    // 1. Regular user tries to access admin-only endpoints
    let admin_endpoints = vec![
        ("GET", "/api/users"),
        ("GET", "/api/feeds"),
    ];
    
    for (method, endpoint) in admin_endpoints {
        let req = match method {
            "GET" => test::TestRequest::get(),
            "POST" => test::TestRequest::post(),
            "PUT" => test::TestRequest::put(),
            "DELETE" => test::TestRequest::delete(),
            _ => test::TestRequest::get(),
        }
        .uri(endpoint)
        .cookie(actix_web::cookie::Cookie::new("session_id", &user_session))
        .to_request();
        
        let resp = test::call_service(&app, req).await;
        assert!(
            resp.status() == 403 || resp.status() == 401,
            "Regular user should not access admin endpoint: {} {}",
            method,
            endpoint
        );
    }
    
    // 2. Try to access endpoints with invalid session
    let req = test::TestRequest::get()
        .uri("/api/subscriptions")
        .cookie(actix_web::cookie::Cookie::new("session_id", "invalid_session_id"))
        .to_request();
    
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 401, "Invalid session should be rejected");
    
    // 3. Try to access endpoints without session
    let req = test::TestRequest::get()
        .uri("/api/subscriptions")
        .to_request();
    
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 401, "Missing session should be rejected");
}