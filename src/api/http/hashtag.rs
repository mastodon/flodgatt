use crate::AppState;
use actix_web::{HttpRequest, Responder};

pub fn index(_req: HttpRequest<AppState>) -> impl Responder {
    "placeholder response from hashtag::index"
}

pub fn local(_req: HttpRequest<AppState>) -> impl Responder {
    "placeholder response from hashtag::local"
}
