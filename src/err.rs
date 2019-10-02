use serde_derive::Serialize;
use std::fmt::Display;

pub fn die_with_msg(msg: impl Display) -> ! {
    eprintln!("FATAL ERROR: {}", msg);
    std::process::exit(1);
}
pub fn unwrap_or_die(s: Option<impl Display>, msg: &str) -> String {
    s.unwrap_or_else(|| {
        eprintln!("FATAL ERROR: {}", msg);
        std::process::exit(1)
    })
    .to_string()
}

#[derive(Serialize)]
pub struct ErrorMessage {
    error: String,
}
impl ErrorMessage {
    fn new(msg: impl std::fmt::Display) -> Self {
        Self {
            error: msg.to_string(),
        }
    }
}

/// Recover from Errors by sending appropriate Warp::Rejections
pub fn handle_errors(
    rejection: warp::reject::Rejection,
) -> Result<impl warp::Reply, warp::reject::Rejection> {
    let err_txt = match rejection.cause() {
        Some(text) if text.to_string() == "Missing request header 'authorization'" => {
            "Error: Missing access token".to_string()
        }
        Some(text) => text.to_string(),
        None => "Error: Nonexistant endpoint".to_string(),
    };
    let json = warp::reply::json(&ErrorMessage::new(err_txt));

    Ok(warp::reply::with_status(
        json,
        warp::http::StatusCode::UNAUTHORIZED,
    ))
}

pub struct CustomError {}

impl CustomError {
    pub fn unauthorized_list() -> warp::reject::Rejection {
        warp::reject::custom("Error: Access to list not authorized")
    }
}
