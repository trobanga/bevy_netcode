use crate::helper::{TestAppBuilder, TestUser};
use futures_util::{SinkExt as _, StreamExt as _};
use tracing::info;

#[actix_web::test]
async fn client_ping_pong() -> anyhow::Result<()> {
    let mut app = TestAppBuilder::new().build();
    app.spawn_app().await;
    let address = format!("ws://{}:{}/", &app.address, app.port);
    let client = client::Client { address };
    let (_res, mut ws) = client.connect("Alice", Some("I like Bob")).await?;

    let mut got_pong = false;
    ws.send(client::ws::Message::Ping(actix_web::web::Bytes::new()))
        .await
        .unwrap();
    if let Some(msg) = ws.next().await {
        match msg {
            Ok(client::ws::Frame::Pong(_)) => {
                got_pong = true;
            }
            _ => {}
        }
    }
    assert!(got_pong);
    Ok(())
}

#[actix_web::test]
async fn ws() {
    let alice = TestUser::new("Alice", "I like Bob");
    let bob = TestUser::new("Bob", "I like Alice");
    let mut app = TestAppBuilder::new().users(vec![alice, bob]).build();
    app.spawn_app().await;

    let address = format!("ws://{}:{}/", &app.address, app.port);
    let client = client::Client { address };
    let (_res, mut ws) = client.connect("Alice", Some("I like Bob")).await.unwrap();

    ws.send(client::ws::Message::Text("Hello".into()))
        .await
        .unwrap();
    let response = ws.next().await.unwrap().unwrap();
    info!(?response);
}
