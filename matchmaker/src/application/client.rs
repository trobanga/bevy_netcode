use std::time::{Duration, Instant};

use actix::prelude::*;
use actix_web_actors::ws;
use tracing::{debug, error, info};

/// How often heartbeat pings are sent
const HEARTBEAT_INTERVAL: Duration = Duration::from_secs(5);

/// How long before lack of client response causes a timeout
const CLIENT_TIMEOUT: Duration = Duration::from_secs(10);

#[derive(Debug)]
pub struct WsClient {
    id: Uuid,
    heartbeat: Instant,
    moderator: Addr<Moderator>,
}

impl WsClient {
    pub fn new(id: Uuid, moderator: Addr<Moderator>) -> Self {
        Self {
            id,
            heartbeat: Instant::now(),
            moderator,
        }
    }

    /// helper method that sends ping to client every 5 seconds (HEARTBEAT_INTERVAL).
    ///
    /// also this method checks heartbeats from client
    fn heartbeat(&self, ctx: &mut <Self as Actor>::Context) {
        ctx.run_interval(HEARTBEAT_INTERVAL, |act, ctx| {
            // check client heartbeats
            if Instant::now().duration_since(act.heartbeat) > CLIENT_TIMEOUT {
                // heartbeat timed out
                error!("Websocket Client heartbeat failed, disconnecting!");

                // stop actor
                ctx.stop();

                return;
            }

            ctx.ping(b"");
        });
    }
}

impl Handler<moderator::Message> for WsClient {
    type Result = ();

    fn handle(&mut self, msg: moderator::Message, ctx: &mut Self::Context) -> Self::Result {
        match msg {
            moderator::Message::NewPeer { id } => info!("Connect to {id}"),
            moderator::Message::PeerDisconnected { id: _ } => todo!(),
        }
    }
}

impl Actor for WsClient {
    type Context = ws::WebsocketContext<Self>;

    /// Method is called on actor start. We start the heartbeat process here.
    fn started(&mut self, ctx: &mut Self::Context) {
        self.heartbeat(ctx);

        let addr = ctx.address();
        info!("WsClient started, trying to connect");
        self.moderator
            .send(moderator::Connect {
                id: self.id,
                addr: addr.recipient(),
            })
            .into_actor(self)
            .then(|res, _act, ctx| {
                info!("bla: {:?}", res);
                match res {
                    Ok(res) => {
                        if let Err(moderator::Error::AlreadyConnected) = res {
                            error!("Already connected. Stopping.");
                            ctx.stop();
                        }
                    }
                    _ => ctx.stop(),
                }
                fut::ready(())
            })
            .wait(ctx);
        info!("started done");
    }

    fn stopping(&mut self, _: &mut Self::Context) -> Running {
        // notify chat server
        self.moderator
            .do_send(moderator::Disconnect { id: self.id });
        Running::Stop
    }
}

/// Handler for ws::Message message
impl StreamHandler<Result<ws::Message, ws::ProtocolError>> for WsClient {
    fn handle(&mut self, msg: Result<ws::Message, ws::ProtocolError>, ctx: &mut Self::Context) {
        debug!(?msg);
        match msg {
            Ok(ws::Message::Ping(msg)) => {
                self.heartbeat = Instant::now();
                ctx.pong(&msg);
            }
            Ok(ws::Message::Pong(_)) => {
                self.heartbeat = Instant::now();
            }
            Ok(ws::Message::Text(text)) => {
                info!("Got text {text}");
                ctx.text(text);
            }
            Ok(ws::Message::Binary(bin)) => ctx.binary(bin),
            Ok(ws::Message::Close(reason)) => {
                ctx.close(reason);
                ctx.stop();
            }
            _ => ctx.stop(),
        }
    }
}

use uuid::Uuid;
pub use ws::start;

use super::moderator::{self, Moderator};
