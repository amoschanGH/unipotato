use unipotato::{handler, get, routes, Request, Response, Unipotato};

#[get("/health")]
pub async fn health(_req: Request) -> Response {
    handler::json(serde_json::json!({"ok": true, "service": "latency_health_server"}))
}

fn main() {
    println!("Starting latency health server on http://127.0.0.1:8080");
    Unipotato::launch(8080).mount("/", routes![self::health]).start();
}
