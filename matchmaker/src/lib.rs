#[macro_use]
extern crate diesel;

use actix_web::rt::task::JoinHandle;

pub mod application;
mod authentication;
pub mod configuration;
mod db;

pub fn spawn_blocking_with_tracing<F, R>(f: F) -> JoinHandle<R>
where
    F: FnOnce() -> R + Send + 'static,
    R: Send + 'static,
{
    let current_span = tracing::Span::current();
    actix_web::rt::task::spawn_blocking(move || current_span.in_scope(f))
}
