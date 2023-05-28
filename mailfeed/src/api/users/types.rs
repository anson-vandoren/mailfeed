use actix_web::web;
use serde::Deserialize;

use crate::models::user::PartialUser;

#[derive(Debug, Deserialize)]
pub struct UserPath {
    pub id: String,
}

pub type RqUserId = web::Path<UserPath>;
pub type RqPartUser = web::Json<PartialUser>;
