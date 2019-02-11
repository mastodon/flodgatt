use actix_web::{HttpRequest, Responder};

pub fn index(_req: HttpRequest) -> impl Responder {
    "OMG! It works!"
}

pub fn local(_req: HttpRequest) -> impl Responder {
    "OMG! It works!"
}
