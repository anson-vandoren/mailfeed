use actix_web::{get, HttpResponse, Responder};

#[get("/")]
pub async fn get_items_for_feed() -> impl Responder {
    // check feed -> subscription -> user -> user.id -> claims.sub
    // maybe we can add this as a claim? or insert it w/ middleware?

    // check user is active

    HttpResponse::Ok().body("get_all_feed_items")
}

#[get("/")]
pub async fn get_feed_item() -> impl Responder {
    HttpResponse::Ok().body("get_feed_item")
}
