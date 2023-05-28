use super::types::Error;
use crate::claims::Claims;
use crate::global::JWT_SECRET;
use crate::models::user::User;
use chrono::{Duration, Utc};
use jsonwebtoken::{decode, encode, Algorithm, DecodingKey, EncodingKey, Header, Validation};

const BEARER: &str = "Bearer ";
const JWT_DURATION_SECONDS: i64 = 60 * 15; // 15 minutes
const REFRESH_DURATION_SECONDS: i64 = 60 * 60 * 24 * 7; // 7 days

fn create_token(user: &User, duration: i64) -> Result<String, Error> {
    let expiration = Utc::now()
        .checked_add_signed(Duration::seconds(duration))
        .expect("valid timestamp")
        .timestamp();

    let uid = match user.id {
        Some(id) => id,
        None => return Err(Error::JWTSecretGenerationError),
    };

    let claims = Claims {
        sub: uid,
        exp: expiration as usize,
        role: user.role.clone(),
        email: user.login_email.clone(),
    };

    let secret = match JWT_SECRET.get() {
        Some(secret) => secret.as_bytes(),
        None => return Err(Error::JWTSecretGenerationError),
    };

    let header = Header::new(Algorithm::HS512);
    encode(&header, &claims, &EncodingKey::from_secret(&secret))
        .map_err(|_| Error::JWTCreationError)
}

pub fn create_access_token(user: &User) -> Result<String, Error> {
    create_token(user, JWT_DURATION_SECONDS)
}

pub fn create_refresh_token(user: &User) -> Result<String, Error> {
    create_token(user, REFRESH_DURATION_SECONDS)
}

pub fn verify_and_extract_claims(header_val: &str) -> Option<Claims> {
    let jwt_secret = match JWT_SECRET.get() {
        Some(secret) => secret.as_bytes(),
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

#[cfg(test)]
#[ctor::ctor]
fn init() {
    use crate::global::init_jwt_secret;
    use crate::test_helpers::test_helpers::get_test_db_connection;
    let mut conn = get_test_db_connection();
    init_jwt_secret(&mut conn)
}
#[cfg(test)]
mod tests {
    use base64::engine::general_purpose;

    use super::*;

    fn get_test_user() -> User {
        User {
            id: Some(1),
            login_email: "testy@mctestface.com".to_string(),
            send_email: "testy@mctestface.com".to_string(),
            role: "user".to_string(),
            password: "password".to_string(),
            created_at: Utc::now().timestamp() as i32,
            is_active: true,
            daily_send_time: "".to_string(),
            refresh_token: None,
        }
    }

    fn token_to_claims(token: &str) -> Claims {
        use base64::Engine;
        let token = token.split('.').collect::<Vec<&str>>()[1];
        let buf = general_purpose::STANDARD_NO_PAD.decode(&token).unwrap();
        let token = String::from_utf8(buf).unwrap();
        serde_json::from_str(&token).unwrap()
    }

    #[test]
    fn test_access_token() {
        let user = get_test_user();
        let jwt = create_access_token(&user);
        assert!(!jwt.is_err());
        let jwt = jwt.unwrap();
        assert!(jwt.len() > 0);

        let jwt = token_to_claims(&jwt);
        assert_eq!(jwt.email, user.login_email);
        assert_eq!(jwt.sub, user.id.unwrap());
        assert_eq!(jwt.role, user.role);
        // expires in about 15 minutes
        assert!(jwt.exp > Utc::now().timestamp() as usize + 15 * 60 - 5);
        assert!(jwt.exp < Utc::now().timestamp() as usize + 15 * 60 + 5);
    }

    #[test]
    fn test_refresh_token() {
        let user = get_test_user();
        let jwt = create_refresh_token(&user);
        assert!(!jwt.is_err());
        let jwt = jwt.unwrap();
        assert!(jwt.len() > 0);

        let jwt = token_to_claims(&jwt);
        assert_eq!(jwt.email, "testy@mctestface.com");
        assert_eq!(jwt.sub, 1);
        // expires in about 7 days
        assert!(jwt.exp > Utc::now().timestamp() as usize + 60 * 60 * 24 * 7 - 5);
        assert!(jwt.exp < Utc::now().timestamp() as usize + 60 * 60 * 24 * 7 + 5);
    }

    #[test]
    fn test_verify_fails_w_bad_signature() {
        let user = get_test_user();
        let jwt = create_access_token(&user);
        assert!(!jwt.is_err());
        let jwt = jwt.unwrap();
        assert!(jwt.len() > 0);
        let parts = jwt.split('.').collect::<Vec<&str>>();
        let sig = parts[2];
        let mut sig = sig.to_string();
        sig.push_str("a");
        let jwt = format!("{}.{}.{}", parts[0], parts[1], sig);
        let claims = verify_and_extract_claims(&jwt);
        assert!(claims.is_none());
    }

    #[test]
    fn test_verify_fails_on_manual_claim_change() {
        use base64::Engine;
        let user = get_test_user();
        let jwt = create_access_token(&user);
        assert!(!jwt.is_err());
        let jwt = jwt.unwrap();
        assert!(jwt.len() > 0);
        let parts = jwt.split('.').collect::<Vec<&str>>();
        let buf = general_purpose::STANDARD_NO_PAD.decode(&parts[1]).unwrap();
        let mut claims = String::from_utf8(buf).unwrap();

        // change roles from user to admin
        claims = claims.replace("user", "admin");

        // back to base64
        claims = general_purpose::STANDARD_NO_PAD.encode(&claims.as_bytes());

        let jwt = format!("{}.{}.{}", parts[0], claims, parts[2]);
        let claims = verify_and_extract_claims(&jwt);
        assert!(claims.is_none());
    }

    #[test]
    fn test_verify_fails_on_algo_none() {
        use base64::Engine;
        let user = get_test_user();
        let jwt = create_access_token(&user);
        assert!(!jwt.is_err());
        let jwt = jwt.unwrap();
        assert!(jwt.len() > 0);
        let parts = jwt.split('.').collect::<Vec<&str>>();
        let header = parts[0];

        // change algo from HS512 to none
        let buf = general_purpose::STANDARD_NO_PAD.decode(&header).unwrap();
        let mut header = String::from_utf8(buf).unwrap();
        header = header.replace("HS512", "none");
        let header = general_purpose::STANDARD_NO_PAD.encode(&header.as_bytes());

        let jwt = format!("{}.{}.{}", header, parts[1], parts[2]);

        let claims = verify_and_extract_claims(&jwt);
        assert!(claims.is_none());
    }
}
