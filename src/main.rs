use secrecy::ExposeSecret;
use sqlx::PgPool;
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
    let connection_pool = PgPool::connect(config.get_db().connection_string().expose_secret())
        .await
        .expect("Failed to connect to Postgres.");

    let address = format!("127.0.0.1:{}", config.get_port());

    let listener = TcpListener::bind(address).expect("Failed to bind address.");

    run(listener, connection_pool)?.await?;

    Ok(())
}
