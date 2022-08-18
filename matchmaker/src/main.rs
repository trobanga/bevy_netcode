use tracing_subscriber::EnvFilter;

use matchmaker::configuration::{ApplicationSettings, Settings};

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
    let settings = Settings {
        application: ApplicationSettings {
            host: "127.0.0.1".to_string(),
            port: 3648,
        },
    };
    matchmaker::application::Application::build(settings)
        .await?
        .run_until_stopped()
        .await?;
    Ok(())
}
