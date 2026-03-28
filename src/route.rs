use hyper::{Response, Method, body::Incoming, StatusCode};
use std::sync::Arc;
use std::collections::HashMap;
use std::hash::{Hash, Hasher};
use regex::Regex;
use dashmap::DashMap;
use once_cell::sync::Lazy;
use smallvec::SmallVec;
use memchr::memchr;

/// Type alias for async request handlers - now uses crate::request::Request
pub type Handler = Arc<
    dyn Fn(crate::request::Request) -> std::pin::Pin<Box<dyn std::future::Future<Output = crate::Response> + Send>> 
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
    param_names: Vec<String>,  // Store parameter names like ["id", "post_id"]
    handler: Handler,
    is_static: bool,  // True if route has no parameters (use string equality instead of regex)
}

/// Global storage for registered routes - lock-free concurrent HashMap
static ROUTES: Lazy<DashMap<Method, Vec<RouteInfo>>> = Lazy::new(DashMap::new);

thread_local! {
    /// Thread-local storage for mount base path
    static MOUNT_BASE: std::cell::RefCell<String> = std::cell::RefCell::new(String::new());
    
    /// Thread-local LRU cache for route matches (capacity 4 for typical 1-4 concurrent routes per thread)
    static ROUTE_CACHE: std::cell::RefCell<lru::LruCache<u64, CachedHandlerMatch>> = 
        std::cell::RefCell::new(lru::LruCache::new(std::num::NonZeroUsize::new(4).unwrap()));
}

#[derive(Clone)]
struct CachedHandlerMatch {
    path: String,
    matched: HandlerMatch,
}

/// Build a stable, allocation-free cache key from method + path.
fn make_route_cache_key(method: &Method, path: &str) -> u64 {
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    method.as_str().hash(&mut hasher);
    path.hash(&mut hasher);
    hasher.finish()
}

/// Extract parameter names from a path pattern like "/users/<id>/posts/<post_id>"
/// Uses SmallVec to avoid heap allocation for typical 1-3 parameters
fn extract_param_names(path: &str) -> Vec<String> {
    // SmallVec avoids heap allocation for typical cases (1-4 params)
    let mut names: SmallVec<[String; 4]> = SmallVec::new();
    let bytes = path.as_bytes();
    let mut i = 0;
    
    while let Some(pos) = memchr(b'<', &bytes[i..]) {
        i += pos + 1;
        let mut name = String::new();
        
        while i < bytes.len() {
            let c = bytes[i] as char;
            if c == '>' {
                break;
            }
            name.push(c);
            i += 1;
        }
        
        if !name.is_empty() {
            names.push(name);
        }
        i += 1;
    }
    
    names.into_vec()
}

/// Register a new route with pattern matching support
pub fn register_route(method: Method, original_path: String, pattern: String, handler: Handler) {
    let base = get_mount_base();
    let full_path = build_full_path(&base, &original_path);
    
    // Normalize pattern to ensure it has proper anchors for efficient regex matching
    let normalized_pattern = normalize_pattern(&pattern);
    
    let full_pattern = if base.is_empty() {
        normalized_pattern
    } else {
        // Build pattern with base prefix, stripping and re-adding anchors
        let pattern_without_anchors = normalized_pattern
            .trim_start_matches('^')
            .trim_end_matches('$');
        format!("^{}{}$", regex::escape(&base), pattern_without_anchors)
    };
    
    // Extract parameter names from the original path
    let param_names = extract_param_names(&full_path);
    
    // Detect if this route is static (no parameters)
    let is_static = param_names.is_empty();
    
    let route_info = RouteInfo {
        method: method.clone(),
        original_path: full_path,
        pattern: Regex::new(&full_pattern).expect("Invalid route pattern"),
        param_names,
        handler,
        is_static,
    };
    
    // Use DashMap's entry API for lock-free insertion
    ROUTES.entry(method).or_insert_with(Vec::new).push(route_info);
}

/// Normalize a regex pattern to ensure it has proper anchors (^ and $) for efficient matching
fn normalize_pattern(pattern: &str) -> String {
    let trimmed = pattern.trim();
    let needs_start = !trimmed.starts_with('^');
    let needs_end = !trimmed.ends_with('$');
    
    match (needs_start, needs_end) {
        (true, true) => format!("^{}$", trimmed),
        (true, false) => format!("^{}", trimmed),
        (false, true) => format!("{}$", trimmed),
        (false, false) => trimmed.to_string(),
    }
}

