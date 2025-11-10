use hyper::{Request, Response, Method, body::Incoming};
use std::sync::{Arc, Mutex};
use once_cell::sync::Lazy;

/// Type alias for async request handlers
pub type Handler = Arc<
    dyn Fn(Request<Incoming>) -> std::pin::Pin<Box<dyn std::future::Future<Output = Response<String>> + Send>> 
    + Send 
    + Sync
>;

/// Represents a single route with method, path, and handler
pub struct Route {
    pub method: Method,
    pub path: String,
    pub handler: Handler,
}

impl Clone for Route {
    fn clone(&self) -> Self {
        Self {
            method: self.method.clone(),
            path: self.path.clone(),
            handler: Arc::clone(&self.handler),
        }
    }
}

/// Global storage for registered routes
static ROUTES: Lazy<Mutex<Vec<Route>>> = Lazy::new(|| Mutex::new(Vec::new()));

/// Thread-local storage for mount base path
thread_local! {
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
        
        if let Ok(mut routes) = ROUTES.lock() {
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
    
    ROUTES.lock().unwrap().push(Route {
        method,
        path: full_path,
        handler,
    });
}

/// Collect all registered routes
pub fn collect_routes() -> Vec<Route> {
    ROUTES.lock()
        .unwrap_or_else(|_| panic!("Failed to lock ROUTES mutex"))
        .clone()
}

// === Private Helper Functions ===

fn count_routes() -> usize {
    ROUTES.lock().unwrap().len()
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
    
    let mut routes = ROUTES.lock().unwrap();
    for i in routes_before..routes.len() {
        if !routes[i].path.starts_with(base) {
            routes[i].path = format!("{}{}", base, routes[i].path);
        }
    }
}
