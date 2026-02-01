use hyper::{Request, Response, Method, body::Incoming, StatusCode};
use std::sync::{Arc, Mutex, OnceLock};
use std::collections::HashMap;
use regex::Regex;

/// Type alias for async request handlers
pub type Handler = Arc<
    dyn Fn(crate::Request) -> std::pin::Pin<Box<dyn std::future::Future<Output = crate::Response> + Send>> 
    + Send 
    + Sync
>;

// Alias for macro compatibility
pub type RouteHandler = Handler;

/// Public route information for external use
pub struct Route {
    pub method: Method,
    pub path: String,
    pub handler: Handler,
}

struct RouteInfo {
    method: Method,
    original_path: String,
    pattern: Regex,
    handler: Handler,
}

/// Global storage for registered routes
static ROUTES: OnceLock<Mutex<HashMap<Method, Vec<RouteInfo>>>> = OnceLock::new();

fn get_routes() -> &'static Mutex<HashMap<Method, Vec<RouteInfo>>> {
    ROUTES.get_or_init(|| Mutex::new(HashMap::new()))
}

thread_local! {
    /// Thread-local storage for mount base path
    static MOUNT_BASE: std::cell::RefCell<String> = std::cell::RefCell::new(String::new());
}

/// Register a new route with pattern matching support
pub fn register_route(method: Method, original_path: String, pattern: String, handler: Handler) {
    let base = get_mount_base();
    let full_path = build_full_path(&base, &original_path);
    let full_pattern = if base.is_empty() {
        pattern
    } else {
        format!("^{}{}$", regex::escape(&base), pattern.trim_start_matches('^').trim_end_matches('$'))
    };
    
    let mut routes = get_routes().lock().unwrap();
    let route_info = RouteInfo {
        method: method.clone(),
        original_path: full_path,
        pattern: Regex::new(&full_pattern).expect("Invalid route pattern"),
        handler,
    };
    routes.entry(method).or_insert_with(Vec::new).push(route_info);
}

/// Find a handler for the given method and path
pub fn find_handler(method: &Method, path: &str) -> Option<Handler> {
    let routes = get_routes().lock().unwrap();
    if let Some(method_routes) = routes.get(method) {
        for route_info in method_routes {
            if route_info.pattern.is_match(path) {
                return Some(route_info.handler.clone());
            }
        }
    }
    None
}

/// Mount routes under a base path
/// 
/// # Arguments
/// * `base` - The base path prefix (e.g., "/api")
/// * `routes_fn` - Closure that registers routes
pub fn mount(base: &str, routes_fn: impl FnOnce()) {
    let base = base.trim_end_matches('/');
    set_mount_base(base);
    routes_fn();
    clear_mount_base();
}

/// Collect all registered routes for inspection
pub fn collect_routes() -> Vec<Route> {
    let routes = get_routes().lock().unwrap();
    let mut result = Vec::new();
    
    for (_method, route_infos) in routes.iter() {
        for route_info in route_infos {
            result.push(Route {
                method: route_info.method.clone(),
                path: route_info.original_path.clone(),
                handler: route_info.handler.clone(),
            });
        }
    }
    
    result
}

/// Get the current mount base path
pub fn get_mount_base() -> String {
    MOUNT_BASE.with(|mb| mb.borrow().clone())
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

/// Router that dispatches requests to registered handlers
pub struct Router;

impl Router {
    /// Create a new router instance
    pub fn new() -> Self {
        Self
    }
    
    /// Route an incoming request to the appropriate handler
    pub async fn route(&self, req: Request<Incoming>) -> Response<String> {
        let method = req.method().clone();
        let path = req.uri().path().to_string();
        
        // Find matching route using regex patterns
        if let Some(handler) = find_handler(&method, &path) {
            // Convert Request<Incoming> to crate::Request
            let (parts, body) = req.into_parts();
            let req = Request::from_parts(parts, body);
            return handler(req).await;
        }
        
        // No route found - return 404
        Self::not_found()
    }
    
    /// Return a 404 Not Found response
    pub fn not_found() -> Response<String> {
        Response::builder()
            .status(StatusCode::NOT_FOUND)
            .body("404 Not Found".to_string())
            .unwrap()
    }
    
    /// Return a 500 Internal Server Error response
    pub fn internal_error(msg: &str) -> Response<String> {
        Response::builder()
            .status(StatusCode::INTERNAL_SERVER_ERROR)
            .body(format!("500 Internal Server Error: {}", msg))
            .unwrap()
    }
    
    /// Return a 405 Method Not Allowed response
    pub fn method_not_allowed() -> Response<String> {
        Response::builder()
            .status(StatusCode::METHOD_NOT_ALLOWED)
            .body("405 Method Not Allowed".to_string())
            .unwrap()
    }
}

impl Default for Router {
    fn default() -> Self {
        Self::new()
    }
}
