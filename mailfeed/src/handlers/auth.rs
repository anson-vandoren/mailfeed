use crate::models::user::{User, UserQuery};
use actix_web::{web, HttpResponse, Responder};
use chrono::{Duration, Utc};
use diesel::SqliteConnection;
use jsonwebtoken::{encode, Algorithm, EncodingKey, Header};
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::{
    models::settings::{NewSetting, Setting},
    DbPool,
};

#[derive(Debug, Deserialize, Serialize)]
pub struct LoginRequest {
    pub email: String,
    pub password: String,
}

pub async fn login(pool: web::Data<DbPool>, login_req: web::Json<LoginRequest>) -> impl Responder {
    let mut conn = pool.get().expect("couldn't get db connection from pool");
    let user = User::get(&mut conn, UserQuery::Email(login_req.email.clone()));

    // if user not found, return with error
    let user = match user {
        Some(user) => user,
        None => return HttpResponse::BadRequest().body("Invalid email or password"),
    };

    // if password is incorrect, return with error
    let res = User::check_password(&user, &login_req.password);
    if res.is_err() || !res.unwrap() {
        return HttpResponse::BadRequest().body("Invalid email or password");
    }

    // if password is correct, create JWT and return it
    let jwt = create_jwt(&mut conn, &user.login_email, &user.roles);

    return match jwt {
        Ok(jwt) => HttpResponse::Ok().body(jwt),
        Err(_) => HttpResponse::InternalServerError().body("Error creating JWT"),
    };
}

pub async fn password_reset() -> impl Responder {
    HttpResponse::Ok().body("password_reset")
}

pub async fn password_reset_confirm() -> impl Responder {
    HttpResponse::Ok().body("password_reset_confirm")
}

pub async fn change_password() -> impl Responder {
    HttpResponse::Ok().body("change_password")
}

const BEARER: &str = "Bearer ";
const JWT_DURATION_SECONDS: i64 = 60 * 60; // 1 hour

#[derive(Error, Debug)]
pub enum Error {
    #[error("jwt creation error")]
    JWTCreationError,
    #[error("failed to get or create JWT secret")]
    JWTSecretGenerationError,
}

#[derive(Debug, Deserialize, Serialize)]
struct Claims {
    sub: String,
    role: String,
    exp: usize,
}

fn create_jwt(conn: &mut SqliteConnection, login_email: &str, role: &str) -> Result<String, Error> {
    let expiration = Utc::now()
        .checked_add_signed(Duration::seconds(JWT_DURATION_SECONDS))
        .expect("valid timestamp")
        .timestamp();

    let claims = Claims {
        sub: login_email.to_owned(),
        role: role.to_owned(),
        exp: expiration as usize,
    };

    let jwt_secret = match get_jwt_secret(conn) {
        Some(secret) => secret.clone().into_bytes(),
        None => return Err(Error::JWTSecretGenerationError),
    };

    let header = Header::new(Algorithm::HS512);
    encode(&header, &claims, &EncodingKey::from_secret(&jwt_secret))
        .map_err(|_| Error::JWTCreationError)
}

fn get_jwt_secret(conn: &mut SqliteConnection) -> Option<String> {
    use crate::schema::settings::dsl::*;
    use diesel::prelude::*;

    let result = settings
        .filter(key.eq("jwt_secret"))
        .select(value)
        .first::<String>(conn);

    if let Ok(secret) = result {
        return Some(secret);
    }

    let jwt_setting = NewSetting {
        user_id: None,
        key: "jwt_secret".to_string(),
        value: generate_jwt_secret(),
    };

    match Setting::add(conn, &jwt_setting) {
        Ok(setting) => Some(setting.value),
        Err(_) => None,
    }
}

fn generate_jwt_secret() -> String {
    use rand::distributions::Alphanumeric;
    use rand::{rngs::OsRng, Rng};

    let rng = OsRng;
    rng.sample_iter(&Alphanumeric)
        .take(32)
        .map(char::from)
        .collect()
}

#[cfg(test)]
mod tests {
    use crate::test_helpers::test_helpers::get_test_db_connection;

    use super::*;

    #[test]
    fn test_creates_jwt_secret_if_not_exists() {
        let mut conn = get_test_db_connection();
        let secret = get_jwt_secret(&mut conn);
        assert_ne!(secret, None);
        assert!(secret.unwrap().len() > 0);
    }

    #[test]
    fn test_same_secret_returned_if_exists() {
        let mut conn = get_test_db_connection();
        let secret = get_jwt_secret(&mut conn);
        let secret2 = get_jwt_secret(&mut conn);
        assert_eq!(secret, secret2);
    }

    #[test]
    fn test_secret_has_no_user_id() {
        let mut conn = get_test_db_connection();
        let secret = get_jwt_secret(&mut conn);
        assert_ne!(secret, None);

        let res = Setting::get(&mut conn, "jwt_secret", None);
        assert!(!res.is_err());
        let setting = res.unwrap();
        assert_eq!(setting.user_id, None);
    }
}
