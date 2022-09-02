use actix::*;
use actix_web::{dev::Server, web, App, HttpServer};
use std::net::TcpListener;
use tracing::info;

use crate::{db::DbPool, middleware::Authentication, settings::Settings};

mod client;
mod moderator;
mod services;

use moderator::Moderator;
use services::{health_check, login, show, useradd, userdel, users};

pub struct Application {
    port: u16,
    server: Server,
}

impl Application {
    pub async fn build(configuration: Settings, pool: DbPool) -> Result<Self, anyhow::Error> {
        let address = format!(
            "{}:{}",
            configuration.application.host, configuration.application.port
        );
        let listener = TcpListener::bind(&address)?;
        let port = listener.local_addr().unwrap().port();
        info!("Running on port: {port}");

        let server = create_server_with_pool(listener, pool)?;
        Ok(Self { port, server })
    }

    pub fn port(&self) -> u16 {
        self.port
    }

    pub async fn run_until_stopped(self) -> Result<(), std::io::Error> {
        self.server.await
    }
}

pub fn create_server_with_pool(
    listener: TcpListener,
    pool: DbPool,
) -> Result<Server, anyhow::Error> {
    let pool = web::Data::new(pool);
    let moderator = web::Data::new(Moderator::default().start());
    Ok(HttpServer::new(move || {
        App::new()
            .app_data(pool.clone())
            .app_data(moderator.clone())
            .service(health_check)
            .service(web::scope("/ws").service(login).wrap(Authentication))
            .service(
                web::scope("/user")
                    .service(useradd)
                    .service(userdel)
                    .service(show),
            )
            .service(users)
    })
    .listen(listener)?
    .run())
}
