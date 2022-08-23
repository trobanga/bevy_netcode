use actix::{Actor, StreamHandler};
use actix_web::{dev::Server, get, web, App, Error, HttpRequest, HttpResponse, HttpServer};
use actix_web_actors::ws;
use std::net::TcpListener;
use tracing::{debug, info};

use crate::{authentication::basic_authentication, db::DbPool, settings::Settings};

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
    Ok(HttpServer::new(move || {
        App::new()
            .app_data(pool.clone())
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

/// Define HTTP actor
struct Ws;

impl Actor for Ws {
    type Context = ws::WebsocketContext<Self>;

    // /// Method is called on actor start. We start the heartbeat process here.
    // fn started(&mut self, ctx: &mut Self::Context) {
    //     self.hb(ctx);
    // }
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
async fn index(
    req: HttpRequest,
    stream: web::Payload,
    pool: web::Data<DbPool>,
) -> Result<HttpResponse, Error> {
    let mut conn = pool.get().expect("Could not get DbConnection");
    basic_authentication(req.headers(), &mut conn).await?;
    let resp = ws::start(Ws {}, &req, stream);
    resp
}
