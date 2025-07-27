use actix_web::web;
use serde::Deserialize;

use crate::models::user::PartialUser;

#[derive(Debug, Deserialize)]
pub struct UserPath {
    pub user_id: String,
}

pub type RqUserId = web::Path<UserPath>;
pub type RqPartUser = web::Form<PartialUser>;
