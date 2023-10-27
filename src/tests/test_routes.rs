#[cfg(test)]
mod tests {
    use once_cell::sync::Lazy;
    use rstest::rstest;
    use secrecy::ExposeSecret;
    use sqlx::{Connection, Executor, PgConnection, PgPool};
    use std::io::{sink, stdout};
    use std::net::TcpListener;
    use uuid::Uuid;
    use zero2prod::config::get_config;
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
        address: String,
        db_pool: PgPool,
    }

    /// Spin up an instance of our application
    /// and returns its address (i.e. http://localhost:XXXX)
    #[allow(clippy::let_underscore_future)]
    async fn spawn_app() -> TestApp {
        // The first time `initialize` is invoked the code in `TRACING` is executed.
        // All other invocations will instead skip execution.
        Lazy::force(&TRACING);

        let listener = TcpListener::bind("127.0.0.1:0").expect("Failed to bind random port.");

        // We retrieve the port assigned to us by the OS
        let port = listener.local_addr().unwrap().port();
        let address = format!("http://127.0.0.1:{port}");

        // Get brand new database
        let connection_pool = create_db().await;
        let server = zero2prod::startup::run(listener, connection_pool.clone())
            .expect("Failed to bind address");
        let _ = tokio::spawn(server);

        TestApp {
            address,
            db_pool: connection_pool,
        }
    }

    // Spin up a brand-new logical database for each integration test run
    async fn create_db() -> PgPool {
        // Randomize name for database
        let mut config = get_config().expect("Failed to read config file.");
        config.set_db_name(Uuid::new_v4().to_string());

        // Create database with randomized name
        let mut connection =
            PgConnection::connect(config.get_db().connection_string_no_db().expose_secret())
                .await
                .expect("Failed to connect to Postgres");
        connection
            .execute(format!(r#"CREATE DATABASE "{}";"#, config.get_db_name()).as_str())
            .await
            .expect("Failed to create database.");

        // Migrate database
        let connection_pool = PgPool::connect(config.get_db().connection_string().expose_secret())
            .await
            .expect("Failed to connect to Postgres.");
        sqlx::migrate!("./migrations")
            .run(&connection_pool)
            .await
            .expect("Failed to migrate the database");

        connection_pool
    }

    #[actix_web::test]
    async fn test_health_check() {
        let app = spawn_app().await;
        let client = reqwest::Client::new();

        let response = client
            .get(&format!("{}/health_check", &app.address))
            .send()
            .await
            .expect("Request failed.");

        assert!(response.status().is_success());
        assert_eq!(Some(0), response.content_length());
    }

    #[actix_web::test]
    async fn test_subscriber_code_200() {
        let app = spawn_app().await;
        let client = reqwest::Client::new();

        let body = "name=le%20guin&email=ursula_le_guin%40gmail.com";
        let response = client
            .post(&format!("{}/subscriptions", &app.address))
            .header("Content-Type", "application/x-www-form-urlencoded")
            .body(body)
            .send()
            .await
            .expect("Failed to execute request");

        assert_eq!(200, response.status().as_u16());

        let query = sqlx::query!("SELECT email, name FROM subscriptions")
            .fetch_one(&app.db_pool)
            .await
            .expect("Failed to fetch saved subscription.");

        assert_eq!(query.email, "ursula_le_guin@gmail.com");
        assert_eq!(query.name, "le guin");
    }

    #[rstest]
    #[case("name=le%20guin", "missing the email")]
    #[case("email=ursula_le_guin%40gmail.com", "missing the name")]
    #[case("", "missing both name and email")]
    #[actix_web::test]
    async fn test_subscriber_code_400(#[case] body: String, #[case] err_msg: &str) {
        let app = spawn_app().await;
        let client = reqwest::Client::new();

        let response = client
            .post(&format!("{}/subscriptions", &app.address))
            .header("Content-Type", "application/x-www-form-urlencoded")
            .body(body)
            .send()
            .await
            .expect("Failed to execute request");

        assert_eq!(
            400,
            response.status().as_u16(),
            "API did NOT fail with 400 Bad Request when payload was {err_msg}."
        );
    }
}
