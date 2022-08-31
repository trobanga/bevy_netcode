use actix::*;
use actix_web::{get, post, web, Error, HttpRequest, HttpResponse, Responder};
use secrecy::Secret;
use serde_json::json;

use crate::{
    authentication::basic_authentication,
    db::{
        actions::{create_user, find_user_by_name},
        DbPool,
    },
};

use super::{client, moderator::Moderator};

#[get("/health_check")]
async fn health_check() -> HttpResponse {
    HttpResponse::Ok().finish()
}

#[get("/")]
async fn index(
    req: HttpRequest,
    stream: web::Payload,
    pool: web::Data<DbPool>,
    moderator: web::Data<Addr<Moderator>>,
) -> Result<HttpResponse, Error> {
    let mut conn = pool.get().expect("Could not get DbConnection");
    let id = basic_authentication(req.headers(), &mut conn).await?;
    let websocket = client::WsClient::new(id, moderator.get_ref().clone());
    client::start(websocket, &req, stream)
}

#[derive(Debug, serde::Deserialize)]
struct FormData {
    username: String,
    pwd: String,
}

#[post("/add_user")]
async fn add_user(
    form: web::Json<FormData>,
    pool: web::Data<DbPool>,
) -> Result<HttpResponse, Error> {
    let mut conn = pool.get().expect("Could not get DbConnection");
    let username = &form.username;
    let pwd: Secret<String> = Secret::new(form.pwd.clone());
    create_user(username, pwd, &mut conn)?;
    Ok(HttpResponse::Ok().await?)
}

#[get("/user/{username}")]
async fn user(
    username: web::Path<String>,
    pool: web::Data<DbPool>,
) -> Result<impl Responder, Error> {
    let mut conn = pool.get().expect("Could not get DbConnection");
    let user = find_user_by_name(&username, &mut conn)?;
    let user = json!({"username": user.name});
    Ok(web::Json(user))
}

// #[get("/users")]
// async fn users(pool: web::Data<DbPool>) -> actix_web::Result<impl Responder> {
//     todo!()
// }
