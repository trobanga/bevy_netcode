use std::sync::Arc;

use getset::Getters;
use tokio::sync::{mpsc, Mutex};
use tracing::{debug, info, warn};
use uuid::Uuid;
use webrtc::{
    api::{
        interceptor_registry::register_default_interceptors, media_engine::MediaEngine, APIBuilder,
    },
    data_channel::{
        data_channel_init::RTCDataChannelInit, data_channel_message::DataChannelMessage,
        RTCDataChannel,
    },
    interceptor::registry::Registry,
    peer_connection::{
        configuration::RTCConfiguration, peer_connection_state::RTCPeerConnectionState,
        sdp::session_description::RTCSessionDescription, RTCPeerConnection,
    },
};

use crate::{
    message::{Message, PeerMessage},
    Packet, Payload,
};

mod rtc_config;
pub use rtc_config::{RtcConfig, RtcConfigBuilder};

#[derive(Getters)]
pub struct Peer {
    id: Uuid,
    peer_id: Uuid,
    #[getset(get = "pub")]
    connection: Arc<RTCPeerConnection>,
    ws_tx: mpsc::UnboundedSender<PeerMessage>,
    outgoing_data_channel: Arc<RTCDataChannel>,
    incoming_data_tx: mpsc::UnboundedSender<Packet>,
    ready: Arc<Mutex<bool>>,
}

impl std::fmt::Debug for Peer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Peer")
            .field("peer_id", &self.peer_id)
            .finish()
    }
}

impl Peer {
    pub async fn new(
        id: Uuid,
        peer_id: Uuid,
        config: &RtcConfig,
        ws_tx: mpsc::UnboundedSender<PeerMessage>,
        incoming_data_tx: mpsc::UnboundedSender<Packet>,
    ) -> anyhow::Result<Self> {
        let connection = Self::create_peer_connection(config).await?;
        let ready = Arc::new(Mutex::new(false));
        let outgoing_data_channel = Self::create_data_channel(&connection, ready.clone()).await?;
        let peer = Self {
            id,
            peer_id,
            connection,
            ws_tx,
            outgoing_data_channel,
            incoming_data_tx,
            ready,
        };
        peer.ice_candidates().await?;
        peer.connect_incoming_data_channel().await?;
        Ok(peer)
    }

    async fn create_peer_connection(config: &RtcConfig) -> anyhow::Result<Arc<RTCPeerConnection>> {
        let mut m = MediaEngine::default();
        m.register_default_codecs()?;
        let mut registry = Registry::new();
        registry = register_default_interceptors(registry, &mut m)?;

        let api = APIBuilder::new()
            .with_media_engine(m)
            .with_interceptor_registry(registry)
            .build();
        let config = RTCConfiguration {
            ice_servers: config.ice_servers.clone(),
            ..Default::default()
        };
        let connection = Arc::new(api.new_peer_connection(config).await?);

        connection
            .on_peer_connection_state_change(Box::new(move |s: RTCPeerConnectionState| {
                info!("Peer Connection State has changed: {}", s);

                if s == RTCPeerConnectionState::Failed {
                    info!("Peer Connection has gone to failed state");
                }

                Box::pin(async {})
            }))
            .await;
        Ok(connection)
    }

    async fn ice_candidates(&self) -> anyhow::Result<()> {
        let id = self.id;
        let peer_id = self.peer_id;
        let tx = self.ws_tx.clone();
        self.connection
            .on_ice_candidate(Box::new(move |c| {
                let tx2 = tx.clone();
                Box::pin(async move {
                    if let Some(candidate) = c {
                        let candidate = candidate.to_json().await.unwrap().candidate;
                        info!(?candidate);
                        let msg = PeerMessage {
                            peer_id,
                            content: Message::IceCandidate { id, candidate },
                        };
                        tx2.send(msg).unwrap();
                    }
                })
            }))
            .await;
        Ok(())
    }

    async fn create_data_channel(
        connection: &RTCPeerConnection,
        ready: Arc<Mutex<bool>>,
    ) -> anyhow::Result<Arc<RTCDataChannel>> {
        let config = RTCDataChannelInit {
            ordered: Some(false),
            max_retransmits: Some(0),
            id: Some(0),
            ..Default::default()
        };

        let data_channel = connection.create_data_channel("data", Some(config)).await?;

        let ready2 = ready.clone();
        data_channel
            .on_open(Box::new(move || {
                Box::pin(async move {
                    *ready2.lock().await = true;
                })
            }))
            .await;

        // Register text message handling
        let d_label = data_channel.label().to_owned();
        data_channel
            .on_message(Box::new(move |msg: DataChannelMessage| {
                let msg_str = String::from_utf8(msg.data.to_vec()).unwrap();
                info!("Message from DataChannel '{}': '{}'", d_label, msg_str);
                Box::pin(async {})
            }))
            .await;

        data_channel
            .on_close(Box::new(move || {
                debug!("Data channel closed");
                Box::pin(async move {})
            }))
            .await;

        data_channel
            .on_error(Box::new(move |e| {
                warn!("Data channel error {:?}", e);
                Box::pin(async move {})
            }))
            .await;

        data_channel
            .on_buffered_amount_low(Box::new(move || {
                info!("Buffered amound low");
                Box::pin(async move {})
            }))
            .await;
        Ok(data_channel)
    }

    async fn connect_incoming_data_channel(&self) -> anyhow::Result<()> {
        let tx = self.incoming_data_tx.clone();
        let id = self.peer_id;
        let ready = self.ready.clone();
        self.connection
            .on_data_channel(Box::new(move |channel| {
                let tx2 = tx.clone();
                let ready2 = ready.clone();
                Box::pin(async move {
                    channel
                        .on_open(Box::new(move || {
                            Box::pin(async move {
                                *ready2.lock().await = true;
                            })
                        }))
                        .await;
                    channel
                        .on_message(Box::new(move |msg: DataChannelMessage| {
                            let payload: Payload = msg.data;
                            let packet = Packet { id, payload };
                            let _ = tx2.send(packet);
                            Box::pin(async {})
                        }))
                        .await;
                })
            }))
            .await;

        Ok(())
    }

    pub async fn ready(&self) -> bool {
        *self.ready.lock().await
    }

    pub async fn handshake_offer(&self) -> anyhow::Result<PeerMessage> {
        let offer = self.create_offer().await?;
        Ok(PeerMessage {
            peer_id: self.peer_id,
            content: Message::Offer { id: self.id, offer },
        })
    }

    pub async fn handshake_accept(
        &self,
        offer: RTCSessionDescription,
    ) -> anyhow::Result<PeerMessage> {
        self.connection.set_remote_description(offer).await?;
        let answer = self.connection.create_answer(None).await?;
        self.connection
            .set_local_description(answer.clone())
            .await?;
        Ok(PeerMessage {
            peer_id: self.peer_id,
            content: Message::Answer {
                id: self.id,
                answer,
            },
        })
    }

    pub async fn handle_answer(&self, answer: RTCSessionDescription) -> anyhow::Result<()> {
        self.connection.set_remote_description(answer).await?;
        Ok(())
    }

    pub async fn create_offer(&self) -> anyhow::Result<RTCSessionDescription> {
        let offer = self.connection.create_offer(None).await?;
        self.connection.set_local_description(offer).await?;

        self.connection
            .local_description()
            .await
            .ok_or_else(|| anyhow::anyhow!("generate local_description failed!"))
    }

    pub async fn send(&self, payload: Payload) -> anyhow::Result<()> {
        self.outgoing_data_channel.send(&payload).await?;
        Ok(())
    }
}
