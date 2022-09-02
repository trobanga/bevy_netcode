use crate::helper::{enable_tracing, TestAppBuilder, TestUser};
use futures_util::{SinkExt as _, StreamExt as _};
use matchmaker::db::actions::display_users;
use tokio::time::{sleep, Duration};
use tracing::info;
use webrtc_socket::{peer::RtcConfigBuilder, WebRTCSocket};

#[actix_web::test]
async fn client_ping_pong() -> anyhow::Result<()> {
    let mut app = TestAppBuilder::new().with_default_user_alice().build();
    app.spawn_app().await;
    let mut rtc_config = RtcConfigBuilder::new()
        .address(app.address)
        .port(app.port)
        .user("Alice")
        .password("I like Bob")
        .build();
    let (_res, mut ws) = WebRTCSocket::connect(&mut rtc_config).await?;

    let mut got_pong = false;
    ws.send(webrtc_socket::ws::Message::Ping(
        actix_web::web::Bytes::new(),
    ))
    .await
    .unwrap();

    let _ = ws.next().await; // ignore first message with Id

    if let Some(msg) = ws.next().await {
        match msg {
            Ok(webrtc_socket::ws::Frame::Pong(_)) => {
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
    enable_tracing();
    let alice = TestUser::new("Alice", "I like Bob");
    let bob = TestUser::new("Bob", "I fancy Alice");
    let charlie = TestUser::new("Charlie", "Charlie loves Charlie");
    let mut app = TestAppBuilder::new()
        .users(vec![alice, bob, charlie])
        .build();
    app.spawn_app().await;

    let mut conn = app.db_pool.get().unwrap();
    info!("Users:");
    display_users(&mut conn).unwrap();

    let rtc_config = RtcConfigBuilder::new()
        .address(app.address.clone())
        .port(app.port)
        .user("Alice")
        .password("I like Bob")
        .build();
    tokio::task::spawn_local(async move {
        let mut alice = WebRTCSocket::new(rtc_config).await?;
        alice.run().await
    });

    sleep(Duration::from_millis(100)).await;
    let rtc_config = RtcConfigBuilder::new()
        .address(app.address)
        .port(app.port)
        .user("Bob")
        .password("I fancy Alice")
        .build();
    let _bob = tokio::task::spawn_local(async move {
        let mut bob = WebRTCSocket::new(rtc_config).await?;
        bob.run().await
    });

    // bob.await.unwrap().unwrap();

    // let caddress = app.address.clone();
    // let cport = app.port;
    // sleep(Duration::from_millis(100)).await;
    // tokio::task::spawn_local(async move {
    //     let mut charlie = webrtc_socket::webrtc_socket::new(
    //         caddress,
    //         cport,
    //         RtcConfig::default(),
    //         "Charlie",
    //         "Charlie loves Charlie",
    //     )
    //     .await?;
    //     charlie.run().await
    // });

    // don't close too early
    sleep(Duration::from_millis(15000)).await;
}
