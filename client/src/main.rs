use client::{Client, RtcConfig};
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

    let mut client = Client::new("ws://127.0.0.1", 3648, RtcConfig::default());

    client.connect("", "").await?;

    Ok(())
}
