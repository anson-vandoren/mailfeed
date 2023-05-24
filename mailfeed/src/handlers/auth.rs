use actix_web::{HttpResponse, Responder};

pub async fn login() -> impl Responder {
    HttpResponse::Ok().body("login")
}

pub async fn logout() -> impl Responder {
    HttpResponse::Ok().body("logout")
}

pub async fn password_reset() -> impl Responder {
    HttpResponse::Ok().body("password_reset")
}