use crate::handlers;

use actix_web::web;

pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/api")
            .service(
                web::scope("/users")
                    // TODO: all these routes should be protected by auth
                    .route("", web::get().to(handlers::user::get_all_users))
                    .route("", web::post().to(handlers::user::create_user))
                    .route("/{id}", web::get().to(handlers::user::get_user))
                    .route("/{id}", web::patch().to(handlers::user::update_user))
                    .route("/{id}", web::delete().to(handlers::user::delete_user))
                    .service(
                        web::scope("/{user_id}/subscriptions")
                            .route(
                                "",
                                web::get().to(handlers::subscription::get_all_subscriptions),
                            )
                            .route(
                                "",
                                web::post().to(handlers::subscription::create_subscription),
                            )
                            .route(
                                "/{id}",
                                web::get().to(handlers::subscription::get_subscription),
                            )
                            .route(
                                "/{id}",
                                web::put().to(handlers::subscription::update_subscription),
                            )
                            .route(
                                "/{id}",
                                web::delete().to(handlers::subscription::delete_subscription),
                            ),
                    ),
            )
            .service(
                web::scope("/auth")
                    .route("/login", web::post().to(handlers::auth::login))
                    .route("/logout", web::post().to(handlers::auth::logout))
                    .route(
                        "/password-reset",
                        web::post().to(handlers::auth::password_reset),
                    )
                    .route(
                        "/password-reset/{token}",
                        web::post().to(handlers::auth::password_reset_confirm),
                    )
                    .route(
                        "/change_password",
                        web::post().to(handlers::auth::change_password),
                    ),
            )
            .service(
                web::scope("/feeds")
                    .route("", web::get().to(handlers::feed::get_all_feeds))
                    .route("", web::post().to(handlers::feed::create_feed))
                    .route("/{id}", web::get().to(handlers::feed::get_feed))
                    .route("/{id}", web::put().to(handlers::feed::update_feed))
                    .route("/{id}", web::delete().to(handlers::feed::delete_feed))
                    .service(
                        web::scope("/{feed_id}/items")
                            .route("", web::get().to(handlers::feed_item::get_all_feed_items))
                            .route("/{id}", web::get().to(handlers::feed_item::get_feed_item)),
                    ),
            ),
    );
}
