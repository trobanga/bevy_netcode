use client::Client;
use futures_util::{SinkExt as _, StreamExt as _};
use once_cell::sync::Lazy;

use matchmaker::{
    application,
    configuration::{ApplicationSettings, Settings},
};

static TRACING: Lazy<()> = Lazy::new(|| {
    if std::env::var("RUST_LOG").is_err() {
        std::env::set_var("RUST_LOG", "debug")
    }
    tracing_subscriber::fmt::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();
});

pub struct TestApp {
    pub address: String,
    pub port: u16,
}

impl TestApp {
    pub fn base_address(&self) -> String {
        format!("http://{}:{}", &self.address, self.port)
    }

    pub fn path(&self, path: &str) -> String {
        format!("{}/{}", &self.base_address(), path)
    }
}

pub async fn spawn_app() -> TestApp {
    Lazy::force(&TRACING);
    let settings = Settings {
        application: ApplicationSettings {
            host: "127.0.0.1".to_string(),
            port: 0,
        },
    };
    let app = application::Application::build(settings)
        .await
        .expect("Failed to build application");
    let port = app.port();
    let _ = tokio::spawn(app.run_until_stopped());

    TestApp {
        address: "127.0.0.1".to_string(),
        port,
    }
}

#[actix_web::test]
async fn spawn_test_app() {
    let test_app = spawn_app().await;
    let path = test_app.path("health_check");
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
    let client = Client { address };
    let (_res, mut ws) = client.connect().await?;

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
