use std::fmt;

use actix_web::{delete, get, post, web, Error, HttpResponse, Responder, ResponseError};
use secrecy::Secret;
use serde_json::json;

use crate::db::{
    actions::{create_user, delete_user, find_user_by_name, list_users},
    DbPool,
};
use crate::middleware::Authentication;

#[derive(Debug, serde::Deserialize)]
pub struct UserData {
    username: String,
    pwd: String,
}

#[derive(Debug, thiserror::Error)]
pub struct R2d2Error(pub r2d2::Error);

impl fmt::Display for R2d2Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "")
    }
}
impl ResponseError for R2d2Error {}

#[post("/add")]
pub async fn useradd(
    form: web::Json<UserData>,
    pool: web::Data<DbPool>,
) -> Result<HttpResponse, Error> {
    let mut conn = pool.get().map_err(|e| R2d2Error(e))?;
    let username = &form.username;
    let pwd: Secret<String> = Secret::new(form.pwd.clone());
    create_user(username, pwd, &mut conn)?;
    Ok(HttpResponse::Ok().await?)
}

#[delete("/del/{username}", wrap = "Authentication")]
pub async fn userdel(
    username: web::Path<String>,
    pool: web::Data<DbPool>,
) -> Result<HttpResponse, Error> {
    let mut conn = pool.get().expect("Could not get DbConnection");
    let username = username.into_inner();
    delete_user(&username, &mut conn)?;

    Ok(HttpResponse::Ok().await?)
}

#[get("/{username}", wrap = "Authentication")]
pub async fn show(
    username: web::Path<String>,
    pool: web::Data<DbPool>,
) -> Result<impl Responder, Error> {
    let mut conn = pool.get().expect("Could not get DbConnection");
    let user = find_user_by_name(&username, &mut conn)?;
    let user = json!({"username": user.name});
    Ok(web::Json(user))
}

#[get("/users")]
pub async fn users(pool: web::Data<DbPool>) -> actix_web::Result<impl Responder> {
    let mut conn = pool.get().expect("Could not get DbConnection");
    let users = list_users(&mut conn)?
        .into_iter()
        .map(|u| u.name)
        .collect::<Vec<String>>();
    let users = serde_json::to_value(users).unwrap();
    Ok(web::Json(users))
}
