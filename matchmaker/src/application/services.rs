use std::{cell::RefCell, rc::Rc};

use actix::*;
use actix_web::{
    get,
    web::{self, ReqData},
    Error, HttpRequest, HttpResponse,
};
use uuid::Uuid;

use super::{client, moderator::Moderator};

mod user;
pub use user::*;

#[get("/health_check")]
async fn health_check() -> HttpResponse {
    HttpResponse::Ok().finish()
}

#[get("/login")]
async fn login(
    req: HttpRequest,
    user_id: ReqData<Rc<RefCell<Option<Uuid>>>>,
    stream: web::Payload,
    moderator: web::Data<Addr<Moderator>>,
) -> Result<HttpResponse, Error> {
    let user_id = user_id.borrow().unwrap();
    let websocket = client::WsClient::new(user_id, moderator.get_ref().clone());
    client::start(websocket, &req, stream)
}
