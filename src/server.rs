use hyper::{Request, Response, body::Incoming, service::service_fn};
use hyper_util::rt::TokioIo;
use tokio::net::TcpListener;
use std::convert::Infallible;
use crate::{route::collect_routes, handler::not_found};

pub struct Server {
    port: u16,
}

impl Server {
    pub fn new(port: u16) -> Self {
        Self { port }
    }

    pub fn mount(self, base: &str, routes_fn: impl FnOnce()) -> Self {
        crate::route::mount(base, routes_fn);
        self
    }

    pub fn start(self) {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async move {
            println!("Starting server...");
            if let Err(e) = self.launch_internal().await {
                eprintln!("Failed to start server: {}", e);
                eprintln!("This might be because port {} is already in use.", self.port);
                eprintln!("Try killing any existing processes on port {}:", self.port);
                eprintln!("  lsof -ti:{} | xargs kill -9", self.port);
                std::process::exit(1);
            }
        });
    }

    async fn launch_internal(&self) -> Result<(), Box<dyn std::error::Error>> {
        let addr = format!("0.0.0.0:{}", self.port);
        let listener = TcpListener::bind(&addr).await?;
        println!("Unipotato listening on http://localhost:{}", self.port);
        loop {
            let (stream, _) = listener.accept().await?;
            let io = TokioIo::new(stream);
            tokio::task::spawn(async move {
                let service = service_fn(router);
                if let Err(err) = hyper::server::conn::http1::Builder::new()
                    .serve_connection(io, service)
                    .await 
                {
                    eprintln!("Error serving connection: {:?}", err);
                }
            });
        }
    }
}

async fn router(req: Request<Incoming>) -> Result<Response<String>, Infallible> {
    let routes = collect_routes();
    for route in routes {
        if req.method() == &route.method {
            if req.uri().path() == route.path.as_str() {
                return Ok((route.handler)(req).await);
            }
        }
    }
    Ok(not_found())
}