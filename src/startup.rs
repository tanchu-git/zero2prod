use actix_web::{dev::Server, web, App, HttpServer};
use sqlx::{postgres::PgPoolOptions, PgPool};
use std::net::TcpListener;
use tracing_actix_web::TracingLogger;

use crate::{
    config::Settings,
    email_client::EmailClient,
    services::{health_check::health_check, subscriptions::subscribe},
};

pub async fn build(config: &Settings) -> Result<Server, std::io::Error> {
    let connection_pool = get_connection_pool(config);

    let sender_email = config
        .get_email_client()
        .sender()
        .expect("Invalid email address.");

    let email_client = EmailClient::new(
        config.get_email_client().get_base_url().to_string(),
        sender_email,
        config.get_email_client().get_secret(),
        config.get_email_client().get_timeout(),
    );

    let address = format!("{}:{}", config.get_app_host(), config.get_app_port());
    let listener = TcpListener::bind(address)?;

    run(listener, connection_pool, email_client)
}

pub fn get_connection_pool(config: &Settings) -> PgPool {
    PgPoolOptions::new()
        .acquire_timeout(std::time::Duration::from_secs(5))
        .connect_lazy_with(config.get_db().with_db())
}

pub fn run(
    listener: TcpListener,
    db_pool: PgPool,
    email_client: EmailClient,
) -> Result<Server, std::io::Error> {
    let db_pool = web::Data::new(db_pool);
    let email_client = web::Data::new(email_client);

    let server = HttpServer::new(move || {
        App::new()
            .wrap(TracingLogger::default())
            .service(health_check)
            .service(subscribe)
            .app_data(db_pool.clone())
            .app_data(email_client.clone())
    })
    .listen(listener)?
    .run();

    Ok(server)
}
