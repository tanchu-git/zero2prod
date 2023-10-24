use zero2prod::run;

mod tests;

#[tokio::main]
async fn main() -> std::io::Result<()> {
    run()?.await
}
