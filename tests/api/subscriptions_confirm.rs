use crate::setup::spawn_app;

#[actix_web::test]
async fn confirmations_without_token_are_rejected_with_400() {
    let app = spawn_app().await;
    let response = reqwest::get(&format!("{}/subscriptions/confirm", &app.address))
        .await
        .unwrap();

    assert_eq!(response.status().as_u16(), 400);
}
