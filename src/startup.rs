use actix_web::{dev::Server, web, App, HttpServer};
use sqlx::PgPool;
use std::net::TcpListener;

use crate::services::{health_check::health_check, subscriptions::subscribe};

pub fn run(listener: TcpListener, db_pool: PgPool) -> Result<Server, std::io::Error> {
    let db_pool = web::Data::new(db_pool);

    let server = HttpServer::new(move || {
        App::new()
            .service(health_check)
            .service(subscribe)
            .app_data(db_pool.clone())
    })
    .listen(listener)?
    .run();

    Ok(server)
}
