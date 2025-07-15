use crate::api::handlers::{get_accounts, get_quote, new_order, get_positions};
use actix_web::web;

pub fn configure_routes(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::resource("/accounts")
            .route(web::post().to(get_accounts))
    )
    .service(
        web::resource("/quote")
            .route(web::get().to(get_quote))
    )
    .service(
        web::resource("/trade/order")
            .route(web::post().to(new_order))
    )
    .service(
        web::resource("/trade/positions")
            .route(web::get().to(get_positions))
    );
}
