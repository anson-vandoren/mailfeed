use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error("jwt creation error")]
    JWTCreationError,
    #[error("failed to get or create JWT secret")]
    JWTSecretGenerationError,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct LoginRequest {
    pub email: String,
    pub password: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct TokenResponse<'a> {
    pub access_token: &'a str,
    pub refresh_token: &'a str,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct RefreshRequest {
    pub refresh_token: String,
}
