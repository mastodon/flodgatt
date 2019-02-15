use crate::AppState;
use actix_web::{HttpRequest, Responder};

pub fn index(_req: HttpRequest<AppState>) -> impl Responder {
    "OMG! It works!"
}
