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
async fn test_basic_auth_workflow() {
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
async fn test_feed_validation_endpoint() {
    let (_temp_dir, pool) = create_test_db();
    let app = test::init_service(create_test_app_with_pool(pool.clone())).await;
    
    // Create admin user and get session
    let (admin_email, admin_password) = create_admin_user(&pool).await;
    let admin_session = extract_session_from_login!(&app, &admin_email, &admin_password).unwrap();
    
    // Test feed validation with empty URL
    let validation_data = json!({
        "url": ""
    });
    
    let req = test::TestRequest::post()
        .uri("/api/feeds/validate")
        .set_json(&validation_data)
        .cookie(actix_web::cookie::Cookie::new("session_id", &admin_session))
        .to_request();
    
    let resp = test::call_service(&app, req).await;
    assert!(resp.status().is_success(), "Feed validation should return response");
    
    let body = test::read_body(resp).await;
    let validation_response: Value = serde_json::from_slice(&body).expect("Failed to parse validation response");
    assert_eq!(validation_response["valid"], false, "Empty URL should be invalid");
    assert!(validation_response["error"].is_string(), "Should include error message");
}

#[actix_web::test]
async fn test_subscription_endpoints_basic() {
    let (_temp_dir, pool) = create_test_db();
    let app = test::init_service(create_test_app_with_pool(pool.clone())).await;
    
    // Setup: Create admin and regular user
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
    
    // Test that user can access subscriptions endpoint (should return empty list initially)
    let req = test::TestRequest::get()
        .uri("/api/subscriptions")
        .cookie(actix_web::cookie::Cookie::new("session_id", &user_session))
        .to_request();
    
    let resp = test::call_service(&app, req).await;
    assert!(resp.status().is_success(), "User should be able to access subscriptions");
    
    let body = test::read_body(resp).await;
    let subscriptions: Value = serde_json::from_slice(&body).expect("Failed to parse subscriptions response");
    assert!(subscriptions.is_array(), "Should return array of subscriptions");
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