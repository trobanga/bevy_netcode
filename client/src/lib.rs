pub use awc::ws;
use awc::{ws::Codec, BoxedSocket, ClientResponse};

#[derive(Debug)]
pub struct Client {
    pub address: String,
}

impl Client {
    pub async fn connect(
        &self,
        user: &str,
        password: Option<&str>,
    ) -> Result<(ClientResponse, actix_codec::Framed<BoxedSocket, Codec>), anyhow::Error> {
        Ok(awc::Client::new()
            .ws(self.address.clone())
            .basic_auth(user, password)
            .connect()
            .await
            .map_err(|e| anyhow::anyhow!("Client error: {}", e))?)
    }
}
