use crate::schema::*;
use argon2::{
    password_hash::{rand_core::OsRng, PasswordHasher, SaltString},
    Argon2, PasswordHash, PasswordVerifier,
};
use diesel::{associations::HasTable, prelude::*};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Queryable, Insertable, Identifiable, AsChangeset)]
#[diesel(table_name = users)]
pub struct User {
    pub id: Option<i32>,
    pub login_email: String,
    pub send_email: String,
    #[serde(skip_serializing)]
    pub password: String,
    pub created_at: i32,
    pub is_active: bool,
    pub daily_send_time: String, // HH:MM+HH:MM
    pub role: String,            // CSV
    pub refresh_token: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, AsChangeset)]
#[diesel(table_name = users)]
pub struct PartialUser {
    pub login_email: Option<String>,
    pub send_email: Option<String>,
    pub is_active: Option<bool>,
    pub daily_send_time: Option<String>, // HH:MM+HH:MM
    pub role: Option<String>,            // CSV
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

impl Default for PartialUser {
    fn default() -> Self {
        Self {
            login_email: None,
            send_email: None,
            is_active: None,
            daily_send_time: None,
            role: None,
            refresh_token: None,
        }
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
}

#[derive(Debug)]
pub enum UserQuery {
    Id(i32),
    Email(String),
}

impl User {
    pub fn create<'a>(
        conn: &mut SqliteConnection,
        new_user: &'a NewUser,
    ) -> Result<User, UserTableError> {
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

        let user = User {
            id: None,
            login_email: new_user.email.clone(),
            send_email: new_user.email.clone(),
            password: password_hash,
            created_at: chrono::Utc::now().timestamp() as i32,
            is_active: true,
            daily_send_time: "00:00+00:00".into(),
            role: "user".into(),
            refresh_token: None,
        };

        // TODO: use .get_result() here
        match diesel::insert_into(users::table())
            .values(&user)
            .execute(conn)
        {
            Ok(_) => Ok(user),
            Err(err) => {
                log::error!("Failed to insert user into database: {:?}", err);
                Err(UserTableError::DatabaseError)
            }
        }?;

        let user_in_db = match users
            .filter(login_email.eq(&new_user.email))
            .first::<User>(conn)
        {
            Ok(user) => user,
            Err(err) => {
                log::error!("Failed to get user from database: {:?}", err);
                return Err(UserTableError::DatabaseError);
            }
        };

        Ok(user_in_db)
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
        log::info!("Updating user (id={:?}): {:?}", user_id, updates);

        match diesel::update(users.filter(id.eq(user_id)))
            .set(updates)
            .get_result::<User>(conn)
        {
            Ok(user) => Ok(user),
            Err(err) => {
                log::error!("Failed to update user: {:?}", err);
                return Err(UserTableError::DatabaseError);
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

    pub fn delete(conn: &mut SqliteConnection, user_id: i32) -> Result<(), UserTableError> {
        use crate::schema::users::dsl::*;
        log::info!("Deleting user (id={})", user_id);

        let deleted_rows = diesel::delete(users.filter(id.eq(user_id)))
            .execute(conn)
            .map_err(|err| {
                log::error!("Failed to delete user: {:?}", err);
                UserTableError::DatabaseError
            })
            .ok();

        if deleted_rows == Some(0) {
            log::warn!("User with id {} does not exist", user_id);
            return Err(UserTableError::UserNotFound);
        } else {
            return Ok(());
        }
    }

    fn hash_password(password: &str) -> Result<String, UserTableError> {
        if password.len() < 1 {
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
    use super::*;
    use crate::test_helpers::test_helpers::get_test_db_connection;

    #[test]
    fn test_create_user() {
        let mut conn = get_test_db_connection();
        let new_user = NewUser {
            email: "test@me.com".into(),
            password: "password".into(),
        };

        let result = User::create(&mut conn, &new_user);
        if let Err(e) = result {
            panic!("Failed to create user: {:?}", e);
        }

        assert!(result.is_ok());

        let result = User::create(&mut conn, &new_user);
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), UserTableError::EmailExists));

        let user = User::get(&mut conn, UserQuery::Email(new_user.email.clone())).unwrap();
        assert_eq!(user.login_email, new_user.email);
        assert_eq!(user.send_email, new_user.email);
        assert_ne!(user.password, new_user.password);
        assert_eq!(user.is_active, true);
        assert_eq!(user.role, "user");
    }

    #[test]
    fn test_password_required() {
        let mut conn = get_test_db_connection();
        let new_user = NewUser {
            email: "test@me.com".into(),
            password: "".into(),
        };

        let result = User::create(&mut conn, &new_user);
        assert!(result.is_err());

        let user = User::get(&mut conn, UserQuery::Email(new_user.email.clone()));
        assert!(user.is_none());
    }

    #[test]
    fn test_can_update_user() {
        let mut conn = get_test_db_connection();
        let new_user = NewUser {
            email: "test@me.com".into(),
            password: "password".into(),
        };

        let result = User::create(&mut conn, &new_user);
        assert!(result.is_ok());

        let existing_user = User::get(&mut conn, UserQuery::Email(new_user.email.clone())).unwrap();
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

        let result = User::update(&mut conn, existing_user.id.unwrap(), &user);
        assert!(result.is_ok());

        let user = User::get(
            &mut conn,
            UserQuery::Email(user.login_email.unwrap().clone()),
        )
        .unwrap();
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

        let result = User::create(&mut conn, &new_user);
        assert!(result.is_ok());

        let user = User::get(&mut conn, UserQuery::Email(new_user.email.clone())).unwrap();
        assert_eq!(user.login_email, new_user.email);

        let result = User::delete(&mut conn, user.id.unwrap());
        assert!(result.is_ok());
    }
}
