use std::{sync::Arc, time::Duration};

use getset::Getters;
use tokio::sync::mpsc;
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
    ice_transport::ice_server::RTCIceServer,
    interceptor::registry::Registry,
    peer_connection::{
        configuration::RTCConfiguration, math_rand_alpha,
        peer_connection_state::RTCPeerConnectionState,
        sdp::session_description::RTCSessionDescription, RTCPeerConnection,
    },
};

use crate::message::{Message, PeerMessage};

#[derive(Debug)]
pub struct RtcConfig {
    ice_servers: Vec<RTCIceServer>,
}

impl Default for RtcConfig {
    fn default() -> Self {
        let ice_servers = vec![RTCIceServer {
            urls: vec!["stun:stun.stunprotocol.org:3478".to_owned()],
            ..Default::default()
        }];
        Self { ice_servers }
    }
}

#[derive(Getters)]
pub struct Peer {
    id: Uuid,
    peer_id: Uuid,
    #[getset(get = "pub")]
    connection: Arc<RTCPeerConnection>,
    tx: mpsc::UnboundedSender<PeerMessage>,
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
        tx: mpsc::UnboundedSender<PeerMessage>,
    ) -> anyhow::Result<Self> {
        let connection = Self::create_peer_connection(config).await?;
        Self::create_data_channel(&connection).await?;
        let peer = Self {
            id,
            peer_id,
            connection,
            tx,
        };
        peer.ice_candidates().await?;
        Ok(peer)
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
        // self.ice_candidates().await?;
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
        // self.ice_candidates().await?;
        Ok(())
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
        let tx = self.tx.clone();
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

    async fn create_data_channel(connection: &RTCPeerConnection) -> anyhow::Result<()> {
        let config = RTCDataChannelInit {
            ordered: Some(false),
            max_retransmits: Some(0),
            id: Some(0),
            ..Default::default()
        };

        let data_channel = connection.create_data_channel("data", Some(config)).await?;
        // Register channel opening handling
        let d1 = Arc::clone(&data_channel);
        data_channel.on_open(Box::new(move || {
        info!("Data channel '{}'-'{}' open. Random messages will now be sent to any connected DataChannels every 5 seconds", d1.label(), d1.id());

        let d2 = Arc::clone(&d1);
        Box::pin(async move {
            let mut result = anyhow::Result::<usize>::Ok(0);
            while result.is_ok() {
                let timeout = tokio::time::sleep(Duration::from_secs(5));
                tokio::pin!(timeout);

                tokio::select! {
                    _ = timeout.as_mut() =>{
                        let message = math_rand_alpha(15);
                        info!("Sending '{}'", message);
                        result = d2.send_text(message).await.map_err(Into::into);
                    }
                };
            }
        })
        })).await;

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
                // TODO: handle this somehow
                debug!("Data channel closed");
                Box::pin(async move {})
            }))
            .await;

        data_channel
            .on_error(Box::new(move |e| {
                // TODO: handle this somehow
                warn!("Data channel error {:?}", e);
                Box::pin(async move {})
            }))
            .await;

        data_channel
            .on_buffered_amount_low(Box::new(move || {
                info!("************* Buffered amound low");
                Box::pin(async move {})
            }))
            .await;
        Ok(())
    }

    pub async fn create_offer(&self) -> anyhow::Result<RTCSessionDescription> {
        let offer = self.connection.create_offer(None).await?;
        self.connection.set_local_description(offer).await?;

        Ok(self
            .connection
            .local_description()
            .await
            .ok_or_else(|| anyhow::anyhow!("generate local_description failed!"))?)
    }
}
