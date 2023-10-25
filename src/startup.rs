use actix_web::{dev::Server, App, HttpServer};
use std::net::TcpListener;

use crate::services::{health_check::health_check, subscriptions::subscribe};

pub fn run(listener: TcpListener) -> Result<Server, std::io::Error> {
    let server = HttpServer::new(|| App::new().service(health_check).service(subscribe))
        .listen(listener)?
        .run();

    Ok(server)
}
