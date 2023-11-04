#[cfg(test)]
mod tests {
    use once_cell::sync::Lazy;
    use rand::Rng;
    use rstest::rstest;
    use sqlx::{Connection, Executor, PgConnection, PgPool};
    use std::io::{sink, stdout};
    use uuid::Uuid;
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};
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
        email_server: MockServer,
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

        let email_server = MockServer::start().await;

        // Randomise configuration to ensure test isolation
        let random_port = rand::thread_rng().gen_range(8000..9000);
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
    async fn test_subscribe_sends_confirmation_email_for_valid_data() {
        let app = spawn_app().await;
        let body = "name=le%20guin&email=ursula_le_guin%40gmail.com";

        Mock::given(path("/campaigns/9b4079798b/actions/test"))
            .and(method("POST"))
            .respond_with(ResponseTemplate::new(200))
            .expect(1)
            .mount(&app.email_server)
            .await;

        app.make_post_request(body).await;
    }

    #[actix_web::test]
    async fn test_subscriber_code_200() {
        let app = spawn_app().await;
        let body = "name=le%20guin&email=ursula_le_guin%40gmail.com";

        Mock::given(path("/campaigns/9b4079798b/actions/test"))
            .and(method("POST"))
            .respond_with(ResponseTemplate::new(200))
            .mount(&app.email_server)
            .await;

        let response = app.make_post_request(body).await;
        assert_eq!(200, response.status().as_u16());
    }

    #[actix_web::test]
    async fn test_subscribe_does_persist_subscriber() {
        let app = spawn_app().await;
        let body = "name=le%20guin&email=ursula_le_guin%40gmail.com";

        Mock::given(path("/campaigns/9b4079798b/actions/test"))
            .and(method("POST"))
            .respond_with(ResponseTemplate::new(200))
            .mount(&app.email_server)
            .await;

        app.make_post_request(body).await;

        let query = sqlx::query!("SELECT email, name, status FROM subscriptions")
            .fetch_one(&app.db_pool)
            .await
            .expect("Failed to fetch saved subscription.");

        assert_eq!(query.email, "ursula_le_guin@gmail.com");
        assert_eq!(query.name, "le guin");
        assert_eq!(query.status, "pending_confirmation");
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

    #[actix_web::test]
    async fn confirmations_without_token_are_rejeceted_with_400() {
        let app = spawn_app().await;
        let response = reqwest::get(&format!("{}/subscriptions/confirm", &app.address))
            .await
            .unwrap();

        assert_eq!(response.status().as_u16(), 400);
    }
}
