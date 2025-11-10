use unipotato::{Request, Response, handler::html, get, routes, launch};

async fn index(_req: Request) -> Response {
    html("Hello from Unipotato!")
}

async fn about(_req: Request) -> Response {
    html("<h1>About</h1>")
}

fn main() {
    launch!(8000)
        .mount("/", || {
            routes![
                get("/", index),
                get("/about", about),
            ];
        })
        .start();
}