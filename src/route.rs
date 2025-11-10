use hyper::{Request, Response, Method, body::Incoming};
use std::sync::{Arc, Mutex};
use once_cell::sync::Lazy;

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

static ROUTES: Lazy<Mutex<Vec<Route>>> = Lazy::new(|| Mutex::new(Vec::new()));

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
    let base = base.trim_end_matches('/');
    
    MOUNT_BASE.with(|mb| {
        *mb.borrow_mut() = base.to_string();
    });
    
    routes_fn();
    
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

pub fn register_route(method: Method, path: String, handler: Handler) {
    let base = get_mount_base();
    let full_path = if base.is_empty() {
        path
    } else {
        format!("{}{}", base, path)
    };
    
    ROUTES.lock().unwrap().push(Route {
        method,
        path: full_path,
        handler,
    });
}
