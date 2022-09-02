use ggrs::{Message, PlayerType};
use uuid::Uuid;

use crate::{Packet, WebRTCSocket};

impl WebRTCSocket {
    #[must_use]
    pub fn players(&self) -> Vec<PlayerType<Uuid>> {
        // needs to be consistent order across all peers
        let mut ids: Vec<Uuid> = self.peers.keys().copied().collect();
        ids.push(self.id.to_owned());
        ids.sort();
        ids.iter()
            .map(|id| {
                if id == &self.id {
                    PlayerType::Local
                } else {
                    PlayerType::Remote(id.to_owned())
                }
            })
            .collect()
    }
}

impl ggrs::NonBlockingSocket<String> for WebRTCSocket {
    fn send_to(&mut self, msg: &Message, addr: &String) {
        if let Ok(id) = addr.as_str().try_into() {
            let payload = bincode::serialize(&msg).unwrap();
            let payload = bytes::Bytes::from(payload);
            let packet = Packet { id, payload };
            self.send_data(packet);
        }
    }

    fn receive_all_messages(&mut self) -> Vec<(String, Message)> {
        let mut messages = vec![];
        for packet in self.receive_data() {
            let msg = bincode::deserialize(&packet.payload).unwrap();
            messages.push((packet.id.to_string(), msg));
        }
        messages
    }
}
