use std::io::stdout;
use zero2prod::config::get_config;
use zero2prod::startup::build;
use zero2prod::telemetry::{get_subscriber, init_subscriber};

#[tokio::main]
async fn main() -> std::io::Result<()> {
    let subscriber = get_subscriber("zero2prod", "info", stdout);
    init_subscriber(subscriber);

    let config = get_config().expect("Failed to read config file.");

    let server = build(&config).await?;

    server.await?;

    Ok(())
}