/// Result of finding a handler - includes the handler and extracted parameters
#[derive(Clone)]
pub struct HandlerMatch {
    pub handler: Handler,
    pub params: HashMap<String, String>,
}

/// Find a handler for the given method and path, extracting parameters
pub fn find_handler_with_params(method: &Method, path: &str) -> Option<HandlerMatch> {
    // Create an allocation-free cache key from method and path
    let cache_key = make_route_cache_key(method, path);
    
    // Check thread-local cache first (lock-free, fast path)
    let cached = ROUTE_CACHE.with(|c| {
        c.borrow_mut().get(&cache_key).cloned()
    });
    
    if let Some(cached_entry) = cached {
        // Guard against rare hash collisions by validating the path.
        if cached_entry.path == path {
            return Some(cached_entry.matched);
        }
    }
    
    // Cache miss - perform actual route lookup
    let result = find_handler_with_params_uncached(method, path);
    
    // Store result in cache for future lookups
    if let Some(ref matched) = result {
        ROUTE_CACHE.with(|c| {
            c.borrow_mut().put(cache_key, CachedHandlerMatch {
                path: path.to_string(),
                matched: matched.clone(),
            });
        });
    }
    
    result
}

/// Internal function to find handler without cache (performs actual regex matching)
fn find_handler_with_params_uncached(method: &Method, path: &str) -> Option<HandlerMatch> {
    // Use DashMap's get() for lock-free concurrent read access
    if let Some(method_routes_ref) = ROUTES.get(method) {
        for route_info in method_routes_ref.iter() {
            // FAST PATH: Static routes use direct string equality (skip regex entirely)
            if route_info.is_static {
                if path == route_info.original_path {
                    return Some(HandlerMatch {
                        handler: route_info.handler.clone(),
                        params: HashMap::new(),
                    });
                }
                continue;
            }
            
            // SLOW PATH: Parameterized routes require regex matching
            // Use is_match to avoid unnecessary capture allocation.
            if route_info.pattern.is_match(path) {
                // Extract parameter values
                let mut params = HashMap::new();
                
                // The captures include the full match at index 0, then each group
                // But our pattern uses [^/]+ which doesn't create capture groups
                // We need to use a different approach - match segments
                let param_values = extract_param_values(&route_info.original_path, path);
                
                for (i, name) in route_info.param_names.iter().enumerate() {
                    if let Some(value) = param_values.get(i) {
                        params.insert(name.clone(), value.clone());
                    }
                }
                
                return Some(HandlerMatch {
                    handler: route_info.handler.clone(),
                    params,
                });
            }
        }
    }
    None
}

/// Extract parameter values by comparing the route pattern with the actual path
/// Uses SmallVec to avoid heap allocation for typical 1-3 parameters
fn extract_param_values(route_path: &str, actual_path: &str) -> Vec<String> {
    // Use SmallVec to stack-allocate common case (1-4 segments)
    let route_segments: SmallVec<[&str; 8]> = route_path.split('/').collect();
    let actual_segments: SmallVec<[&str; 8]> = actual_path.split('/').collect();
    
    let mut values = Vec::new();
    
    for (route_seg, actual_seg) in route_segments.iter().zip(actual_segments.iter()) {
        if route_seg.starts_with('<') && route_seg.ends_with('>') {
            values.push(actual_seg.to_string());
        }
    }
    
    values
}

