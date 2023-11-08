use actix_web::{post, HttpResponse};

#[post("/newsletters")]
async fn publish_newsletter() -> HttpResponse {
    HttpResponse::Ok().finish()
}
