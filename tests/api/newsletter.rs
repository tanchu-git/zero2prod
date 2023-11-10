use crate::setup::{spawn_app, TestApp};

use wiremock::{
    matchers::{any, method, path},
    Mock, ResponseTemplate,
};

#[actix_web::test]
async fn newsletters_are_not_delivered_to_unconfirmed_subscribers() {
    let app = spawn_app().await;
    create_unconfirmed_subscriber(&app).await;
    Mock::given(any())
        .respond_with(ResponseTemplate::new(200))
        // We assert that no request is fired with our email provider!
        .expect(0)
        .mount(&app.email_server)
        .await;

    // A sketch of the newsletter payload structure.
    // We might change it later on.
    let newsletter_request_body = serde_json::json!({
    "test_emails": "Newsletter title",
    "type": "html"
    });

    let response = reqwest::Client::new()
        .post(&format!("{}/newsletters", &app.address))
        .json(&newsletter_request_body)
        .send()
        .await
        .expect("Failed to execute request.");

    assert_eq!(response.status().as_u16(), 200);
    // Mock verifies on Drop that we haven't sent the newsletter email
}

#[actix_web::test]
async fn newsletters_are_delivered_to_confirmed_subscribers() {
    let app = spawn_app().await;
    let token = create_unconfirmed_subscriber(&app).await;

    Mock::given(path("/campaigns/9b4079798b/actions/test"))
        .and(method("POST"))
        .respond_with(ResponseTemplate::new(200))
        // We assert that no request is fired with our email provider!
        //.expect(1)
        .mount(&app.email_server)
        .await;

    app.confirm_subscriber(&token).await;

    // A sketch of the newsletter payload structure.
    // We might change it later on.
    let newsletter_request_body = serde_json::json!({
    "test_emails": "Newsletter title",
    "type": "html"
    });

    let response = reqwest::Client::new()
        .post(&format!("{}/newsletters", &app.address))
        .json(&newsletter_request_body)
        .send()
        .await
        .expect("Failed to execute request.");

    assert_eq!(response.status().as_u16(), 200);
    // Mock verifies on Drop that we haven't sent the newsletter email
}

/// Use the public API of the application under test to create
/// an unconfirmed subscriber.
async fn create_unconfirmed_subscriber(app: &TestApp) -> String {
    let body = "name=le%20guin&email=ursula_le_guin%40gmail.com";

    let _mock_guard = Mock::given(path("campaigns/9b4079798b/actions/test"))
        .and(method("POST"))
        .respond_with(ResponseTemplate::new(200))
        .named("Create unconfirmed subscriber")
        .expect(1)
        .mount_as_scoped(&app.email_server)
        .await;
    app.make_post_request(body)
        .await
        .error_for_status()
        .unwrap();

    let query = sqlx::query!("SELECT subscription_token, subscriber_id FROM subscription_tokens")
        .fetch_one(&app.db_pool)
        .await
        .expect("Failed to fetch saved subscription.");

    query.subscription_token.clone()
}
