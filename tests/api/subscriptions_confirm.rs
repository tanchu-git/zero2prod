use wiremock::{
    matchers::{method, path},
    Mock, ResponseTemplate,
};

use crate::setup::spawn_app;

#[actix_web::test]
async fn confirmations_without_token_are_rejected_with_400() {
    let app = spawn_app().await;
    let response = reqwest::get(&format!("{}/subscriptions/confirm", &app.address))
        .await
        .unwrap();

    assert_eq!(response.status().as_u16(), 400);
}

#[actix_web::test]
async fn test_confirmation_persist() {
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

    assert_eq!(query.status, "pending_confirmation");

    let query = sqlx::query!("SELECT subscription_token, subscriber_id FROM subscription_tokens")
        .fetch_one(&app.db_pool)
        .await
        .expect("Failed to fetch saved subscription.");

    app.confirm_subscriber(&query.subscription_token).await;

    let q = sqlx::query!("SELECT email, name, status FROM subscriptions")
        .fetch_one(&app.db_pool)
        .await
        .expect("Failed to fetch saved subscription.");

    assert_eq!(q.status, "confirmed");
}
