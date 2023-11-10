use once_cell::sync::Lazy;
use rand::Rng;
use sqlx::{Connection, Executor, PgConnection, PgPool};
use std::io::{sink, stdout};
use uuid::Uuid;
use wiremock::MockServer;
use zero2prod::config::{get_config, Settings};
use zero2prod::startup::get_connection_pool;
use zero2prod::telemetry::*;

static TRACING: Lazy<()> = Lazy::new(|| {
    let subscriber_name = "test";
    let default_filter = "info";

    // Start tracing and ensure that the `tracing` stack
    // is only initialised once using `once_cell`
    match std::env::var("TEST_LOG").is_ok() {
        true => {
            let subscriber = get_subscriber(subscriber_name, default_filter, stdout);
            init_subscriber(subscriber);
        }
        false => {
            let subscriber = get_subscriber(subscriber_name, default_filter, sink);
            init_subscriber(subscriber);
        }
    }
});

pub struct TestApp {
    pub address: String,
    pub db_pool: PgPool,
    pub email_server: MockServer,
}

impl TestApp {
    pub async fn make_post_request(&self, body: &str) -> reqwest::Response {
        reqwest::Client::new()
            .post(&format!("{}/subscriptions", &self.address))
            .header("Content-Type", "application/x-www-form-urlencoded")
            .body(body.to_string())
            .send()
            .await
            .expect("Failed to execute request")
    }

    pub async fn confirm_subscriber(&self, token: &str) -> reqwest::Response {
        reqwest::Client::new()
            .get(&format!(
                "{}/subscriptions/confirm?subscription_token={}",
                &self.address, token
            ))
            .send()
            .await
            .expect("Failed to execute request")
    }
}

/// Spin up an instance of our application
/// and returns its address (i.e. http://localhost:XXXX)
#[allow(clippy::let_underscore_future)]
pub async fn spawn_app() -> TestApp {
    // The first time `initialize` is invoked the code in `TRACING` is executed.
    // All other invocations will instead skip execution.
    Lazy::force(&TRACING);

    let email_server = MockServer::start().await;

    // Randomise configuration to ensure test isolation
    let random_port = rand::thread_rng().gen_range(1000..9000);
    let config = {
        let mut c = get_config().expect("Failed to read config.");
        // Use a different database for each test case
        c.set_db_name(Uuid::new_v4().to_string());
        // Use a random OS port
        c.set_app_port(random_port);
        c.set_email_client(email_server.uri());
        c
    };

    create_db(&config).await;

    let server = zero2prod::startup::build(&config)
        .await
        .expect("Failed to bind address");
    let address = format!("http://127.0.0.1:{}", config.get_app_port());
    let _ = tokio::spawn(server);

    TestApp {
        address,
        db_pool: get_connection_pool(&config),
        email_server,
    }
}

// Spin up a brand-new logical database for each integration test run
async fn create_db(config: &Settings) {
    let mut connection = PgConnection::connect_with(&config.get_db().without_db())
        .await
        .expect("Failed to connect to Postgres");
    connection
        .execute(format!(r#"CREATE DATABASE "{}";"#, config.get_db_name()).as_str())
        .await
        .expect("Failed to create database.");

    // Migrate database
    let connection_pool = PgPool::connect_with(config.get_db().with_db())
        .await
        .expect("Failed to connect to Postgres.");
    sqlx::migrate!("./migrations")
        .run(&connection_pool)
        .await
        .expect("Failed to migrate the database");
}