/// Find a handler for the given method and path (legacy, without params)
pub fn find_handler(method: &Method, path: &str) -> Option<Handler> {
    find_handler_with_params(method, path).map(|m| m.handler)
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
    let mut result = Vec::new();
    
    // Iterate through all methods in the DashMap (lock-free read)
    for method_routes_ref in ROUTES.iter() {
        let (_method, route_infos) = method_routes_ref.pair();
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
    pub async fn route(&self, req: hyper::Request<Incoming>) -> Response<String> {
        let method = req.method().clone();
        let path = req.uri().path().to_string();
        
        // Find matching route using regex patterns with param extraction
        if let Some(HandlerMatch { handler, params }) = find_handler_with_params(&method, &path) {
            // Create our custom Request with extracted path parameters
            let custom_req = crate::request::Request::with_params(req, params);
            return handler(custom_req).await;
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

#[cfg(test)]
mod tests {
    use super::*;
    use hyper::Method;
    use std::sync::atomic::{AtomicUsize, Ordering};

    static TEST_COUNTER: AtomicUsize = AtomicUsize::new(0);

    // ============ Test Helpers ============

    fn reset_routes() {
        ROUTES.clear();
        clear_mount_base();
    }

    fn next_test_path(prefix: &str) -> String {
        let id = TEST_COUNTER.fetch_add(1, Ordering::SeqCst);
        format!("/{prefix}_{id}")
    }

    fn test_handler(body: &'static str) -> Handler {
        Arc::new(move |_req| {
            Box::pin(async move {
                Response::builder()
                    .status(StatusCode::OK)
                    .body(body.to_string())
                    .unwrap()
            })
        })
    }

    // ============ Parameter Extraction Tests ============

    #[test]
    fn extract_param_names_handles_static_and_dynamic_segments() {
        assert_eq!(extract_param_names("/users/<id>"), vec!["id".to_string()]);
        assert_eq!(
            extract_param_names("/users/<uid>/posts/<pid>"),
            vec!["uid".to_string(), "pid".to_string()]
        );
        assert!(extract_param_names("/users/static").is_empty());
    }

    #[test]
    fn extract_param_values_matches_segment_positions() {
        let values = extract_param_values("/users/<uid>/posts/<pid>", "/users/42/posts/9");
        assert_eq!(values, vec!["42".to_string(), "9".to_string()]);

        let none = extract_param_values("/static/path", "/static/path");
        assert!(none.is_empty());
    }

    // ============ Path Building & Mount Tests ============

    #[test]
    fn build_full_path_respects_mount_base() {
        assert_eq!(build_full_path("", "/users"), "/users");
        assert_eq!(build_full_path("/api", "/users"), "/api/users");
    }

    #[test]
    fn mount_sets_and_clears_base_path() {
        reset_routes();
        assert_eq!(get_mount_base(), "");

        mount("/api", || {
            assert_eq!(get_mount_base(), "/api");
        });

        assert_eq!(get_mount_base(), "");
    }

    #[test]
    fn register_route_applies_mount_base_to_path() {
        reset_routes();

        mount("/api", || {
            register_route(
                Method::GET,
                "/users/<id>".to_string(),
                "^/users/[^/]+$".to_string(),
                test_handler("mounted"),
            );
        });

        let found = find_handler_with_params(&Method::GET, "/api/users/7").expect("mounted route should match");
        assert_eq!(found.params.get("id"), Some(&"7".to_string()));
    }

    // ============ Route Matching & Dispatch Tests ============

    #[test]
    fn find_handler_with_params_extracts_named_params() {
        reset_routes();

        register_route(
            Method::GET,
            "/users/<id>".to_string(),
            "^/users/[^/]+$".to_string(),
            test_handler("ok"),
        );

        let found = find_handler_with_params(&Method::GET, "/users/123").expect("route should match");
        assert_eq!(found.params.get("id"), Some(&"123".to_string()));
    }

    #[test]
    fn find_handler_with_params_respects_http_method() {
        reset_routes();

        let path = next_test_path("method_guard");
        let pattern = format!("^{}$", regex::escape(&path));
        register_route(Method::GET, path.clone(), pattern, test_handler("get"));

        assert!(find_handler_with_params(&Method::POST, &path).is_none());
        assert!(find_handler_with_params(&Method::GET, &path).is_some());
    }

    // ============ Router Error Response Tests ============

    #[test]
    fn router_error_helpers_return_expected_status_codes() {
        assert_eq!(Router::not_found().status(), StatusCode::NOT_FOUND);
        assert_eq!(Router::internal_error("boom").status(), StatusCode::INTERNAL_SERVER_ERROR);
        assert_eq!(Router::method_not_allowed().status(), StatusCode::METHOD_NOT_ALLOWED);
    }
}
