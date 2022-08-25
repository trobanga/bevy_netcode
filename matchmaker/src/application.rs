use actix::*;
use actix_web::{dev::Server, get, web, App, Error, HttpRequest, HttpResponse, HttpServer};
use std::net::TcpListener;
use tracing::info;

use crate::{authentication::basic_authentication, db::DbPool, settings::Settings};

use self::moderator::Moderator;

mod client;
mod moderator;

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
            .service(index)
    })
    .listen(listener)?
    .run())
}

#[get("/health_check")]
async fn health_check() -> HttpResponse {
    HttpResponse::Ok().finish()
}

#[get("/")]
async fn index(
    req: HttpRequest,
    stream: web::Payload,
    pool: web::Data<DbPool>,
    moderator: web::Data<Addr<Moderator>>,
) -> Result<HttpResponse, Error> {
    let mut conn = pool.get().expect("Could not get DbConnection");
    let id = basic_authentication(req.headers(), &mut conn).await?;
    let websocket = client::WsClient::new(id, moderator.get_ref().clone());
    client::start(websocket, &req, stream)
}
