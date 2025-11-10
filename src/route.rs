use hyper::{Request, Response, Method, body::Incoming};
use std::sync::{Arc, Mutex};
use lazy_static::lazy_static;

pub type Handler = Arc<dyn Fn(Request<Incoming>) -> std::pin::Pin<Box<dyn std::future::Future<Output = Response<String>> + Send>> + Send + Sync>;

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

lazy_static! {
    static ref ROUTES: Mutex<Vec<Route>> = Mutex::new(Vec::new());
}

impl Route {
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

pub fn mount(base: &str, routes_fn: impl FnOnce()) {
    // Store the current base path for nested routes
    let base = base.trim_end_matches('/');
    
    // Execute the routes registration with context
    MOUNT_BASE.with(|mb| {
        *mb.borrow_mut() = base.to_string();
    });
    
    routes_fn();
    
    // Clear the base path after registration
    MOUNT_BASE.with(|mb| {
        mb.borrow_mut().clear();
    });
}

thread_local! {
    static MOUNT_BASE: std::cell::RefCell<String> = std::cell::RefCell::new(String::new());
}

pub fn get_mount_base() -> String {
    MOUNT_BASE.with(|mb| mb.borrow().clone())
}

pub fn collect_routes() -> Vec<Route> {
    ROUTES.lock().unwrap_or_else(|_| {
        panic!("Failed to lock ROUTES mutex")
    }).clone()
}
