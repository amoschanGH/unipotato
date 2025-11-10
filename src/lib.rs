pub mod macros;
pub mod route;
pub mod server;
pub mod handler;

pub use route::{Route, mount};
pub use server::Server;

// Re-export common types so developers don't need to import from hyper
pub type Request = hyper::Request<hyper::body::Incoming>;
pub type Response = hyper::Response<String>;