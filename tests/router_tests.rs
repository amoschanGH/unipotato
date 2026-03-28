use unipotato::route::{Router, Handler, collect_routes, find_handler_with_params, register_route, mount};
use hyper::{Method, Response};
use std::sync::Arc;

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

/// Helper to create a simple regex pattern from a path
fn simple_pattern(path: &str) -> String {
    format!("^{}$", regex::escape(path))
}

// ============ Route Registration Tests ============

#[test]
fn test_route_registration() {
    let path = "/test_reg";
    register_route(
        Method::GET,
        path.to_string(),
        simple_pattern(path),
        make_handler("test"),
    );
    
    let routes = collect_routes();
    assert!(routes.iter().any(|r| r.path == path && r.method == Method::GET));
}

// ============ Mount Prefix Tests ============

#[test]
fn test_mount_prefix() {
    mount("/api", || {
        let path = "/mounted_test";
        register_route(
            Method::GET,
            path.to_string(),
            simple_pattern(path),
            make_handler("mounted"),
        );
    });
    
    let routes = collect_routes();
    assert!(routes.iter().any(|r| r.path == "/api/mounted_test"));
}

// ============ Router Error Response Tests ============

#[test]
fn test_router_error_responses() {
    let not_found = Router::not_found();
    assert_eq!(not_found.status(), 404);
    
    let error = Router::internal_error("test error");
    assert_eq!(error.status(), 500);
    
    let not_allowed = Router::method_not_allowed();
    assert_eq!(not_allowed.status(), 405);
}

// ============ Parameter Extraction Tests ============

#[test]
fn test_parameterized_route_match_and_param_extraction() {
    register_route(
        Method::GET,
        "/users/<id>/posts/<post_id>".to_string(),
        "^/users/[^/]+/posts/[^/]+$".to_string(),
        make_handler("matched"),
    );

    let matched = find_handler_with_params(&Method::GET, "/users/42/posts/9").expect("expected route match");
    assert_eq!(matched.params.get("id"), Some(&"42".to_string()));
    assert_eq!(matched.params.get("post_id"), Some(&"9".to_string()));
}

// ============ HTTP Method Dispatch Tests ============

#[test]
fn test_method_mismatch_returns_none_for_handler_lookup() {
    let path = "/method_mismatch_route";
    register_route(
        Method::GET,
        path.to_string(),
        simple_pattern(path),
        make_handler("get-only"),
    );

    assert!(find_handler_with_params(&Method::POST, path).is_none());
    assert!(find_handler_with_params(&Method::GET, path).is_some());
}
