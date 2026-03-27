use unipotato::{Request, Response, handler::html, get};
use std::fs;

/// GET /template/
/// Serves `templates/index.html`, replacing `{{ title }}` at runtime.
#[get("/")]
pub async fn index(_req: Request) -> Response {
    let tmpl = fs::read_to_string("templates/index.html")
        .unwrap_or_else(|_| "<h1>Template not found</h1>".to_string());
    let rendered = tmpl.replace("{{ title }}", "Unipotato");
    html(rendered)
}

/// GET /training
/// Serves the training dashboard
#[get("/")]
pub async fn training_dashboard(_req: Request) -> Response {
    let tmpl = fs::read_to_string("templates/training-dashboard.html")
        .unwrap_or_else(|_| "<h1>Training dashboard not found</h1>".to_string());
    html(tmpl)
}
