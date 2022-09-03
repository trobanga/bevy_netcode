use std::collections::HashMap;

use anyhow::anyhow;
pub use awc::ws;
use awc::{ws::Codec, BoxedSocket, ClientResponse};
use futures_util::{SinkExt as _, StreamExt as _};
use message::{PeerMessage, StateMessage};
use peer::{Peer, RtcConfig};
use tokio::{
    select,
    sync::mpsc::{self, UnboundedReceiver, UnboundedSender},
};
use tracing::{debug, error, trace};
use uuid::Uuid;
use webrtc::{
    ice_transport::ice_candidate::RTCIceCandidateInit,
    peer_connection::sdp::session_description::RTCSessionDescription,
};

use crate::message::Message;

pub mod blocking;
pub mod ggrs_socket;
pub mod message;
pub mod peer;

pub use ggrs_socket::GgrsSocket;

pub type Payload = bytes::Bytes;

pub struct Packet {
    id: Uuid,
    payload: Payload,
}

pub struct WebRTCSocket {
    id: Uuid,
    rtc_config: RtcConfig,
    peers: HashMap<Uuid, Peer>,
    ws: actix_codec::Framed<BoxedSocket, Codec>,
    in_data_tx: UnboundedSender<Packet>,
    in_data_rx: Option<UnboundedReceiver<Packet>>,
    out_data_tx: UnboundedSender<Packet>,
    out_data_rx: UnboundedReceiver<Packet>,
    state_tx: UnboundedSender<StateMessage>,
    state_rx: UnboundedReceiver<StateMessage>,
}

impl std::fmt::Debug for WebRTCSocket {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Client")
            .field("id", &self.id)
            .field("rtc_config", &self.rtc_config)
            .field("peers", &self.peers)
            .finish()
    }
}

impl WebRTCSocket {
    pub async fn new(rtc_config: RtcConfig) -> anyhow::Result<Self> {
        let mut rtc_config = rtc_config;
        let (_res, mut ws) = WebRTCSocket::connect(&mut rtc_config).await?;
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
        let (state_tx, state_rx) = mpsc::unbounded_channel::<StateMessage>();
        Ok(Self {
            id,
            rtc_config,
            peers: Default::default(),
            ws,
            in_data_tx,
            in_data_rx: Some(in_data_rx),
            out_data_tx,
            out_data_rx,
            state_tx,
            state_rx,
        })
    }

    fn user(&self) -> &str {
        &self.rtc_config.user
    }

    pub async fn connect(
        rtc_config: &mut RtcConfig,
    ) -> Result<(ClientResponse, actix_codec::Framed<BoxedSocket, Codec>), anyhow::Error> {
        let password = rtc_config.take_password();
        awc::Client::new()
            .ws(rtc_config.login_url())
            .basic_auth(&rtc_config.user, password.as_deref())
            .connect()
            .await
            .map_err(|e| anyhow::anyhow!("Client error: {}", e))
    }

    pub async fn run(&mut self) -> Result<(), anyhow::Error> {
        debug!("WebRTC run() started");

        let (ws_tx, mut ws_rx) = mpsc::unbounded_channel::<PeerMessage>();
        loop {
            select! {
                Some(msg) = ws_rx.recv() => {
                    trace!(?msg);
                    self.send_text(serde_json::to_string(&msg).unwrap()).await?;
                }
                Some(Ok(msg)) = self.ws.next() => {
                    trace!(?msg);
                    match msg {
                        ws::Frame::Text(msg) => {
                            let msg: Message = serde_json::from_slice(&msg)?;
                            match msg {
                                Message::Id(_) => {}
                                Message::NewPeer { id } => self.new_peer(id, ws_tx.clone()).await?,
                                Message::PeerDisconnected { id } => {
                                    debug!("Received PeerDisconnected msg for: {id}");
                                    let _ = self.peers.remove(&id);
                                }
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
                        trace!("Send packet with {} bytes to peer {}", packet.payload.len(), packet.id);
                        peer.send(packet.payload).await?;
                    }
                }
                Some(msg) = self.state_rx.recv() => {
                    match msg {
                        StateMessage::ReadyPeers(tx) => {
                            let mut peers = vec![];

                            for (&id, peer) in self.peers.iter() {
                                if peer.ready().await {
                                    peers.push(id);
                                }
                            }
                            let _ = tx.send(peers);
                        }
                    }
                }
            }
        }
        Ok(())
    }

    pub fn state_tx(&self) -> UnboundedSender<StateMessage> {
        self.state_tx.clone()
    }

    pub fn out_data_tx(&self) -> UnboundedSender<Packet> {
        self.out_data_tx.clone()
    }

    pub fn in_data_rx(&mut self) -> Option<UnboundedReceiver<Packet>> {
        self.in_data_rx.take()
    }

    pub fn send_data(&mut self, packet: Packet) {
        let _ = self.out_data_tx.send(packet);
    }

    pub fn receive_data(&mut self) -> Option<impl IntoIterator<Item = Packet> + '_> {
        if let Some(in_data_rx) = &mut self.in_data_rx {
            Some(
                std::iter::repeat_with(move || in_data_rx.try_recv())
                    .take_while(|p| !p.is_err())
                    .map(|p| p.unwrap()),
            )
        } else {
            None
        }
    }

    async fn send_text(&mut self, msg: String) -> anyhow::Result<()> {
        Ok(self.ws.send(ws::Message::Text(msg.into())).await?)
    }

    async fn new_peer(
        &mut self,
        id: Uuid,
        tx: mpsc::UnboundedSender<PeerMessage>,
    ) -> anyhow::Result<()> {
        debug!("New peer with id: {id}");
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
        debug!("{} got offer from {id}. Offer is: {:?}", self.user(), offer);
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
        debug!(
            "{} got answer from {id}. Answer is: {answer:?}",
            self.user()
        );
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
