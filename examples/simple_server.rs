use unipotato::{handler::html, get, routes, mount, launch};
use hyper::{Request, Response, body::Incoming};

async fn index(_req: Request<Incoming>) -> Response<String> {
    html("Hello from Unipotato!")
}

async fn about(_req: Request<Incoming>) -> Response<String> {
    html("<h1>About</h1>")
}

fn main() {
    mount("/", || {
        routes![
            get("/", index),
            get("/about", about),
        ];
    });
    
    launch!();
}