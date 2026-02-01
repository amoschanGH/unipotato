use unipotato::route::{Router, Route, Handler, collect_routes, register_route, mount};
use hyper::{Method, Request, Response};
use std::sync::Arc;

fn clear_routes() {
    // Note: In real tests, you'd want a way to clear routes between tests
    // For now, tests may accumulate routes
}

fn make_handler(response_body: &'static str) -> Handler {
    Arc::new(move |_req| {
        Box::pin(async move {
            Response::builder()
                .status(200)
                .body(response_body.to_string())
                .unwrap()
        }) as std::pin::Pin<Box<dyn std::future::Future<Output = Response<String>> + Send>>
    })
}

#[test]
fn test_route_registration() {
    register_route(Method::GET, "/test".to_string(), make_handler("test"));
    
    let routes = collect_routes();
    assert!(routes.iter().any(|r| r.path == "/test" && r.method == Method::GET));
}

#[test]
fn test_path_matching_exact() {
    assert!(Router::path_matches("/users", "/users"));
    assert!(Router::path_matches("/api/users", "/api/users"));
    assert!(!Router::path_matches("/users", "/posts"));
    assert!(!Router::path_matches("/users", "/users/123"));
}

#[test]
fn test_path_matching_with_params() {
    assert!(Router::path_matches("/users/:id", "/users/123"));
    assert!(Router::path_matches("/users/:id/posts/:post_id", "/users/123/posts/456"));
    assert!(!Router::path_matches("/users/:id", "/users"));
    assert!(!Router::path_matches("/users/:id", "/users/123/extra"));
}

#[test]
fn test_extract_params() {
    let params = Router::extract_params("/users/:id", "/users/123");
    assert_eq!(params.get("id"), Some(&"123".to_string()));
    
    let params = Router::extract_params("/users/:user_id/posts/:post_id", "/users/42/posts/99");
    assert_eq!(params.get("user_id"), Some(&"42".to_string()));
    assert_eq!(params.get("post_id"), Some(&"99".to_string()));
}

#[test]
fn test_mount_prefix() {
    mount("/api", || {
        register_route(Method::GET, "/mounted".to_string(), make_handler("mounted"));
    });
    
    let routes = collect_routes();
    assert!(routes.iter().any(|r| r.path == "/api/mounted"));
}

#[test]
fn test_router_error_responses() {
    let not_found = Router::not_found();
    assert_eq!(not_found.status(), 404);
    
    let error = Router::internal_error("test error");
    assert_eq!(error.status(), 500);
    
    let not_allowed = Router::method_not_allowed();
    assert_eq!(not_allowed.status(), 405);
}
