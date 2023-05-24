use actix_web::{HttpResponse, Responder};

pub async fn get_all_feed_items() -> impl Responder {
    HttpResponse::Ok().body("get_all_feed_items")
}

pub async fn get_feed_item() -> impl Responder {
    HttpResponse::Ok().body("get_feed_item")
}