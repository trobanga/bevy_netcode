use once_cell::sync::Lazy;

use matchmaker::{
    application,
    db::{self, actions::create_user, DbPool},
    settings::{ApplicationSettings, Settings},
};
use secrecy::Secret;
use tracing::info;

use crate::test_db::{self, TestDb};

static TRACING: Lazy<()> = Lazy::new(|| {
    if std::env::var("RUST_LOG").is_err() {
        std::env::set_var("RUST_LOG", "debug")
    }
    tracing_subscriber::fmt::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();
});

pub fn enable_tracing() {
    Lazy::force(&TRACING);
}

pub struct TestApp {
    pub address: String,
    pub port: u16,
    pub db_pool: DbPool,
    pub user: TestUser,
    #[allow(dead_code)]
    test_db: TestDb,
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

    let test_db = test_db::TestDb::new();
    info!(?test_db);
    let db_pool = db::create_pool(test_db.url());
    let mut conn = db_pool.get().unwrap();
    test_db.run_migrations(&mut conn).unwrap();

    let app = application::Application::build(settings, db_pool.clone())
        .await
        .expect("Failed to build application");
    let port = app.port();
    let _ = tokio::spawn(app.run_until_stopped());

    let user = TestUser::default();
    user.store(&db_pool);

    TestApp {
        address: "127.0.0.1".to_string(),
        port,
        db_pool,
        user,
        test_db,
    }
}

pub struct TestUser {
    name: String,
    password: String,
}

impl Default for TestUser {
    fn default() -> Self {
        Self {
            name: "Alice".to_string(),
            password: "I like Bob".to_string(),
        }
    }
}

impl TestUser {
    pub fn store(&self, pool: &DbPool) {
        let mut conn = pool.get().expect("Could not get DbConnection");
        create_user(&self.name, Secret::new(self.password.clone()), &mut conn).unwrap();
    }
}
