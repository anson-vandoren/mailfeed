use crate::schema::*;
use argon2::{
    password_hash::{rand_core::OsRng, PasswordHash, PasswordHasher, PasswordVerifier, SaltString},
    Argon2,
};
use diesel::{associations::HasTable, prelude::*};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Queryable, Insertable)]
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
    pub fn create<'a>(
        conn: &mut SqliteConnection,
        new_user: &'a NewUser,
    ) -> QueryResult<usize> {
        use crate::schema::users::dsl::*;
        // check no user with this email exists
        let user_exists = users
            .filter(login_email.eq(&new_user.email))
            .first::<User>(conn)
            .is_ok();

        if user_exists {
            return Err(diesel::result::Error::RollbackTransaction);
        }

        // hash the password
        let salt = SaltString::generate(&mut OsRng);
        let argon2 = Argon2::default();
        let password_hash = argon2
            .hash_password(new_user.password.as_bytes(), &salt)
            .unwrap()
            .to_string();

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

        // insert the User into the database
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

    pub fn get(conn: &mut SqliteConnection, query: UserQuery)-> Option<User> {
        use crate::schema::users::dsl::*;
        match query {
            UserQuery::Id(user_id) => users.filter(id.eq(user_id)).first::<User>(conn).ok(),
            UserQuery::Email(email) => {
                users.filter(login_email.eq(email)).first::<User>(conn).ok()
            }
        }
    }
}
