use std::{fmt::Display, sync::Arc, time::Duration};

use anyhow::anyhow;
pub use awc::ws;
use awc::{ws::Codec, BoxedSocket, ClientResponse};
use futures_util::{SinkExt, StreamExt};
use tracing::{error, info, trace};
use uuid::Uuid;
use webrtc::{
    api::{
        interceptor_registry::register_default_interceptors, media_engine::MediaEngine, APIBuilder,
    },
    data_channel::data_channel_message::DataChannelMessage,
    ice_transport::ice_server::RTCIceServer,
    interceptor::registry::Registry,
    peer_connection::{
        configuration::RTCConfiguration, math_rand_alpha,
        peer_connection_state::RTCPeerConnectionState, RTCPeerConnection,
    },
};

use crate::message::Message;

pub mod message;

#[derive(Debug)]
pub struct Client {
    address: String,
    rtc_config: RtcConfig,
}

impl Client {
    pub fn new<S: Display>(address: S, port: u16, rtc_config: RtcConfig) -> Self {
        let address = format!("ws://{}:{}/", address, port);

        Self {
            address,
            rtc_config,
        }
    }

    pub async fn establish_connection(
        &self,
        user: &str,
        password: Option<&str>,
    ) -> Result<(ClientResponse, actix_codec::Framed<BoxedSocket, Codec>), anyhow::Error> {
        awc::Client::new()
            .ws(self.address.clone())
            .basic_auth(user, password)
            .connect()
            .await
            .map_err(|e| anyhow::anyhow!("Client error: {}", e))
    }

    pub async fn connect(&self, user: &str, password: &str) -> Result<(), anyhow::Error> {
        let (_res, mut ws) = self.establish_connection(user, Some(password)).await?;

        // drop(_res);
        info!("Waiting for messages");
        while let Some(Ok(msg)) = ws.next().await {
            info!("Got message: {:?}", msg);
            match msg {
                ws::Frame::Text(msg) => {
                    let msg: Message = serde_json::from_slice(&msg)?;
                    match msg {
                        Message::NewPeer { id } => {
                            let pc =
                                PeerConnection::new(self.rtc_config.ice_servers.clone()).await?;
                        }
                        Message::Offer { id, offer } => todo!(),
                        Message::Answer { id, answer } => todo!(),
                    }
                }
                ws::Frame::Binary(_) => todo!(),
                ws::Frame::Continuation(_) => todo!(),
                ws::Frame::Ping(msg) => ws.send(ws::Message::Pong(msg)).await?,
                ws::Frame::Pong(_) => {}
                ws::Frame::Close(_) => ws.close().await?,
            }
        }
        Ok(())
    }
}

struct Peer {
    id: Uuid,
    connection: PeerConnection,
}

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

struct PeerConnection {
    peer_connection: Arc<RTCPeerConnection>,
}

impl PeerConnection {
    async fn new(ice_servers: Vec<RTCIceServer>) -> anyhow::Result<Self> {
        let mut m = MediaEngine::default();
        m.register_default_codecs()?;
        let mut registry = Registry::new();
        registry = register_default_interceptors(registry, &mut m)?;

        let api = APIBuilder::new()
            .with_media_engine(m)
            .with_interceptor_registry(registry)
            .build();
        let config = RTCConfiguration {
            ice_servers: vec![RTCIceServer {
                urls: vec!["stun:stun.stunprotocol.org:3478".to_owned()],
                ..Default::default()
            }],
            ..Default::default()
        };
        let peer_connection = Arc::new(api.new_peer_connection(config).await?);

        let (done_tx, mut done_rx) = tokio::sync::mpsc::channel::<()>(1);

        peer_connection
            .on_peer_connection_state_change(Box::new(move |s: RTCPeerConnectionState| {
                info!("Peer Connection State has changed: {}", s);

                if s == RTCPeerConnectionState::Failed {
                    // Wait until PeerConnection has had no network activity for 30 seconds or another failure. It may be reconnected using an ICE Restart.
                    // Use webrtc.PeerConnectionStateDisconnected if you are interested in detecting faster timeout.
                    // Note that the PeerConnection may come back from PeerConnectionStateDisconnected.
                    info!("Peer Connection has gone to failed exiting");
                    let _ = done_tx.try_send(());
                }

                Box::pin(async {})
            }))
            .await;

        // peer_connection.on_ice_candidate(|c| {}).await;

        Ok(Self { peer_connection })
    }

    async fn create_data_channel(&self) -> anyhow::Result<()> {
        let data_channel = self
            .peer_connection
            .create_data_channel("data", None)
            .await?;
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
        Ok(())
    }

    pub async fn offer(&self) -> anyhow::Result<String> {
        // Create an offer to send to the browser
        let offer = self.peer_connection.create_offer(None).await?;

        // Create channel that is blocked until ICE Gathering is complete
        let mut gather_complete = self.peer_connection.gathering_complete_promise().await;

        // Sets the LocalDescription, and starts our UDP listeners
        self.peer_connection.set_local_description(offer).await?;

        // Block until ICE Gathering is complete, disabling trickle ICE
        // we do this because we only can exchange one signaling message
        // in a production application you should exchange ICE Candidates via OnICECandidate
        let _ = gather_complete.recv().await;

        // Output the answer in base64 so we can paste it in browser
        if let Some(local_desc) = self.peer_connection.local_description().await {
            let json_str = serde_json::to_string(&local_desc)?;
            let b64 = encode(&json_str);
            trace!("Offer: {}", b64);
            Ok(b64)
        } else {
            error!("generate local_description failed!");
            Err(anyhow!("generate local_description failed!"))
        }

        // Wait for the answer to be pasted
        // let line = signal::must_read_stdin()?;
        // let desc_data = decode(line.as_str())?;
        // let answer = serde_json::from_str::<RTCSessionDescription>(&desc_data)?;

        // // Apply the answer as the remote description
        // peer_connection.set_remote_description(answer).await?;
    }
}

/// encode encodes the input in base64
/// It can optionally zip the input before encoding
pub fn encode(b: &str) -> String {
    //if COMPRESS {
    //    b = zip(b)
    //}

    base64::encode(b)
}

/// decode decodes the input from base64
/// It can optionally unzip the input after decoding
pub fn decode(s: &str) -> anyhow::Result<String> {
    let b = base64::decode(s)?;

    //if COMPRESS {
    //    b = unzip(b)
    //}

    let s = String::from_utf8(b)?;
    Ok(s)
}
