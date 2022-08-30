use tracing_subscriber::EnvFilter;

use matchmaker::{
    db::{create_pool, database_url_from_env},
    settings::{ApplicationSettings, Settings},
};

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
            port: 0,
        },
    };
    matchmaker::application::Application::build(settings, create_pool(database_url_from_env()))
        .await?
        .run_until_stopped()
        .await?;
    Ok(())
}
