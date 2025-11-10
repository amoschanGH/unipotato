use hyper::{Request, Response, body::Incoming, service::service_fn};
use hyper_util::rt::TokioIo;
use tokio::net::TcpListener;
use std::convert::Infallible;
use crate::{
    handler::not_found, 
    route::collect_routes,
    log_debug, log_error, log_info, log_request, log_response, log_success,
};

/// Main server struct that handles HTTP requests
pub struct Unipotato {
    port: u16,
}

impl Unipotato {
    /// Create a new server instance
    pub fn launch(port: u16) -> Self {
        Self { port }
    }

    /// Mount routes under a specific base path
    /// 
    /// # Example
    /// ```
    /// Server::new(8000)
    ///     .mount("/api", routes![...])
    ///     .mount("/admin", routes![...])
    /// ```
    pub fn mount<F>(self, base: &str, routes_fn: F) -> Self 
    where 
        F: FnOnce()
    {
        crate::route::mount(base, routes_fn);
        self
    }

    /// Start the server and begin listening for requests
    pub fn start(self) {
        crate::banner::print_banner();
        
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async move {
            log_info!("Starting Unipotato server...");
            
            if let Err(e) = self.launch_internal().await {
                self.handle_startup_error(e);
            }
        });
    }

    /// Handle startup errors with helpful messages
    fn handle_startup_error(&self, error: Box<dyn std::error::Error>) {
        log_error!("Failed to start server: {}", error);
        log_error!("This might be because port {} is already in use.", self.port);
        log_info!("Try killing any existing processes on port {}:", self.port);
        log_info!("  lsof -ti:{} | xargs kill -9", self.port);
        std::process::exit(1);
    }

    /// Internal method to launch the server and accept connections
    async fn launch_internal(&self) -> Result<(), Box<dyn std::error::Error>> {
        let addr = format!("0.0.0.0:{}", self.port);
        let listener = TcpListener::bind(&addr).await?;
        
        self.log_registered_routes();
        log_success!("Unipotato listening on http://localhost:{}", self.port);
        
        self.accept_connections(listener).await
    }

    /// Log all registered routes for debugging
    fn log_registered_routes(&self) {
        let routes = collect_routes();
        log_info!("Registered {} routes:", routes.len());
        
        for route in &routes {
            log_debug!("  {} {}", route.method, route.path);
        }
    }

    /// Accept and handle incoming connections
    async fn accept_connections(&self, listener: TcpListener) -> Result<(), Box<dyn std::error::Error>> {
        loop {
            let (stream, _) = listener.accept().await?;
            let io = TokioIo::new(stream);
            
            tokio::task::spawn(async move {
                if let Err(err) = Self::serve_connection(io).await {
                    log_error!("Error serving connection: {:?}", err);
                }
            });
        }
    }

    /// Serve a single HTTP connection
    async fn serve_connection(io: TokioIo<tokio::net::TcpStream>) -> Result<(), hyper::Error> {
        let service = service_fn(router);
        hyper::server::conn::http1::Builder::new()
            .serve_connection(io, service)
            .await
    }
}

/// Route incoming requests to registered handlers
async fn router(req: Request<Incoming>) -> Result<Response<String>, Infallible> {
    let method = req.method().clone();
    let path = req.uri().path().to_string();
    
    log_request!(method.as_str(), &path);
    
    let response = match find_matching_route(&req) {
        Some(handler) => handler(req).await,
        None => not_found(),
    };
    
    let status = response.status().as_u16();
    log_response!(status, &path);
    
    Ok(response)
}

/// Find a route handler that matches the request
fn find_matching_route(req: &Request<Incoming>) -> Option<crate::route::Handler> {
    let routes = collect_routes();
    
    for route in routes {
        if req.method() == &route.method && req.uri().path() == route.path.as_str() {
            return Some(route.handler);
        }
    }
    
    None
}