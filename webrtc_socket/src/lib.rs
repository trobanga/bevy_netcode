use std::collections::HashMap;

use anyhow::anyhow;
pub use awc::ws;
use awc::{ws::Codec, BoxedSocket, ClientResponse};
use futures_util::{SinkExt, StreamExt};
use message::PeerMessage;
use peer::{Peer, RtcConfig};
use tokio::{
    select,
    sync::mpsc::{self, UnboundedReceiver, UnboundedSender},
};
use tracing::{error, info};
use uuid::Uuid;
use webrtc::{
    ice_transport::ice_candidate::RTCIceCandidateInit,
    peer_connection::sdp::session_description::RTCSessionDescription,
};

use crate::message::Message;

mod ggrs_socket;
pub mod message;
pub mod peer;

pub type Payload = bytes::Bytes;
pub struct Packet {
    id: Uuid,
    payload: Payload,
}

pub struct WebRTCSocket {
    id: Uuid,
    #[allow(dead_code)]
    user: String,
    address: String,
    rtc_config: RtcConfig,
    peers: HashMap<Uuid, Peer>,
    ws: actix_codec::Framed<BoxedSocket, Codec>,
    in_data_tx: UnboundedSender<Packet>,
    in_data_rx: UnboundedReceiver<Packet>,
    out_data_tx: UnboundedSender<Packet>,
    out_data_rx: UnboundedReceiver<Packet>,
}

impl std::fmt::Debug for WebRTCSocket {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Client")
            .field("id", &self.id)
            .field("address", &self.address)
            .field("rtc_config", &self.rtc_config)
            .field("peers", &self.peers)
            .finish()
    }
}

impl WebRTCSocket {
    pub async fn new<S: AsRef<str>>(
        address: S,
        port: u16,
        rtc_config: RtcConfig,
        user: &str,
        password: &str,
    ) -> anyhow::Result<Self> {
        let address = format!("ws://{}:{}/", address.as_ref(), port);
        let (_res, mut ws) = WebRTCSocket::connect(&address, user, password).await?;
        let id = if let Some(Ok(ws::Frame::Text(msg))) = ws.next().await {
            let msg: Message = serde_json::from_slice(&msg)?;
            if let Message::Id(id) = msg {
                id
            } else {
                return Err(anyhow!("First message must be Id!"));
            }
        } else {
            return Err(anyhow!("Error with Ws connection!"));
        };
        let (in_data_tx, in_data_rx) = mpsc::unbounded_channel::<Packet>();
        let (out_data_tx, out_data_rx) = mpsc::unbounded_channel::<Packet>();
        Ok(Self {
            id,
            user: user.to_string(),
            address,
            rtc_config,
            peers: Default::default(),
            ws,
            in_data_tx,
            in_data_rx,
            out_data_tx,
            out_data_rx,
        })
    }

    pub async fn connect(
        address: &str,
        user: &str,
        password: &str,
    ) -> Result<(ClientResponse, actix_codec::Framed<BoxedSocket, Codec>), anyhow::Error> {
        awc::Client::new()
            .ws(address)
            .basic_auth(user, Some(password))
            .connect()
            .await
            .map_err(|e| anyhow::anyhow!("Client error: {}", e))
    }

    pub async fn run(&mut self) -> Result<(), anyhow::Error> {
        let (ws_tx, mut ws_rx) = mpsc::unbounded_channel::<PeerMessage>();
        loop {
            select! {
                Some(msg) = ws_rx.recv() => {
                    self.send_text(serde_json::to_string(&msg).unwrap()).await?;
                }
                Some(Ok(msg)) = self.ws.next() => {
                    match msg {
                        ws::Frame::Text(msg) => {
                            let msg: Message = serde_json::from_slice(&msg)?;
                            match msg {
                                Message::Id(_) => {}
                                Message::NewPeer { id } => self.new_peer(id, ws_tx.clone()).await?,
                                Message::Offer { id, offer } =>  self.handle_offer(id, offer, ws_tx.clone()).await?,
                                Message::Answer { id, answer } => self.handle_answer(id, answer).await?,
                                Message::IceCandidate { id, candidate } => self.handle_ice_candidate(id, candidate).await?,
                            }
                        }
                        ws::Frame::Close(_) => {
                            self.ws.close().await?;
                            break;
                        },
                        ws::Frame::Ping(msg) => self.ws.send(ws::Message::Pong(msg)).await?,
                        ws::Frame::Pong(_) => {}
                        ws::Frame::Binary(_) => todo!(),
                        ws::Frame::Continuation(_) => todo!(),
                    }
                }
                Some(packet) = self.out_data_rx.recv() => {
                    if let Some(peer) = self.peers.get(&packet.id) {
                        peer.send(packet.payload).await?;
                    }
                }
            }
        }
        Ok(())
    }

    pub fn send_data(&mut self, packet: Packet) {
        let _ = self.out_data_tx.send(packet);
    }

    pub fn receive_data(&mut self) -> impl IntoIterator<Item = Packet> + '_ {
        std::iter::repeat_with(|| self.in_data_rx.try_recv())
            .take_while(|p| !p.is_err())
            .map(|p| p.unwrap())
    }

    async fn send_text(&mut self, msg: String) -> anyhow::Result<()> {
        Ok(self.ws.send(ws::Message::Text(msg.into())).await?)
    }

    async fn new_peer(
        &mut self,
        id: Uuid,
        tx: mpsc::UnboundedSender<PeerMessage>,
    ) -> anyhow::Result<()> {
        info!("New peer with id: {id}");
        let peer = self.peers.entry(id).or_insert(
            Peer::new(self.id, id, &self.rtc_config, tx, self.in_data_tx.clone()).await?,
        );
        let offer = peer.handshake_offer().await?;
        self.send_text(serde_json::to_string(&offer).unwrap())
            .await?;
        Ok(())
    }

    async fn handle_offer(
        &mut self,
        id: Uuid,
        offer: RTCSessionDescription,
        tx: mpsc::UnboundedSender<PeerMessage>,
    ) -> anyhow::Result<()> {
        info!("I {}, got offer! {id} {:?}", self.user, offer);
        let peer = self.peers.entry(id).or_insert(
            Peer::new(self.id, id, &self.rtc_config, tx, self.in_data_tx.clone()).await?,
        );
        let answer = peer.handshake_accept(offer).await?;
        self.send_text(serde_json::to_string(&answer).unwrap())
            .await?;
        Ok(())
    }

    async fn handle_answer(
        &mut self,
        id: Uuid,
        answer: RTCSessionDescription,
    ) -> anyhow::Result<()> {
        info!("I {}, got answer {id} {answer:?}", self.user);
        if let Some(peer) = self.peers.get(&id) {
            peer.handle_answer(answer).await?;
        }
        Ok(())
    }

    async fn handle_ice_candidate(&self, id: Uuid, candidate: String) -> anyhow::Result<()> {
        if let Some(peer) = self.peers.get(&id) {
            let candidate = RTCIceCandidateInit {
                candidate,
                ..Default::default()
            };
            match peer.connection().add_ice_candidate(candidate).await {
                Ok(_) => {}
                Err(e) => error!(?e),
            }
        }
        Ok(())
    }
}
