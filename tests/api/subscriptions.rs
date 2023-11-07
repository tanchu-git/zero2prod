use rstest::rstest;
use wiremock::{
    matchers::{method, path},
    Mock, ResponseTemplate,
};

use crate::setup::spawn_app;

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
async fn test_subscriber_code_400_with_empty_field(#[case] body: &str, #[case] test_case: &str) {
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

#[tokio::test]
async fn subscribe_fails_if_there_is_a_fatal_database_error() {
    // Arrange
    let app = spawn_app().await;
    let body = "name=le%20guin&email=ursula_le_guin%40gmail.com";
    // Sabotage the database
    sqlx::query!("ALTER TABLE subscription_tokens DROP COLUMN subscription_token;",)
        .execute(&app.db_pool)
        .await
        .unwrap();
    // Act
    let response = app.make_post_request(body).await;
    // Assert
    assert_eq!(response.status().as_u16(), 500);
}
