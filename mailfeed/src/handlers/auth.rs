use crate::models::user::{PartialUser, User, UserQuery};
use actix_web::{web, HttpRequest, HttpResponse, Responder};
use chrono::{Duration, Utc};
use diesel::SqliteConnection;
use jsonwebtoken::{decode, encode, Algorithm, DecodingKey, EncodingKey, Header, Validation};
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

#[derive(Debug, Deserialize, Serialize)]
pub struct LoginResponse {
    pub access_token: String,
    pub refresh_token: String,
}

pub async fn login(pool: web::Data<DbPool>, login_req: web::Json<LoginRequest>) -> impl Responder {
    let mut conn = pool.get().expect("couldn't get db connection from pool");

    let user = match User::get(&mut conn, UserQuery::Email(login_req.email.clone())) {
        Some(user) => user,
        None => return HttpResponse::BadRequest().body("Invalid email or password"),
    };

    let is_password_correct = match User::check_password(&user, &login_req.password) {
        Ok(is_correct) => is_correct,
        Err(_) => return HttpResponse::BadRequest().body("Invalid email or password"),
    };

    if !is_password_correct {
        return HttpResponse::BadRequest().body("Invalid email or password");
    }

    let refresh_token = match create_refresh_token(&mut conn, &user.login_email, &user.role) {
        Ok(token) => token,
        Err(_) => return HttpResponse::InternalServerError().body("Error creating refresh token"),
    };

    let access_token = match create_access_token(&mut conn, &user.login_email, &user.role) {
        Ok(token) => token,
        Err(_) => return HttpResponse::InternalServerError().body("Error creating access token"),
    };

    let updates = PartialUser {
        refresh_token: Some(refresh_token.clone()),
        ..Default::default()
    };
    // add refresh token to users table
    if let Err(e) = User::update(&mut conn, user.id.unwrap(), &updates) {
        log::error!("Error updating user: {:?}", e);
        return HttpResponse::InternalServerError().body("Error updating user");
    }

    let response = LoginResponse {
        access_token,
        refresh_token,
    };

    HttpResponse::Ok().json(response)
}

pub async fn logout(pool: web::Data<DbPool>, req: HttpRequest) -> impl Responder {
    let mut conn = pool.get().expect("couldn't get db connection from pool");

    let auth_header = req.headers().get("Authorization");
    if auth_header.is_none() {
        return HttpResponse::BadRequest().body("No Authorization header");
    }

    let auth_header = auth_header.unwrap();
    let auth_header = auth_header.to_str().unwrap();

    let claims = verify_and_extract_claims(&mut conn, auth_header);

    if claims.is_none() {
        return HttpResponse::Unauthorized().body("Invalid token");
    }

    let claims = claims.unwrap();

    if let Err(e) = User::clear_refresh_token(&mut conn, UserQuery::Email(claims.sub.clone())) {
        log::error!("Error clearing refresh token: {:?}", e);
        return HttpResponse::InternalServerError().body("Error clearing refresh token");
    }

    HttpResponse::Ok().body("logout successful")
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
const REFRESH_DURATION_SECONDS: i64 = 60 * 60 * 24 * 7; // 70 days

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

fn create_access_token(
    conn: &mut SqliteConnection,
    login_email: &str,
    role: &str,
) -> Result<String, Error> {
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

fn create_refresh_token(
    conn: &mut SqliteConnection,
    login_email: &str,
    role: &str,
) -> Result<String, Error> {
    let expiration = Utc::now()
        .checked_add_signed(Duration::seconds(REFRESH_DURATION_SECONDS))
        .expect("valid timestamp")
        .timestamp();

    let claims = Claims {
        sub: login_email.to_owned(),
        exp: expiration as usize,
        role: role.to_owned(),
    };

    let jwt_secret = match get_jwt_secret(conn) {
        Some(secret) => secret.clone().into_bytes(),
        None => return Err(Error::JWTSecretGenerationError),
    };

    let header = Header::new(Algorithm::HS512);
    encode(&header, &claims, &EncodingKey::from_secret(&jwt_secret))
        .map_err(|_| Error::JWTCreationError)
}

fn verify_and_extract_claims(conn: &mut SqliteConnection, header_val: &str) -> Option<Claims> {
    let jwt_secret = match get_jwt_secret(conn) {
        Some(secret) => secret.clone().into_bytes(),
        None => return None,
    };

    let token = header_val.trim_start_matches(BEARER);
    if token.is_empty() {
        return None;
    }

    let validation = Validation::new(Algorithm::HS512);

    decode::<Claims>(token, &DecodingKey::from_secret(&jwt_secret), &validation)
        .map(|data| data.claims)
        .ok()
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
    use base64::engine::general_purpose;

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

    #[test]
    fn test_access_token() {
        use base64::Engine;
        let mut conn = get_test_db_connection();
        let jwt = create_access_token(&mut conn, "testy@mctestface.com", "user");
        assert!(!jwt.is_err());
        let jwt = jwt.unwrap();
        assert!(jwt.len() > 0);
        // split on '.' and take the second part
        let jwt = jwt.split('.').collect::<Vec<&str>>()[1];
        // decode base64
        let buf = general_purpose::STANDARD_NO_PAD.decode(&jwt).unwrap();
        let jwt = String::from_utf8(buf).unwrap();
        assert!(jwt.len() > 0);
        // decode json
        let jwt: Claims = serde_json::from_str(&jwt).unwrap();
        assert_eq!(jwt.sub, "testy@mctestface.com");
        assert_eq!(jwt.role, "user");
        // expires in about an hour
        assert!(jwt.exp > Utc::now().timestamp() as usize + 60 * 60 - 5);
        assert!(jwt.exp < Utc::now().timestamp() as usize + 60 * 60 + 5);
    }

    #[test]
    fn test_refresh_token() {
        use base64::Engine;
        let mut conn = get_test_db_connection();
        let jwt = create_refresh_token(&mut conn, "testy@mctestface.com", "user");
        assert!(!jwt.is_err());
        let jwt = jwt.unwrap();
        assert!(jwt.len() > 0);
        // split on '.' and take the second part
        let jwt = jwt.split('.').collect::<Vec<&str>>()[1];
        // decode base64
        let buf = general_purpose::STANDARD_NO_PAD.decode(&jwt).unwrap();
        let jwt = String::from_utf8(buf).unwrap();
        assert!(jwt.len() > 0);
        // decode json
        let jwt: Claims = serde_json::from_str(&jwt).unwrap();
        assert_eq!(jwt.sub, "testy@mctestface.com");
        // expires in about 7 days
        assert!(jwt.exp > Utc::now().timestamp() as usize + 60 * 60 * 24 * 7 - 5);
        assert!(jwt.exp < Utc::now().timestamp() as usize + 60 * 60 * 24 * 7 + 5);
    }
}
