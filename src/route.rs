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
static MOUNT_CONFIGS: Lazy<Mutex<Vec<(String, Box<dyn FnOnce() + Send>)>>> = Lazy::new(|| Mutex::new(Vec::new()));

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

pub fn store_mount_config(base: String, routes_fn: Box<dyn FnOnce() + Send>) {
    if let Ok(mut configs) = MOUNT_CONFIGS.lock() {
        configs.push((base, routes_fn));
    }
}

pub fn apply_mount_configs() {
    let configs = {
        let mut mount_configs = MOUNT_CONFIGS.lock().unwrap();
        std::mem::take(&mut *mount_configs)
    };
    
    for (base, routes_fn) in configs {
        mount(&base, routes_fn);
    }
}

pub fn mount(base: &str, routes_fn: impl FnOnce()) {
    let base = base.trim_end_matches('/');
    
    // Store current routes count
    let routes_before = ROUTES.lock().unwrap().len();
    
    MOUNT_BASE.with(|mb| {
        *mb.borrow_mut() = base.to_string();
    });
    
    routes_fn();
    
    MOUNT_BASE.with(|mb| {
        mb.borrow_mut().clear();
    });
    
    // Update paths of newly added routes
    let mut routes = ROUTES.lock().unwrap();
    for i in routes_before..routes.len() {
        if !routes[i].path.starts_with(base) && !base.is_empty() {
            routes[i].path = format!("{}{}", base, routes[i].path);
        }
    }
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
