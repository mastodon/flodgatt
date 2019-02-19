use crate::{common::HEARTBEAT_INTERVAL_SECONDS, AppState};
use actix::{Actor, AsyncContext, StreamHandler};
use actix_redis::{Command, RespValue};
use actix_web::{ws, HttpRequest, Responder};
use log::{debug, info};
use std::time::Duration;

/// Define http actor
struct WebsocketActor;

impl Actor for WebsocketActor {
    type Context = ws::WebsocketContext<Self, AppState>;
}

/// Handler for ws::Message message
impl StreamHandler<ws::Message, ws::ProtocolError> for WebsocketActor {
    fn started(&mut self, ctx: &mut Self::Context) {
        ctx.run_interval(Duration::from_secs(HEARTBEAT_INTERVAL_SECONDS), |_, inner_ctx| {
            inner_ctx.ping("");
        });
    }

    fn handle(&mut self, msg: ws::Message, ctx: &mut Self::Context) {
        debug!("Message {:?}", msg);

        let red = ctx.state().redis.send(Command(RespValue::SimpleString("GET".into())));

        match msg {
            ws::Message::Pong(msg) => debug!("{}", msg),
            ws::Message::Text(text) => ctx.text(text),
            _ => (),
        }
    }
}

pub fn index(req: HttpRequest<AppState>) -> impl Responder {
    ws::start(&req, WebsocketActor)
}
