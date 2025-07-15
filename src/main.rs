use anyhow::Result;
use dotenv::dotenv;

mod api;
mod settings;
mod validate;

#[actix_web::main]
async fn main() -> Result<()> {
    dotenv().ok();
    settings::init_logger()?;
    api::server::start_server().await?;
    Ok(())
}
