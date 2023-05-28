use actix_web::{HttpResponse, Responder};

pub async fn get_all_subscriptions() -> impl Responder {
    HttpResponse::Ok().body("get_all_subscriptions")
}

pub async fn create_subscription() -> impl Responder {
    HttpResponse::Ok().body("create_subscription")
}

pub async fn get_subscription() -> impl Responder {
    HttpResponse::Ok().body("get_subscription")
}

pub async fn update_subscription() -> impl Responder {
    HttpResponse::Ok().body("update_subscription")
}

pub async fn delete_subscription() -> impl Responder {
    HttpResponse::Ok().body("delete_subscription")
}
