use diesel::pg::PgConnection;
use diesel::r2d2::ConnectionManager;
use diesel::{prelude::*, r2d2};
use dotenv::dotenv;
use std::env;

pub mod actions;
mod models;
mod schema;

pub type DbPool = r2d2::Pool<ConnectionManager<PgConnection>>;
pub type DbConnection = PgConnection;

pub fn database_url_from_env() -> String {
    dotenv().ok();
    env::var("DATABASE_URL").expect("DATABASE_URL must be set")
}

pub fn create_pool<S: AsRef<str>>(database_url: S) -> DbPool {
    let manager = ConnectionManager::<DbConnection>::new(database_url.as_ref());
    r2d2::Pool::builder()
        .build(manager)
        .expect("Failed to create db pool")
}

pub fn establish_connection<S: AsRef<str>>(database_url: S) -> DbConnection {
    PgConnection::establish(database_url.as_ref())
        .expect(&format!("Error connecting to {}", database_url.as_ref()))
}
