use std::future::{ready, Ready};

use crate::{global::JWT_SECRET, types::ErrorMessage};
use actix_web::{error::ResponseError, http::StatusCode, FromRequest, HttpRequest, HttpResponse};
use actix_web_httpauth::extractors::bearer::BearerAuth;
use derive_more::Display;
use jsonwebtoken::{decode, Algorithm, DecodingKey, Validation};
use serde::{Deserialize, Serialize};

#[derive(Debug, Display)]
enum ClientError {
    #[display(fmt = "decode")]
    Decode(jsonwebtoken::errors::Error),
    #[display(fmt = "not_found")]
    NotFound(String),
}
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Claims {
    pub sub: i32,
    pub role: String,
    pub exp: usize,
    pub email: String,
}

impl ResponseError for ClientError {
    fn error_response(&self) -> HttpResponse {
        match self {
            Self::Decode(_) => HttpResponse::Unauthorized().json(ErrorMessage {
                error: Some("invalid_token".to_string()),
                error_description: Some(
                    "Authorization header value must follow this format: Bearer access-token"
                        .to_string(),
                ),
                message: "Bad credentials".to_string(),
            }),
            Self::NotFound(msg) => HttpResponse::Unauthorized().json(ErrorMessage {
                error: Some("invalid_token".to_string()),
                error_description: Some(msg.to_string()),
                message: "Bad credentials".to_string(),
            }),
        }
    }

    fn status_code(&self) -> StatusCode {
        StatusCode::UNAUTHORIZED
    }
}

impl FromRequest for Claims {
    type Error = actix_web::Error;
    type Future = Ready<Result<Self, Self::Error>>;

    fn from_request(req: &HttpRequest, _: &mut actix_web::dev::Payload) -> Self::Future {
        let bearer_auth = match BearerAuth::extract(req).into_inner() {
            Ok(auth) => auth,
            Err(err) => return ready(Err(err.into())),
        };

        let token = bearer_auth.token();

        let mut validation = Validation::new(Algorithm::HS512);
        validation.set_audience(&["mailfeed"]);

        let jwt_secret = JWT_SECRET
            .get()
            .ok_or_else(|| ClientError::NotFound("JWT_SECRET not found".to_string()));

        let key = match jwt_secret {
            Ok(secret) => DecodingKey::from_secret(secret.as_bytes()),
            Err(err) => return ready(Err(err.into())),
        };

        let token = match decode::<Claims>(token, &key, &validation).map_err(ClientError::Decode) {
            Ok(token) => token,
            Err(err) => return ready(Err(err.into())),
        };

        ready(Ok(token.claims.clone()))
    }
}
