use serde_derive::Serialize;
#[derive(Serialize)]
struct ErrorMessage {
    error: String,
}
impl ErrorMessage {
    fn new(msg: impl std::fmt::Display) -> Self {
        Self {
            error: msg.to_string(),
        }
    }
}

pub fn handle_errors(
    rejection: warp::reject::Rejection,
) -> Result<impl warp::Reply, warp::reject::Rejection> {
    let err_txt = match rejection.cause() {
        Some(text) if text.to_string() == "Missing request header 'authorization'" => {
            "Error: Missing access token".to_string()
        }
        Some(text) => text.to_string(),
        None => "Unknown server error".to_string(),
    };
    let json = warp::reply::json(&ErrorMessage::new(err_txt));
    Ok(warp::reply::with_status(
        json,
        warp::http::StatusCode::UNAUTHORIZED,
    ))
}
