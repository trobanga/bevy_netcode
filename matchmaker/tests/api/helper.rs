use once_cell::sync::Lazy;

use matchmaker::{
    application,
    db::{self, actions::create_user, DbPool},
    settings::{ApplicationSettings, Settings},
};
use secrecy::Secret;

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
    pub users: Vec<TestUser>,
    #[allow(dead_code)]
    test_db: TestDb,
}

impl TestApp {
    pub fn new() -> Self {
        Lazy::force(&TRACING);

        let test_db = test_db::TestDb::new();
        let db_pool = db::create_pool(test_db.url());
        let mut conn = db_pool.get().unwrap();
        test_db.run_migrations(&mut conn).unwrap();
        Self {
            address: "127.0.0.1".to_string(),
            port: 0,
            db_pool,
            users: vec![],
            test_db,
        }
    }

    pub async fn spawn_app(&mut self) {
        let settings = Settings {
            application: ApplicationSettings {
                host: "127.0.0.1".to_string(),
                port: 0,
            },
        };
        let app = application::Application::build(settings, self.db_pool.clone())
            .await
            .expect("Failed to build application");
        self.port = app.port();
        let _ = tokio::spawn(app.run_until_stopped());
    }

    pub fn base_address(&self) -> String {
        format!("http://{}:{}", &self.address, self.port)
    }

    pub fn path(&self, path: &str) -> String {
        format!("{}/{}", &self.base_address(), path)
    }

    pub fn add_user(&mut self, name: &str, password: &str) {
        let user = TestUser::new(name, password);
        user.store(&self.db_pool);
        self.users.push(user);
    }

    pub fn set_users(&mut self, users: Vec<TestUser>) {
        self.users = users;
    }
}

#[derive(Default)]
pub struct TestAppBuilder {
    users: Vec<TestUser>,
}

impl TestAppBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn users(mut self, users: Vec<TestUser>) -> Self {
        self.users = users;
        self
    }

    pub fn build(self) -> TestApp {
        let mut app = TestApp::new();

        let users = if self.users.len() == 0 {
            let user = TestUser::default();
            vec![user]
        } else {
            self.users
        };
        for user in &users {
            user.store(&app.db_pool);
        }
        app.set_users(users);
        app
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
    pub fn new(name: &str, password: &str) -> Self {
        Self {
            name: name.to_string(),
            password: password.to_string(),
        }
    }

    pub fn store(&self, pool: &DbPool) {
        let mut conn = pool.get().expect("Could not get DbConnection");
        create_user(&self.name, Secret::new(self.password.clone()), &mut conn).unwrap();
    }
}
