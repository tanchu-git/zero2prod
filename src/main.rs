use std::net::TcpListener;
use zero2prod::startup::run;

mod tests;

#[tokio::main]
async fn main() -> std::io::Result<()> {
    let listener = TcpListener::bind("http://127.0.0.1:8080").expect("Failed to bind address.");

    run(listener)?.await
}
