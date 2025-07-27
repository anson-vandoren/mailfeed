use mailfeed::{
    models::user::{NewUser, User, UserTableError},
    session::SessionClaims,
};
use diesel::{Connection, sqlite::SqliteConnection};
use diesel_migrations::{embed_migrations, EmbeddedMigrations, MigrationHarness};

const MIGRATIONS: EmbeddedMigrations = embed_migrations!("src/migrations");

fn get_test_db_connection() -> SqliteConnection {
    let database_url = ":memory:";
    let mut conn = SqliteConnection::establish(database_url)
        .expect("Failed to create in-memory database");
    
    // Run migrations
    conn.run_pending_migrations(MIGRATIONS)
        .expect("Failed to run migrations");

    conn
}

// Note: hash_password is private, so we test it indirectly through user creation

#[test]
fn test_password_verification_success() {
    let mut conn = get_test_db_connection();
    let password = "correct_password";
    
    // Create a user to get a hashed password
    let new_user = NewUser {
        email: "test@example.com".to_string(),
        password: password.to_string(),
    };
    
    let admin_claims = SessionClaims {
        sub: 1,
        email: "admin@example.com".to_string(),
        role: "admin".to_string(),
    };
    
    let user = User::create(&mut conn, &new_user, admin_claims).expect("Failed to create user");
    
    let verification_result = User::check_password(&user, password);
    assert!(verification_result.is_ok());
    assert!(verification_result.unwrap());
}

#[test]
fn test_password_verification_failure() {
    let mut conn = get_test_db_connection();
    let correct_password = "correct_password";
    let wrong_password = "wrong_password";
    
    // Create a user with the correct password
    let new_user = NewUser {
        email: "test2@example.com".to_string(),
        password: correct_password.to_string(),
    };
    
    let admin_claims = SessionClaims {
        sub: 1,
        email: "admin@example.com".to_string(),
        role: "admin".to_string(),
    };
    
    let user = User::create(&mut conn, &new_user, admin_claims).expect("Failed to create user");
    
    let verification_result = User::check_password(&user, wrong_password);
    assert!(verification_result.is_ok());
    assert!(!verification_result.unwrap()); // Should return false for wrong password
}

#[test]
fn test_empty_password_rejected() {
    let mut conn = get_test_db_connection();
    
    let new_user = NewUser {
        email: "test3@example.com".to_string(),
        password: "".to_string(), // Empty password
    };
    
    let admin_claims = SessionClaims {
        sub: 1,
        email: "admin@example.com".to_string(),
        role: "admin".to_string(),
    };
    
    let result = User::create(&mut conn, &new_user, admin_claims);
    assert!(result.is_err());
    assert!(matches!(result.unwrap_err(), UserTableError::PasswordTooShort));
}

#[test]
fn test_user_creation_requires_admin() {
    let mut conn = get_test_db_connection();
    
    let new_user = NewUser {
        email: "test@example.com".to_string(),
        password: "password123".to_string(),
    };
    
    // Non-admin claims
    let user_claims = SessionClaims {
        sub: 1,
        email: "user@example.com".to_string(),
        role: "user".to_string(),
    };
    
    let result = User::create(&mut conn, &new_user, user_claims);
    assert!(result.is_err());
    assert!(matches!(result.unwrap_err(), UserTableError::UserNotFound)); // Non-admin gets UserNotFound
}

#[test]
fn test_admin_can_create_user() {
    let mut conn = get_test_db_connection();
    
    let new_user = NewUser {
        email: "newuser@example.com".to_string(),
        password: "password123".to_string(),
    };
    
    // Admin claims
    let admin_claims = SessionClaims {
        sub: 1,
        email: "admin@example.com".to_string(),
        role: "admin".to_string(),
    };
    
    let result = User::create(&mut conn, &new_user, admin_claims);
    assert!(result.is_ok());
    
    let created_user = result.unwrap();
    assert_eq!(created_user.login_email, new_user.email);
    assert_eq!(created_user.send_email, new_user.email);
    assert_ne!(created_user.password, new_user.password); // Password should be hashed
    assert!(created_user.is_active);
    assert_eq!(created_user.role, "user"); // New users get 'user' role by default
}

#[test]
fn test_duplicate_email_rejected() {
    let mut conn = get_test_db_connection();
    
    let email = "duplicate@example.com";
    let new_user = NewUser {
        email: email.to_string(),
        password: "password123".to_string(),
    };
    
    let admin_claims = SessionClaims {
        sub: 1,
        email: "admin@example.com".to_string(),
        role: "admin".to_string(),
    };
    
    // Create first user
    let result1 = User::create(&mut conn, &new_user, admin_claims.clone());
    assert!(result1.is_ok());
    
    // Try to create second user with same email
    let result2 = User::create(&mut conn, &new_user, admin_claims);
    assert!(result2.is_err());
    assert!(matches!(result2.unwrap_err(), UserTableError::EmailExists));
}

#[test]
fn test_user_exists_check() {
    let mut conn = get_test_db_connection();
    
    let email = "exists@example.com";
    
    // Check non-existent user
    assert!(!User::exists(&mut conn, email));
    
    // Create user
    let new_user = NewUser {
        email: email.to_string(),
        password: "password123".to_string(),
    };
    
    let admin_claims = SessionClaims {
        sub: 1,
        email: "admin@example.com".to_string(),
        role: "admin".to_string(),
    };
    
    User::create(&mut conn, &new_user, admin_claims).expect("Failed to create user");
    
    // Check existing user
    assert!(User::exists(&mut conn, email));
}

