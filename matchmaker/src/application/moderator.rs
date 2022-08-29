use std::collections::HashMap;

use actix::prelude::*;
use tracing::info;
use uuid::Uuid;

#[derive(Message)]
#[rtype(result = "()")]
pub enum Message {
    NewPeer { id: Uuid, addr: Recipient<Message> },
    Peers(HashMap<Uuid, Recipient<Message>>),
    PeerDisconnected { id: Uuid },
    PeerMessage(client::message::Message),
}

#[derive(Message)]
#[rtype(result = "()")]
pub struct PeerMessage(pub client::message::PeerMessage);

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("Already connected")]
    AlreadyConnected,
}

#[derive(Message)]
#[rtype(result = "Result<(), Error>")]
pub struct Connect {
    pub id: Uuid,
    pub addr: Recipient<Message>,
}

#[derive(Message)]
#[rtype(result = "()")]
pub struct Disconnect {
    pub id: Uuid,
}

#[derive(Default)]
pub struct Moderator {
    clients: HashMap<Uuid, Recipient<Message>>,
}

impl Actor for Moderator {
    type Context = Context<Self>;
}

impl Handler<Connect> for Moderator {
    type Result = Result<(), Error>;

    fn handle(&mut self, msg: Connect, ctx: &mut Self::Context) -> Self::Result {
        info!("Client {} requests connection", msg.id);
        if self.clients.contains_key(&msg.id) {
            return Err(Error::AlreadyConnected);
        }

        let peers = self.clients.clone();

        self.clients.insert(msg.id, msg.addr.clone());

        for (id, client) in self.clients.iter() {
            if *id == msg.id {
                client
                    .send(Message::Peers(peers.clone()))
                    .into_actor(self)
                    .then(|_, _, _| fut::ready(()))
                    .wait(ctx);
                continue;
            }
            client
                .send(Message::NewPeer {
                    id: msg.id,
                    addr: msg.addr.clone(),
                })
                .into_actor(self)
                .then(|_, _, _| fut::ready(()))
                .wait(ctx);
        }
        Ok(())
    }
}

impl Handler<Disconnect> for Moderator {
    type Result = ();

    fn handle(&mut self, _msg: Disconnect, _ctx: &mut Self::Context) -> Self::Result {}
}
