use unipotato::{Request, Response, handler::{json, text}, get};
use serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize)]
struct Status {
    ok: bool,
    app: String,
}

/// GET /
#[get("/")]
pub async fn index(_req: Request) -> Response {
    text("🥔 ml_server is running!")
}

/// GET /health
#[get("/health")]
pub async fn health(_req: Request) -> Response {
    json(Status { ok: true, app: "ml_server".to_string() })
}
