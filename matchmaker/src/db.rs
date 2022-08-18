use diesel::pg::PgConnection;
use diesel::r2d2::ConnectionManager;
use diesel::{prelude::*, r2d2};
use dotenv::dotenv;
use std::env;

pub mod actions;
mod models;
mod schema;

pub type DbPool = r2d2::Pool<ConnectionManager<PgConnection>>;

pub fn create_pool() -> DbPool {
    dotenv().ok();
    let database_url = env::var("DATABASE_URL").expect("DATABASE_URL must be set");

    let manager = ConnectionManager::<PgConnection>::new(database_url);
    r2d2::Pool::builder()
        .build(manager)
        .expect("Failed to create db pool")
}

pub fn establish_connection() -> PgConnection {
    dotenv().ok();

    let database_url = env::var("DATABASE_URL").expect("DATABASE_URL must be set");
    PgConnection::establish(&database_url).expect(&format!("Error connecting to {}", database_url))
}
