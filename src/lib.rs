pub mod macros;
pub mod route;
pub mod server;
pub mod handler;
pub mod logger;
pub mod request;
pub mod banner;

pub use route::{Route, mount};
pub use server::Unipotato;
pub use request::{Query, Body};

// Re-export common types so developers don't need to import from hyper
pub type Request = hyper::Request<hyper::body::Incoming>;
pub type Response = hyper::Response<String>;

// Re-export the proc-macro attributes
pub use unipotato_macros::{get, post, put, delete, patch};

// Re-export for proc-macros
pub mod export {
    pub use ctor;
}
