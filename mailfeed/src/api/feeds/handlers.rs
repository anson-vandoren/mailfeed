use actix_web::{delete, get, patch, post, HttpResponse, Responder};

#[get("")]
pub async fn get_all_feeds() -> impl Responder {
    HttpResponse::Ok().body("get_all_feeds")
}

#[post("")]
pub async fn create_feed() -> impl Responder {
    HttpResponse::Ok().body("create_feed")
}

#[get("/{id}")]
pub async fn get_feed() -> impl Responder {
    HttpResponse::Ok().body("get_feed")
}

#[patch("/{id}")]
pub async fn update_feed() -> impl Responder {
    HttpResponse::Ok().body("update_feed")
}

#[delete("/{id}")]
pub async fn delete_feed() -> impl Responder {
    HttpResponse::Ok().body("delete_feed")
}
