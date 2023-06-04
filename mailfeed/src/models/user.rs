use crate::{claims::Claims, schema::*};
use argon2::{
    password_hash::{rand_core::OsRng, PasswordHasher, SaltString},
    Argon2, PasswordHash, PasswordVerifier,
};
use diesel::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Queryable, Identifiable, AsChangeset)]
#[diesel(table_name = users)]
pub struct User {
    pub id: i32,
    pub login_email: String,
    pub send_email: String,
    #[serde(skip_serializing)]
    pub password: String,
    pub created_at: i32,
    pub is_active: bool,
    pub daily_send_time: String, // HH:MM+HH:MM
    pub role: String,            // CSV
    #[serde(skip_serializing)]
    pub refresh_token: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Insertable, AsChangeset)]
#[diesel(table_name = users)]
pub struct InsertableUser {
    pub login_email: String,
    pub send_email: String,
    #[serde(skip_serializing)]
    pub password: String,
    pub created_at: i32,
    pub is_active: bool,
    pub daily_send_time: String, // HH:MM+HH:MM
    pub role: String,            // CSV
    #[serde(skip_serializing)]
    pub refresh_token: Option<String>,
}

#[derive(Debug, Default, Serialize, Deserialize, AsChangeset)]
#[diesel(table_name = users)]
pub struct PartialUser {
    pub login_email: Option<String>,
    pub send_email: Option<String>,
    pub is_active: Option<bool>,
    pub daily_send_time: Option<String>, // HH:MM+HH:MM
    pub role: Option<String>,
    #[serde(skip_deserializing)]
    pub refresh_token: Option<String>,
}

