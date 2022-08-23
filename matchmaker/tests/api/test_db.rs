use diesel::{pg::Pg, sql_query, Connection, PgConnection, RunQueryDsl};
use diesel_migrations::{embed_migrations, EmbeddedMigrations, MigrationHarness};
use matchmaker::db::database_url_from_env;
use std::sync::atomic::AtomicU32;
use tracing::{debug, warn};
use url::Url;

static TEST_DB_COUNTER: AtomicU32 = AtomicU32::new(0);

pub const MIGRATIONS: EmbeddedMigrations = embed_migrations!();

#[derive(Debug)]
pub struct TestDb {
    default_db_url: String,
    url: String,
    name: String,
    delete_on_drop: bool,
}

impl TestDb {
    pub fn new() -> Self {
        let name = format!(
            "test_db_{}_{}",
            std::process::id(),
            TEST_DB_COUNTER.fetch_add(1, std::sync::atomic::Ordering::SeqCst)
        );
        let default_db_url = database_url_from_env();
        let mut conn = PgConnection::establish(&default_db_url).unwrap();
        sql_query(format!("CREATE DATABASE {};", name))
            .execute(&mut conn)
            .unwrap();
        let mut url = Url::parse(&default_db_url).unwrap();
        url.set_path(&name);
        Self {
            default_db_url,
            url: url.to_string(),
            name,
            delete_on_drop: true,
        }
    }

    pub fn url(&self) -> &str {
        &self.url
    }

    pub fn conn(&self) -> PgConnection {
        PgConnection::establish(self.url.as_str()).unwrap()
    }

    #[allow(dead_code)]
    pub fn leak(&mut self) {
        self.delete_on_drop = false;
    }

    pub fn run_migrations(
        &self,
        connection: &mut impl MigrationHarness<Pg>,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync + 'static>> {
        connection.run_pending_migrations(MIGRATIONS)?;
        Ok(())
    }
}

impl Drop for TestDb {
    fn drop(&mut self) {
        if !self.delete_on_drop {
            warn!("TestDb leaking database {}", self.name);
            return;
        }
        debug!("Dropping DB {}", self.name);
        let mut conn = PgConnection::establish(&self.default_db_url).unwrap();
        sql_query(format!(
            "SELECT pg_terminate_backend(pid) FROM pg_stat_activity WHERE datname = '{}'",
            self.name
        ))
        .execute(&mut conn)
        .unwrap();
        sql_query(format!("DROP DATABASE {}", self.name))
            .execute(&mut conn)
            .unwrap();
    }
}
