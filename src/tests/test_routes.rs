#[cfg(test)]
mod tests {
    use rstest::rstest;
    use sqlx::{Connection, PgConnection};
    use std::net::TcpListener;
    use zero2prod::config::get_config;

    /// Spin up an instance of our application
    /// and returns its address (i.e. http://localhost:XXXX)
    fn spawn_app() -> String {
        let listener = TcpListener::bind("127.0.0.1:0").expect("Failed to bind random port.");

        let port = listener.local_addr().unwrap().port();

        let server = zero2prod::startup::run(listener).expect("Failed to bind address");

        let _ = tokio::spawn(server);

        format!("http://127.0.0.1:{port}")
    }

    #[actix_web::test]
    async fn test_health_check() {
        let address = spawn_app();
        let client = reqwest::Client::new();

        let response = client
            .get(&format!("{}/health_check", &address))
            .send()
            .await
            .expect("Request failed.");

        assert!(response.status().is_success());
        assert_eq!(Some(0), response.content_length());
    }

    #[tokio::test]
    async fn test_subscriber_code_200() {
        let app_address = spawn_app();

        let config = get_config().expect("Failed to read config file.");
        let connection_string = config.get_db().connection_string();
        let mut connection = PgConnection::connect(&connection_string)
            .await
            .expect("Failed to connect to Postgre.");

        let client = reqwest::Client::new();

        let body = "name=le%20guin&email=ursula_le_guin%40gmail.com";
        let response = client
            .post(&format!("{}/subscriptions", &app_address))
            .header("Content-Type", "application/x-www-form-urlencoded")
            .body(body)
            .send()
            .await
            .expect("Failed to execute request");

        assert_eq!(200, response.status().as_u16());

        let query = sqlx::query!("SELECT email, name FROM subscriptions")
            .fetch_one(&mut connection)
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
        let app_address = spawn_app();
        let client = reqwest::Client::new();

        let response = client
            .post(&format!("{}/subscriptions", &app_address))
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
