pub mod macros;
pub mod route;
pub mod server;
pub mod handler;
pub mod logger;
pub mod request;
pub mod banner;

pub use route::{Route, mount, register_route, Router, Handler, collect_routes, find_handler_with_params, HandlerMatch};
pub use server::Unipotato;
pub use request::{Query, Body, Request};

// Type alias for backward compatibility with macro tests
pub type Server = Unipotato;

// Re-export Response type
pub type Response = hyper::Response<String>;

// Re-export the proc-macro attributes
pub use unipotato_macros::{get, post, put, delete, patch};

// Re-export internal crates so macros and user code can reach them without
// declaring these as direct dependencies.
pub use hyper;
pub use paste;

// Re-export for proc-macros
pub mod export {
    pub use ctor;
}
