use crate::AppState;
use actix_web::{
    error::Result,
    http::header::AUTHORIZATION,
    middleware::{Middleware, Started},
    HttpRequest, HttpResponse,
};

pub struct Auth;

impl Middleware<AppState> for Auth {
    fn start(&self, req: &HttpRequest<AppState>) -> Result<Started> {
        let res = req
            .headers()
            .get(AUTHORIZATION)
            .map(|bearer| Started::Done)
            .unwrap_or_else(|| Started::Response(HttpResponse::Unauthorized().finish()));

        Ok(res)
    }
}
