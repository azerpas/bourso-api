use crate::api::routes::configure_routes;
use actix_web::{App, HttpServer};

pub async fn start_server() -> std::io::Result<()> {
    HttpServer::new(|| App::new().configure(configure_routes))
        .bind("127.0.0.1:8080")?
        .run()
        .await
}
