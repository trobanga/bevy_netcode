use actix::{Actor, StreamHandler};
use actix_web::{dev::Server, get, web, App, Error, HttpRequest, HttpResponse, HttpServer};
use actix_web_actors::ws;
use std::net::TcpListener;
use tracing::{debug, info};

use crate::{configuration::Settings, db};

pub struct Application {
    port: u16,
    server: Server,
}

impl Application {
    pub async fn build(configuration: Settings) -> Result<Self, anyhow::Error> {
        let address = format!(
            "{}:{}",
            configuration.application.host, configuration.application.port
        );
        let listener = TcpListener::bind(&address)?;
        let port = listener.local_addr().unwrap().port();
        info!("Running on port: {port}");
        let server = run(listener).await?;
        Ok(Self { port, server })
    }

    pub fn port(&self) -> u16 {
        self.port
    }

    pub async fn run_until_stopped(self) -> Result<(), std::io::Error> {
        self.server.await
    }
}

pub async fn run(listener: TcpListener) -> Result<Server, anyhow::Error> {
    let server = HttpServer::new(|| {
        let pool = db::create_pool();
        App::new()
            .app_data(web::Data::new(pool.clone()))
            .service(health_check)
            .service(index)
    })
    .workers(2)
    .listen(listener)?
    .run();
    Ok(server)
}

#[get("/health_check")]
async fn health_check() -> HttpResponse {
    HttpResponse::Ok().finish()
}

/// Define HTTP actor
struct Ws;

impl Actor for Ws {
    type Context = ws::WebsocketContext<Self>;
}

/// Handler for ws::Message message
impl StreamHandler<Result<ws::Message, ws::ProtocolError>> for Ws {
    fn handle(&mut self, msg: Result<ws::Message, ws::ProtocolError>, ctx: &mut Self::Context) {
        debug!(?msg);
        match msg {
            Ok(ws::Message::Ping(msg)) => ctx.pong(&msg),
            Ok(ws::Message::Text(text)) => ctx.text(text),
            Ok(ws::Message::Binary(bin)) => ctx.binary(bin),
            _ => (),
        }
    }
}

#[get("/")]
async fn index(req: HttpRequest, stream: web::Payload) -> Result<HttpResponse, Error> {
    let resp = ws::start(Ws {}, &req, stream);
    println!("{:?}", resp);
    resp
}
