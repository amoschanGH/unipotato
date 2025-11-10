use unipotato::{launch, handler::html, Route};
use hyper::{Request, Response, Method, body::Incoming};
use std::sync::Arc;

async fn index(_req: Request<Incoming>) -> Response<String> {
    html("Hello from Unipotato!")
}

async fn about(_req: Request<Incoming>) -> Response<String> {
    html("<h1>About</h1>")
}

#[tokio::main]
async fn main() {
    // Register routes
    let index_handler = Arc::new(|req: Request<Incoming>| {
        Box::pin(index(req)) as std::pin::Pin<Box<dyn std::future::Future<Output = Response<String>> + Send>>
    });
    
    let about_handler = Arc::new(|req: Request<Incoming>| {
        Box::pin(about(req)) as std::pin::Pin<Box<dyn std::future::Future<Output = Response<String>> + Send>>
    });
    
    Route::new(Method::GET, "/", index_handler);
    Route::new(Method::GET, "/about", about_handler);
    
    println!("Starting server...");
    if let Err(e) = launch().await {
        eprintln!("Failed to start server: {}", e);
        eprintln!("This might be because port 8000 is already in use.");
        eprintln!("Try killing any existing processes on port 8000:");
        eprintln!("  lsof -ti:8000 | xargs kill -9");
        std::process::exit(1);
    }
}