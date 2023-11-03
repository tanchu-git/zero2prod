#[cfg(test)]
mod tests {
    use once_cell::sync::Lazy;
    use rand::Rng;
    use rstest::rstest;
    use sqlx::{Connection, Executor, PgConnection, PgPool};
    use std::io::{sink, stdout};
    use uuid::Uuid;
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

    struct TestApp {
        address: String,
        db_pool: PgPool,
    }

    impl TestApp {
        async fn make_post_request(&self, body: &str) -> reqwest::Response {
            reqwest::Client::new()
                .post(&format!("{}/subscriptions", &self.address))
                .header("Content-Type", "application/x-www-form-urlencoded")
                .body(body.to_string())
                .send()
                .await
                .expect("Failed to execute request")
        }
    }

    /// Spin up an instance of our application
    /// and returns its address (i.e. http://localhost:XXXX)
    #[allow(clippy::let_underscore_future)]
    async fn spawn_app() -> TestApp {
        // The first time `initialize` is invoked the code in `TRACING` is executed.
        // All other invocations will instead skip execution.
        Lazy::force(&TRACING);

        // Randomise configuration to ensure test isolation
        let random_port = rand::thread_rng().gen_range(8000..9000);
        let config = {
            let mut c = get_config().expect("Failed to read config.");
            // Use a different database for each test case
            c.set_db_name(Uuid::new_v4().to_string());
            // Use a random OS port
            c.set_app_port(random_port);
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

        let body = "name=le%20guin&email=ursula_le_guin%40gmail.com";
        let response = app.make_post_request(body).await;

        assert_eq!(200, response.status().as_u16());

        let query = sqlx::query!("SELECT email, name FROM subscriptions")
            .fetch_one(&app.db_pool)
            .await
            .expect("Failed to fetch saved subscription.");

        assert_eq!(query.email, "ursula_le_guin@gmail.com");
        assert_eq!(query.name, "le guin");
    }

    #[rstest]
    #[case("name=&email=ursula_le_guin%40gmail.com", "empty name")]
    #[case("name=Ursula&email=", "empty email")]
    #[case("name=Ursula&email=definitely-not-an-email", "invalid email")]
    #[trace]
    #[actix_web::test]
    async fn test_subscriber_code_400_with_empty_field(
        #[case] body: &str,
        #[case] test_case: &str,
    ) {
        let app = spawn_app().await;

        let response = app.make_post_request(body).await;

        assert_eq!(
            400,
            response.status().as_u16(),
            "The API did not return a 400 OK when the test case was {test_case}."
        );
    }

    #[rstest]
    #[case("name=le%20guin", "missing the email")]
    #[case("email=ursula_le_guin%40gmail.com", "missing the name")]
    #[case("", "missing both name and email")]
    #[trace]
    #[actix_web::test]
    async fn test_subscriber_code_400(#[case] body: &str, #[case] test_case: &str) {
        let app = spawn_app().await;

        let response = app.make_post_request(body).await;

        assert_eq!(
            400,
            response.status().as_u16(),
            "API did NOT fail with 400 Bad Request when test case was {test_case}."
        );
    }
}
