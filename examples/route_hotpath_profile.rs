use hyper::{Method, Response, StatusCode};
use std::hint::black_box;
use std::sync::Arc;
use unipotato::route::{find_handler_with_params, register_route, Handler};

fn make_handler() -> Handler {
    Arc::new(|_req| {
        Box::pin(async move {
            Response::builder()
                .status(StatusCode::OK)
                .body("ok".to_string())
                .unwrap()
        })
    })
}

fn main() {
    // Register a mix of static and parameterized routes to exercise regex + param extraction.
    for i in 0..64 {
        let static_path = format!("/static/route/{i}");
        let static_pattern = format!("^/static/route/{i}$");
        register_route(Method::GET, static_path, static_pattern, make_handler());

        let param_path = format!("/users/<id>/posts/{i}");
        let param_pattern = format!("^/users/[^/]+/posts/{i}$");
        register_route(Method::GET, param_path, param_pattern, make_handler());
    }

    let paths: Vec<String> = (0..64)
        .flat_map(|i| {
            [
                format!("/static/route/{i}"),
                format!("/users/{}/posts/{i}", i * 10 + 1),
            ]
        })
        .collect();

    // Hot loop for profiling route lookup behavior under repeated access.
    let mut hits = 0usize;
    for n in 0..5_000_000usize {
        let path = &paths[n % paths.len()];
        if black_box(find_handler_with_params(&Method::GET, path)).is_some() {
            hits += 1;
        }
    }

    // Prevent the optimizer from eliding the loop.
    println!("hits={hits}");
}
