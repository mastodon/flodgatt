use actix::{Actor, StreamHandler};
use actix_web::{ws, HttpRequest, Responder};
use log::debug;

/// Define http actor
struct WebsocketActor;

impl Actor for WebsocketActor {
    type Context = ws::WebsocketContext<Self>;
}

/// Handler for ws::Message message
impl StreamHandler<ws::Message, ws::ProtocolError> for WebsocketActor {
    fn handle(&mut self, msg: ws::Message, ctx: &mut Self::Context) {
        debug!("Message {:?}", msg);

        match msg {
            ws::Message::Ping(msg) => ctx.pong(&msg),
            ws::Message::Text(text) => ctx.text(text),
            ws::Message::Binary(bin) => ctx.binary(bin),
            _ => (),
        }
    }
}

pub fn index(req: HttpRequest) -> impl Responder {
    ws::start(&req, WebsocketActor)
}
