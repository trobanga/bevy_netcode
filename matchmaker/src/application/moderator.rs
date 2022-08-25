use std::collections::HashMap;

use actix::prelude::*;
use uuid::Uuid;

#[derive(Message)]
#[rtype(result = "()")]
pub enum Message {
    NewPeer { id: Uuid },
    PeerDisconnected { id: Uuid },
}

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
        if self.clients.contains_key(&msg.id) {
            return Err(Error::AlreadyConnected);
        }
        self.clients.insert(msg.id, msg.addr);

        for (id, client) in self.clients.iter() {
            if *id == msg.id {
                continue;
            }
            client
                .send(Message::NewPeer { id: msg.id })
                .into_actor(self)
                .then(|_, _, _| fut::ready(()))
                .wait(ctx);
        }
        Ok(())
    }
}

impl Handler<Disconnect> for Moderator {
    type Result = ();

    fn handle(&mut self, msg: Disconnect, ctx: &mut Self::Context) -> Self::Result {}
}
