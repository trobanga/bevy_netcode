use actix_web::web::Bytes;
use awc::ws;
use client::Client;
use futures_util::{SinkExt as _, StreamExt as _};
use tracing::{debug, info};
use tracing_subscriber::EnvFilter;

fn setup() {
    if std::env::var("RUST_LOG").is_err() {
        std::env::set_var("RUST_LOG", "debug")
    }
    tracing_subscriber::fmt::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .init();
}

#[actix_web::main]
async fn main() -> anyhow::Result<()> {
    setup();

    let client = Client {
        address: "ws://127.0.0.1:3648/".to_string(),
    };

    let (res, mut ws) = client.connect().await?;

    debug!(?res);

    info!("Ping");
    ws.send(ws::Message::Ping(Bytes::new())).await.unwrap();

    if let Some(msg) = ws.next().await {
        match msg {
            Ok(ws::Frame::Pong(_)) => {
                info!("Pong");
            }
            _ => {}
        }
    }
    Ok(())
}
