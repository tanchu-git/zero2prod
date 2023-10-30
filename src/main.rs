use sqlx::postgres::PgPoolOptions;
use std::io::stdout;
use std::net::TcpListener;
use zero2prod::config::get_config;
use zero2prod::startup::run;
use zero2prod::telemetry::{get_subscriber, init_subscriber};

mod tests;

#[tokio::main]
async fn main() -> std::io::Result<()> {
    let subscriber = get_subscriber("zero2prod", "info", stdout);
    init_subscriber(subscriber);

    let config = get_config().expect("Failed to read config file.");

    let connection_pool = PgPoolOptions::new()
        .acquire_timeout(std::time::Duration::from_secs(5))
        .connect_lazy_with(config.get_db().with_db());

    let address = format!("{}:{}", config.get_app_host(), config.get_app_port());

    let listener = TcpListener::bind(address).expect("Failed to bind address.");

    run(listener, connection_pool)?.await?;

    Ok(())
}
