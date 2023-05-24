use actix_web::{HttpResponse, Responder};

pub async fn get_all_feeds() -> impl Responder {
    HttpResponse::Ok().body("get_all_feeds")
}

pub async fn create_feed() -> impl Responder {
    HttpResponse::Ok().body("create_feed")
}

pub async fn get_feed() -> impl Responder {
    HttpResponse::Ok().body("get_feed")
}

pub async fn update_feed() -> impl Responder {
    HttpResponse::Ok().body("update_feed")
}

pub async fn delete_feed() -> impl Responder {
    HttpResponse::Ok().body("delete_feed")
}
