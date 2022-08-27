use uuid::Uuid;

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub enum Message {
    NewPeer { id: Uuid },
    Offer { id: Uuid, offer: String },
    Answer { id: Uuid, answer: String },
}
