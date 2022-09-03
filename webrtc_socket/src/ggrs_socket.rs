use ggrs::{Message, PlayerType};
use tokio::sync::mpsc::{self, UnboundedReceiver, UnboundedSender};
use uuid::Uuid;

use crate::{message::StateMessage, Packet, WebRTCSocket};

// impl WebRTCSocket {
//     #[must_use]
//     pub fn players(&self) -> Vec<PlayerType<Uuid>> {
//         // needs to be consistent order across all peers
//         let mut ids: Vec<Uuid> = self.peers.keys().copied().collect();
//         ids.push(self.id.to_owned());
//         ids.sort();
//         ids.iter()
//             .map(|id| {
//                 if id == &self.id {
//                     PlayerType::Local
//                 } else {
//                     PlayerType::Remote(id.to_owned())
//                 }
//             })
//             .collect()
//     }
// }

// impl ggrs::NonBlockingSocket<Uuid> for WebRTCSocket {
//     fn send_to(&mut self, msg: &Message, addr: &Uuid) {
//         let payload = bincode::serialize(&msg).unwrap();
//         let payload = bytes::Bytes::from(payload);
//         let packet = Packet { id: *addr, payload };
//         self.send_data(packet);
//     }

//     fn receive_all_messages(&mut self) -> Vec<(Uuid, Message)> {
//         let mut messages = vec![];
//         for packet in self.receive_data().unwrap() {
//             let msg = bincode::deserialize(&packet.payload).unwrap();
//             messages.push((packet.id, msg));
//         }
//         messages
//     }
// }

#[derive(Debug)]
pub struct GgrsSocket {
    id: Uuid,
    in_data_rx: UnboundedReceiver<Packet>,
    out_data_tx: UnboundedSender<Packet>,
    state_tx: UnboundedSender<StateMessage>,
}

impl GgrsSocket {
    pub fn new(webrtc_socket: &mut WebRTCSocket) -> Self {
        let id = webrtc_socket.id;
        let in_data_rx = webrtc_socket.in_data_rx().unwrap();
        let out_data_tx = webrtc_socket.out_data_tx();
        let state_tx = webrtc_socket.state_tx();
        Self {
            id,
            in_data_rx,
            out_data_tx,
            state_tx,
        }
    }

    pub fn players(&self) -> Vec<PlayerType<Uuid>> {
        let (tx, mut rx) = mpsc::unbounded_channel::<Vec<Uuid>>();
        self.state_tx.send(StateMessage::ReadyPeers(tx)).unwrap();
        let mut ids = rx.blocking_recv().unwrap();
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

impl ggrs::NonBlockingSocket<Uuid> for GgrsSocket {
    fn send_to(&mut self, msg: &Message, addr: &Uuid) {
        let payload = bincode::serialize(&msg).unwrap();
        let payload = bytes::Bytes::from(payload);
        let packet = Packet { id: *addr, payload };
        let _ = self.out_data_tx.send(packet);
    }

    fn receive_all_messages(&mut self) -> Vec<(Uuid, Message)> {
        let incoming_messages = std::iter::repeat_with(move || self.in_data_rx.try_recv())
            .take_while(|p| !p.is_err())
            .map(|p| p.unwrap());

        let mut messages = vec![];
        for packet in incoming_messages {
            let msg = bincode::deserialize(&packet.payload).unwrap();
            messages.push((packet.id, msg));
        }
        messages
    }
}
