use hyper::{Request, Response, Method, body::Incoming};
use std::sync::{Arc, Mutex, OnceLock};
use std::pin::Pin;
use std::future::Future;

/// Type alias for async request handlers
pub type Handler = Arc<
    dyn Fn(Request<Incoming>) -> Pin<Box<dyn Future<Output = Response<String>> + Send>> 
    + Send 
    + Sync
>;

// Alias for macro compatibility
pub type RouteHandler = Handler;

/// Represents a single route with method, path, and handler
#[derive(Clone)]
pub struct Route {
    pub method: Method,
    pub path: String,
    pub handler: Handler,
}

/// Global storage for registered routes
fn routes_storage() -> &'static Mutex<Vec<Route>> {
    static ROUTES: OnceLock<Mutex<Vec<Route>>> = OnceLock::new();
    ROUTES.get_or_init(|| Mutex::new(Vec::new()))
}

thread_local! {
    /// Thread-local storage for mount base path
    static MOUNT_BASE: std::cell::RefCell<String> = std::cell::RefCell::new(String::new());
}

impl Route {
    /// Create and register a new route
    pub fn new(method: Method, path: impl Into<String>, handler: Handler) {
        let route = Self { 
            method, 
            path: path.into(), 
            handler 
        };
        
        if let Ok(mut routes) = routes_storage().lock() {
            routes.push(route);
        }
    }
}

/// Mount routes under a base path
/// 
/// # Arguments
/// * `base` - The base path prefix (e.g., "/api")
/// * `routes_fn` - Closure that registers routes
pub fn mount(base: &str, routes_fn: impl FnOnce()) {
    let base = base.trim_end_matches('/');
    let routes_before = count_routes();
    
    set_mount_base(base);
    routes_fn();
    clear_mount_base();
    
    apply_base_to_new_routes(base, routes_before);
}

/// Get the current mount base path
pub fn get_mount_base() -> String {
    MOUNT_BASE.with(|mb| mb.borrow().clone())
}

/// Register a new route with the current mount base
pub fn register_route(method: Method, path: String, handler: Handler) {
    let base = get_mount_base();
    let full_path = build_full_path(&base, &path);
    
    routes_storage().lock().unwrap().push(Route {
        method,
        path: full_path,
        handler,
    });
}

/// Collect all registered routes
pub fn collect_routes() -> Vec<Route> {
    routes_storage().lock()
        .unwrap_or_else(|_| panic!("Failed to lock ROUTES mutex"))
        .clone()
}

// === Private Helper Functions ===

fn count_routes() -> usize {
    routes_storage().lock().unwrap().len()
}

fn set_mount_base(base: &str) {
    MOUNT_BASE.with(|mb| {
        *mb.borrow_mut() = base.to_string();
    });
}

fn clear_mount_base() {
    MOUNT_BASE.with(|mb| {
        mb.borrow_mut().clear();
    });
}

fn build_full_path(base: &str, path: &str) -> String {
    if base.is_empty() {
        path.to_string()
    } else {
        format!("{}{}", base, path)
    }
}

fn apply_base_to_new_routes(base: &str, routes_before: usize) {
    if base.is_empty() {
        return;
    }
    
    let mut routes = routes_storage().lock().unwrap();
    for i in routes_before..routes.len() {
        if !routes[i].path.starts_with(base) {
            routes[i].path = format!("{}{}", base, routes[i].path);
        }
    }
}
