use actix_web::{HttpRequest, Responder};

pub fn index(_req: HttpRequest) -> impl Responder {
    "OMG! WebSocket works!"
}
