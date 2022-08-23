use crate::helper::spawn_app;
use futures_util::{SinkExt as _, StreamExt as _};

#[actix_web::test]
async fn spawn_test_app() {
    let app = spawn_app().await;
    let path = app.path("health_check");
    let response = reqwest::Client::new()
        .get(&path)
        .send()
        .await
        .expect("Failed to execute request.");
    assert_eq!(response.status(), 200);
}

#[actix_web::test]
async fn client_ping_pong() -> anyhow::Result<()> {
    let app = spawn_app().await;
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