impl PartialUser {
    pub fn is_empty(&self) -> bool {
        self.login_email.is_none()
            && self.send_email.is_none()
            && self.is_active.is_none()
            && self.daily_send_time.is_none()
            && self.role.is_none()
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct NewUser {
    pub email: String,
    pub password: String,
}

#[derive(Debug, Deserialize)]
pub enum UserTableError {
    UserNotFound,
    EmailExists,
    PasswordHashError,
    PasswordTooShort,
    DatabaseError,
    Unauthorized,
}

#[derive(Debug)]
pub enum UserQuery<'a> {
    Id(i32),
    Email(&'a str),
}

impl User {
    // TODO: refactor the way the models for feed_items and feeds are
    pub fn create(
        conn: &mut SqliteConnection,
        new_user: &NewUser,
        claims: Claims,
    ) -> Result<User, UserTableError> {
        if &claims.role != "admin" {
            log::warn!("User {} is not an admin", claims.sub);
            return Err(UserTableError::UserNotFound);
        }
        use crate::schema::users::dsl::*;
        let user_exists = users
            .filter(login_email.eq(&new_user.email))
            .first::<User>(conn)
            .is_ok();

        if user_exists {
            log::warn!("User with email {} already exists", new_user.email);
            return Err(UserTableError::EmailExists);
        }

        let password_hash = match Self::hash_password(&new_user.password) {
            Ok(hash) => hash,
            Err(UserTableError::PasswordTooShort) => {
                log::warn!("Password too short");
                return Err(UserTableError::PasswordTooShort);
            }
            Err(_) => {
                log::error!("Failed to hash password");
                return Err(UserTableError::PasswordHashError);
            }
        };

        let user = InsertableUser {
            login_email: new_user.email.clone(),
            send_email: new_user.email.clone(),
            password: password_hash,
            created_at: chrono::Utc::now().timestamp() as i32,
            is_active: true,
            daily_send_time: "00:00+00:00".into(),
            role: "user".into(),
            refresh_token: None,
        };

        match diesel::insert_into(users).values(&user).get_result(conn) {
            Ok(in_db) => Ok(in_db),
            Err(err) => {
                log::error!("Failed to insert user into database: {:?}", err);
                Err(UserTableError::DatabaseError)
            }
        }
    }

    pub fn exists(conn: &mut SqliteConnection, email: &str) -> bool {
        use crate::schema::users::dsl::*;
        users
            .filter(login_email.eq(email))
            .first::<User>(conn)
            .is_ok()
    }

    pub fn get(conn: &mut SqliteConnection, query: UserQuery) -> Option<User> {
        use crate::schema::users::dsl::*;
        log::info!("Getting user: {:?}", query);
        match query {
            UserQuery::Id(user_id) => users.filter(id.eq(user_id)).first::<User>(conn).ok(),
            UserQuery::Email(email) => users.filter(login_email.eq(email)).first::<User>(conn).ok(),
        }
    }

    pub fn get_all(conn: &mut SqliteConnection) -> Result<Vec<User>, UserTableError> {
        use crate::schema::users::dsl::*;
        log::info!("Getting all users");
        users.load::<User>(conn).map_err(|err| {
            log::error!("Failed to get users: {:?}", err);
            UserTableError::DatabaseError
        })
    }

    pub fn get_all_admin(conn: &mut SqliteConnection) -> Result<Vec<User>, UserTableError> {
        use crate::schema::users::dsl::*;
        log::info!("Getting all admins");
        users
            .filter(role.eq("admin"))
            .load::<User>(conn)
            .map_err(|err| {
                log::error!("Failed to get admins: {:?}", err);
                UserTableError::DatabaseError
            })
    }

    pub fn update(
        conn: &mut SqliteConnection,
        user_id: i32,
        updates: &PartialUser,
    ) -> Result<User, UserTableError> {
        use crate::schema::users::dsl::*;

        if let Some(update_email) = &updates.login_email {
            let user_exists = User::exists(conn, update_email);
            if user_exists {
                log::warn!("User with email {} already exists", update_email);
                return Err(UserTableError::EmailExists);
            }
        }
        log::info!("Updating user (id={:?})", user_id);

        match diesel::update(users.filter(id.eq(user_id)))
            .set(updates)
            .get_result::<User>(conn)
        {
            Ok(user) => Ok(user),
            Err(err) => {
                log::error!("Failed to update user: {:?}", err);
                Err(UserTableError::DatabaseError)
            }
        }
    }

    pub fn clear_refresh_token(
        conn: &mut SqliteConnection,
        user_id: UserQuery,
    ) -> Result<(), UserTableError> {
        use crate::schema::users::dsl::*;

        log::info!("Clearing refresh token for user: {:?}", user_id);

        let res = match user_id {
            UserQuery::Id(user_id) => diesel::update(users.filter(id.eq(user_id)))
                .set(refresh_token.eq(None::<String>))
                .execute(conn),
            UserQuery::Email(email) => diesel::update(users.filter(login_email.eq(email)))
                .set(refresh_token.eq(None::<String>))
                .execute(conn),
        };

        match res {
            Ok(_) => Ok(()),
            Err(err) => {
                log::error!("Failed to clear refresh token: {:?}", err);
                Err(UserTableError::DatabaseError)
            }
        }
    }

    pub fn delete(
        conn: &mut SqliteConnection,
        user_id: i32,
        claims: Claims,
    ) -> Result<(), UserTableError> {
        use crate::schema::users::dsl::*;
        log::info!("Deleting user (id={})", user_id);

        if claims.role != "admin" && claims.sub != user_id {
            log::warn!(
                "User {} is not authorized to delete user {}",
                claims.sub,
                user_id
            );
            return Err(UserTableError::Unauthorized);
        }

        let deleted_rows = diesel::delete(users.filter(id.eq(user_id)))
            .execute(conn)
            .map_err(|err| {
                log::error!("Failed to delete user: {:?}", err);
                UserTableError::DatabaseError
            })
            .ok();

        if deleted_rows == Some(0) {
            log::warn!("User with id {} does not exist", user_id);
            Err(UserTableError::UserNotFound)
        } else {
            Ok(())
        }
    }

    fn hash_password(password: &str) -> Result<String, UserTableError> {
        if password.is_empty() {
            return Err(UserTableError::PasswordTooShort);
        }
        let salt = SaltString::generate(&mut OsRng);
        let argon2 = Argon2::default();
        argon2
            .hash_password(password.as_bytes(), &salt)
            .map(|hash| hash.to_string())
            .map_err(|_| UserTableError::PasswordHashError)
    }

    pub fn check_password(user: &User, password: &str) -> Result<bool, UserTableError> {
        let argon2 = Argon2::default();
        let password_hash = PasswordHash::new(&user.password).map_err(|_| {
            log::error!("Failed to parse password hash");
            UserTableError::PasswordHashError
        })?;
        let result = argon2
            .verify_password(password.as_bytes(), &password_hash)
            .map_err(|_| {
                log::error!("Failed to verify password");
                UserTableError::PasswordHashError
            })
            .is_ok();
        Ok(result)
    }
}

#[cfg(test)]
mod tests {
    use chrono::Utc;

    use super::*;
    use crate::test_helpers::test_helpers::get_test_db_connection;

    #[test]
    fn test_create_user() {
        let mut conn = get_test_db_connection();
        let new_user = NewUser {
            email: "test@me.com".into(),
            password: "password".into(),
        };

        let claims = Claims {
            sub: 0,
            email: new_user.email.clone(),
            role: "admin".into(),
            exp: (Utc::now().timestamp() + 1000) as usize,
        };

        let result = User::create(&mut conn, &new_user, claims.clone());
        if let Err(e) = result {
            panic!("Failed to create user: {:?}", e);
        }

        assert!(result.is_ok());

        let result = User::create(&mut conn, &new_user, claims);
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), UserTableError::EmailExists));

