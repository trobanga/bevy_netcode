use std::collections::HashMap;

pub type PeerId = uuid::Uuid;

pub enum Message {}

#[derive(Debug, Default)]
pub struct Peer {
    id: PeerId,
}

impl Peer {
    pub fn new(id: PeerId) -> Self {
        Self { id }
    }
}

#[derive(Debug, Default)]
pub struct State {
    clients: HashMap<PeerId, Peer>,
}

impl State {
    pub async fn add_peer(&self, peer: Peer) {}
}
