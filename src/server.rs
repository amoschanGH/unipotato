use hyper::{Request, Response, body::Incoming, service::service_fn};
use hyper_util::rt::TokioIo;
use tokio::net::TcpListener;
use std::convert::Infallible;
use crate::{route::collect_routes, handler::not_found};

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

pub async fn launch() -> Result<(), Box<dyn std::error::Error>> {
    let listener = TcpListener::bind("0.0.0.0:8000").await?;
    println!("Unipotato listening on http://localhost:8000");
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