use hyper::{Response, StatusCode};

pub fn html(body: impl Into<String>) -> Response<String> {
    Response::builder().header("Content-Type", "text/html").body(body.into()).unwrap()
}

pub fn json<T: serde::Serialize>(data: T) -> Response<String> {
    Response::builder()
        .header("Content-Type", "application/json")
        .body(serde_json::to_string(&data).unwrap())
        .unwrap()
}

pub fn not_found() -> Response<String> {
    Response::builder().status(StatusCode::NOT_FOUND).body("404 Not Found".to_string()).unwrap()
}