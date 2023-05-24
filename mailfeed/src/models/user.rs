use crate::schema::*;
use argon2::{
    password_hash::{rand_core::OsRng, PasswordHash, PasswordHasher, PasswordVerifier, SaltString},
    Argon2,
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
    pub roles: String,           // CSV
}

#[derive(Debug, Serialize, Deserialize)]
pub struct NewUser {
    pub email: String,
    pub password: String,
}

pub enum UserQuery {
    Id(i32),
    Email(String),
}

impl User {
    pub fn create<'a>(conn: &mut SqliteConnection, new_user: &'a NewUser) -> QueryResult<usize> {
        use crate::schema::users::dsl::*;
        let user_exists = users
            .filter(login_email.eq(&new_user.email))
            .first::<User>(conn)
            .is_ok();

        if user_exists {
            return Err(diesel::result::Error::RollbackTransaction);
        }

        let password_hash = Self::hash_password(&new_user.password)?;

        let user = User {
            id: None,
            login_email: new_user.email.clone(),
            send_email: new_user.email.clone(),
            password: password_hash,
            created_at: chrono::Utc::now().timestamp() as i32,
            is_active: true,
            daily_send_time: "00:00+00:00".into(),
            roles: "user".into(),
        };

        diesel::insert_into(users::table())
            .values(&user)
            .execute(conn)
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
        match query {
            UserQuery::Id(user_id) => users.filter(id.eq(user_id)).first::<User>(conn).ok(),
            UserQuery::Email(email) => users.filter(login_email.eq(email)).first::<User>(conn).ok(),
        }
    }

    pub fn update(conn: &mut SqliteConnection, user: &User) -> QueryResult<usize> {
        use crate::schema::users::dsl::*;

        let password_hash = Self::hash_password(&user.password)?;

        let user = User {
            id: user.id,
            login_email: user.login_email.clone(),
            send_email: user.send_email.clone(),
            password: password_hash,
            created_at: user.created_at,
            is_active: user.is_active,
            daily_send_time: user.daily_send_time.clone(),
            roles: user.roles.clone(),
        };

        diesel::update(users.filter(id.eq(user.id.unwrap())))
            .set(&user)
            .execute(conn)
    }

    pub fn delete(conn: &mut SqliteConnection, user_id: i32) -> QueryResult<usize> {
        use crate::schema::users::dsl::*;
        diesel::delete(users.filter(id.eq(user_id))).execute(conn)
    }

    fn hash_password(password: &str) -> Result<String, diesel::result::Error> {
        if password.len() < 1 {
            return Err(diesel::result::Error::RollbackTransaction);
        }
        let salt = SaltString::generate(&mut OsRng);
        let argon2 = Argon2::default();
        argon2
            .hash_password(password.as_bytes(), &salt)
            .map(|hash| hash.to_string())
            .map_err(|_| diesel::result::Error::RollbackTransaction)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_helpers::test_helpers::get_test_db_connection;
    use diesel::result::Error;

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
        assert_eq!(result.unwrap_err(), Error::RollbackTransaction);

        let user = User::get(&mut conn, UserQuery::Email(new_user.email.clone())).unwrap();
        assert_eq!(user.login_email, new_user.email);
        assert_eq!(user.send_email, new_user.email);
        assert_ne!(user.password, new_user.password);
        assert_eq!(user.is_active, true);
        assert_eq!(user.roles, "user");
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

        let user = User::get(&mut conn, UserQuery::Email(new_user.email.clone())).unwrap();
        assert_eq!(user.login_email, new_user.email);
        assert_eq!(user.send_email, new_user.email);
        assert_ne!(user.password, new_user.password);
        assert_eq!(user.is_active, true);
        assert_eq!(user.roles, "user");

        let user = User {
            id: user.id,
            login_email: "myNewEmail@ok.yup".into(),
            send_email: "test@me.com".into(),
            password: "password".into(),
            created_at: user.created_at,
            is_active: user.is_active,
            daily_send_time: user.daily_send_time,
            roles: user.roles,
        };

        let result = User::update(&mut conn, &user);
        assert!(result.is_ok());

        let user = User::get(&mut conn, UserQuery::Email(user.login_email.clone())).unwrap();
        assert_eq!(user.login_email, "myNewEmail@ok.yup");
        assert_eq!(user.send_email, "test@me.com");
        assert_ne!(user.password, "password");
        assert_eq!(user.is_active, true);
        assert_eq!(user.roles, "user");
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