#[test]
fn test_get_user_by_email() {
    let mut conn = get_test_db_connection();
    
    let email = "findme@example.com";
    let new_user = NewUser {
        email: email.to_string(),
        password: "password123".to_string(),
    };
    
    let admin_claims = SessionClaims {
        sub: 1,
        email: "admin@example.com".to_string(),
        role: "admin".to_string(),
    };
    
    let created_user = User::create(&mut conn, &new_user, admin_claims).expect("Failed to create user");
    
    // Find user by email
    use mailfeed::models::user::UserQuery;
    let found_user = User::get(&mut conn, UserQuery::Email(email));
    
    assert!(found_user.is_some());
    let found_user = found_user.unwrap();
    assert_eq!(found_user.id, created_user.id);
    assert_eq!(found_user.login_email, email);
}

#[test]
fn test_get_user_by_id() {
    let mut conn = get_test_db_connection();
    
    let new_user = NewUser {
        email: "findbyid@example.com".to_string(),
        password: "password123".to_string(),
    };
    
    let admin_claims = SessionClaims {
        sub: 1,
        email: "admin@example.com".to_string(),
        role: "admin".to_string(),
    };
    
    let created_user = User::create(&mut conn, &new_user, admin_claims).expect("Failed to create user");
    
    // Find user by ID
    use mailfeed::models::user::UserQuery;
    let found_user = User::get(&mut conn, UserQuery::Id(created_user.id));
    
    assert!(found_user.is_some());
    let found_user = found_user.unwrap();
    assert_eq!(found_user.id, created_user.id);
    assert_eq!(found_user.login_email, new_user.email);
}

#[test]
fn test_get_nonexistent_user() {
    let mut conn = get_test_db_connection();
    
    use mailfeed::models::user::UserQuery;
    
    // Try to get user that doesn't exist
    let result_by_email = User::get(&mut conn, UserQuery::Email("nonexistent@example.com"));
    assert!(result_by_email.is_none());
    
    let result_by_id = User::get(&mut conn, UserQuery::Id(99999));
    assert!(result_by_id.is_none());
}

#[test]
fn test_user_deletion_authorization() {
    let mut conn = get_test_db_connection();
    
    // Create a user
    let new_user = NewUser {
        email: "todelete@example.com".to_string(),
        password: "password123".to_string(),
    };
    
    let admin_claims = SessionClaims {
        sub: 1,
        email: "admin@example.com".to_string(),
        role: "admin".to_string(),
    };
    
    let created_user = User::create(&mut conn, &new_user, admin_claims.clone()).expect("Failed to create user");
    
    // Admin should be able to delete any user
    let result = User::delete(&mut conn, created_user.id, admin_claims);
    assert!(result.is_ok());
}

#[test]
fn test_user_can_delete_self() {
    let mut conn = get_test_db_connection();
    
    // Create a user
    let new_user = NewUser {
        email: "selfdelete@example.com".to_string(),
        password: "password123".to_string(),
    };
    
    let admin_claims = SessionClaims {
        sub: 1,
        email: "admin@example.com".to_string(),
        role: "admin".to_string(),
    };
    
    let created_user = User::create(&mut conn, &new_user, admin_claims).expect("Failed to create user");
    
    // User should be able to delete themselves
    let self_claims = SessionClaims {
        sub: created_user.id,
        email: created_user.login_email.clone(),
        role: "user".to_string(),
    };
    
    let result = User::delete(&mut conn, created_user.id, self_claims);
    assert!(result.is_ok());
}

#[test]
fn test_user_cannot_delete_others() {
    let mut conn = get_test_db_connection();
    
    // Create two users
    let admin_claims = SessionClaims {
        sub: 1,
        email: "admin@example.com".to_string(),
        role: "admin".to_string(),
    };
    
    let user1 = NewUser {
        email: "user1@example.com".to_string(),
        password: "password123".to_string(),
    };
    let user2 = NewUser {
        email: "user2@example.com".to_string(),
        password: "password123".to_string(),
    };
    
    let created_user1 = User::create(&mut conn, &user1, admin_claims.clone()).expect("Failed to create user1");
    let created_user2 = User::create(&mut conn, &user2, admin_claims).expect("Failed to create user2");
    
    // User1 tries to delete User2
    let user1_claims = SessionClaims {
        sub: created_user1.id,
        email: created_user1.login_email.clone(),
        role: "user".to_string(),
    };
    
    let result = User::delete(&mut conn, created_user2.id, user1_claims);
    assert!(result.is_err());
    assert!(matches!(result.unwrap_err(), UserTableError::Unauthorized));
}

#[test]
fn test_clear_refresh_token() {
    let mut conn = get_test_db_connection();
    
    // Create a user
    let new_user = NewUser {
        email: "refreshtoken@example.com".to_string(),
        password: "password123".to_string(),
    };
    
    let admin_claims = SessionClaims {
        sub: 1,
        email: "admin@example.com".to_string(),
        role: "admin".to_string(),
    };
    
    let created_user = User::create(&mut conn, &new_user, admin_claims).expect("Failed to create user");
    
    use mailfeed::models::user::UserQuery;
    
    // Clear refresh token by ID
    let result = User::clear_refresh_token(&mut conn, UserQuery::Id(created_user.id));
    assert!(result.is_ok());
    
    // Clear refresh token by email
    let result = User::clear_refresh_token(&mut conn, UserQuery::Email(&created_user.login_email));
    assert!(result.is_ok());
}