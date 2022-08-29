use uuid::Uuid;
use webrtc::peer_connection::sdp::session_description::RTCSessionDescription;

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub enum Message {
    Id(Uuid),
    NewPeer {
        id: Uuid,
    },
    Offer {
        id: Uuid,
        offer: RTCSessionDescription,
    },
    Answer {
        id: Uuid,
        answer: RTCSessionDescription,
    },
    IceCandidate {
        id: Uuid,
        candidate: String,
    },
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct PeerMessage {
    pub peer_id: Uuid,
    pub content: Message,
}
