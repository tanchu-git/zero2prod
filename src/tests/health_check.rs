fn spawn_app() {
    let server = zero2prod::run().expect("Failed to bind address");
    // Launch the server as a background task
    // tokio::spawn returns a handle to the spawned future,
    // but we have no use for it here, hence the non-binding let
    let _ = tokio::spawn(server);
}

#[actix_web::test]
async fn test_health_check() {
    spawn_app();

    let client = reqwest::Client::new();

    let response = client
        .get("http://127.0.0.1:8080/health_check")
        .send()
        .await
        .expect("Request failed.");

    assert!(response.status().is_success());
    assert_eq!(Some(0), response.content_length())
}
