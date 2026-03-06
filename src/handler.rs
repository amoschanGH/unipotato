use hyper::StatusCode;

pub fn text(body: impl Into<String>) -> hyper::Response<String> {
    hyper::Response::builder()
        .header("Content-Type", "text/plain; charset=utf-8")
        .body(body.into())
        .unwrap()
}

pub fn html(body: impl Into<String>) -> hyper::Response<String> {
    hyper::Response::builder()
        .header("Content-Type", "text/html")
        .body(body.into())
        .unwrap()
}

pub fn json<T: serde::Serialize>(data: T) -> hyper::Response<String> {
    hyper::Response::builder()
        .header("Content-Type", "application/json")
        .body(serde_json::to_string(&data).unwrap())
        .unwrap()
}

pub fn not_found() -> hyper::Response<String> {
    hyper::Response::builder()
        .status(StatusCode::NOT_FOUND)
        .body("404 Not Found".to_string())
        .unwrap()
}