        let user = User::get(&mut conn, UserQuery::Email(&new_user.email)).unwrap();
        assert_eq!(user.login_email, new_user.email);
        assert_eq!(user.send_email, new_user.email);
        assert_ne!(user.password, new_user.password);
        assert_eq!(user.is_active, true);
        assert_eq!(user.role, "user");
    }

    #[test]
    fn test_non_admin_cannot_create() {
        let mut conn = get_test_db_connection();
        let new_user = NewUser {
            email: "test@me.com".into(),
            password: "password".into(),
        };

        let claims = Claims {
            sub: 0,
            email: new_user.email.clone(),
            role: "user".into(),
            exp: (Utc::now().timestamp() + 1000) as usize,
        };

        let result = User::create(&mut conn, &new_user, claims);
        assert!(result.is_err());
    }

    #[test]
    fn test_password_required() {
        let mut conn = get_test_db_connection();
        let new_user = NewUser {
            email: "test@me.com".into(),
            password: "".into(),
        };

        let claims = Claims {
            sub: 0,
            email: new_user.email.clone(),
            role: "admin".into(),
            exp: (Utc::now().timestamp() + 1000) as usize,
        };

        let result = User::create(&mut conn, &new_user, claims);
        assert!(result.is_err());

        let user = User::get(&mut conn, UserQuery::Email(&new_user.email));
        assert!(user.is_none());
    }

    #[test]
    fn test_can_update_user() {
        let mut conn = get_test_db_connection();
        let new_user = NewUser {
            email: "test@me.com".into(),
            password: "password".into(),
        };

        let claims = Claims {
            sub: 0,
            email: new_user.email.clone(),
            role: "admin".into(),
            exp: (Utc::now().timestamp() + 1000) as usize,
        };

        let result = User::create(&mut conn, &new_user, claims);
        assert!(result.is_ok());

        let existing_user = User::get(&mut conn, UserQuery::Email(&new_user.email)).unwrap();
        assert_eq!(existing_user.login_email, new_user.email);
        assert_eq!(existing_user.send_email, new_user.email);
        assert_ne!(existing_user.password, new_user.password);
        assert_eq!(existing_user.is_active, true);
        assert_eq!(existing_user.role, "user");

        let user = PartialUser {
            login_email: Some("myNewEmail@ok.yup".into()),
            send_email: Some("test@me.com".into()),
            is_active: Some(true),
            role: None,
            daily_send_time: None,
            refresh_token: Some("some refresh token".into()),
        };

        let result = User::update(&mut conn, existing_user.id, &user);
        assert!(result.is_ok());

        let user = User::get(&mut conn, UserQuery::Email(&user.login_email.unwrap())).unwrap();
        assert_eq!(user.login_email, "myNewEmail@ok.yup");
        assert_eq!(user.send_email, "test@me.com");
        assert_ne!(user.password, "password");
        assert_eq!(user.is_active, true);
        assert_eq!(user.role, "user");
    }

    #[test]
    fn test_delete_user() {
        let mut conn = get_test_db_connection();
        let new_user = NewUser {
            email: "me@test.com".into(),
            password: "password".into(),
        };

        let claims = Claims {
            sub: 0,
            email: new_user.email.clone(),
            role: "admin".into(),
            exp: (Utc::now().timestamp() + 1000) as usize,
        };

        let result = User::create(&mut conn, &new_user, claims.clone());
        assert!(result.is_ok());

        let user = User::get(&mut conn, UserQuery::Email(&new_user.email)).unwrap();
        assert_eq!(user.login_email, new_user.email);

        let result = User::delete(&mut conn, user.id, claims);
        assert!(result.is_ok());
    }

    #[test]
    fn test_non_admin_cannot_delete() {
        let mut conn = get_test_db_connection();
        let new_user = NewUser {
            email: "me@test.com".into(),
            password: "password".into(),
        };

        let claims = Claims {
            sub: 0,
            email: "admin".into(),
            role: "admin".into(),
            exp: (Utc::now().timestamp() + 1000) as usize,
        };

        let result = User::create(&mut conn, &new_user, claims);
        assert!(result.is_ok());

        let user = User::get(&mut conn, UserQuery::Email(&new_user.email)).unwrap();
        assert_eq!(user.login_email, new_user.email);

        let claims = Claims {
            sub: 0,
            email: new_user.email.clone(),
            role: "user".into(),
            exp: (Utc::now().timestamp() + 1000) as usize,
        };

        let result = User::delete(&mut conn, user.id, claims);
        assert!(result.is_err());

        // should be able to delete self
        let claims = Claims {
            sub: user.id,
            email: new_user.email.clone(),
            role: "user".into(),
            exp: (Utc::now().timestamp() + 1000) as usize,
        };

        let result = User::delete(&mut conn, user.id, claims);
        assert!(result.is_ok());
    }
}
