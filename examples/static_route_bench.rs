// Benchmark static vs parameterized route matching
// Demonstrates the fast-path optimization for routes without parameters

use unipotato::route::{register_route, find_handler_with_params};
use hyper::Method;
use std::sync::Arc;
use std::time::Instant;

fn main() {
    // Register 64 static routes (fast path)
    for i in 0..64 {
        let path = format!("/static/route_{}", i);
        register_route(
            Method::GET,
            path,
            format!("^/static/route_{}$", i),
            Arc::new(|_req| {
                Box::pin(async {
                    hyper::Response::builder()
                        .status(200)
                        .body("ok".to_string())
                        .unwrap()
                })
            }),
        );
    }
    
    // Register 64 parameterized routes (slow path - requires regex)
    for _i in 0..64 {
        let path = format!("/param/<id>/sub_<item>");
        register_route(
            Method::GET,
            path,
            format!("^/param/[^/]+/sub_[^/]+$"),
            Arc::new(|_req| {
                Box::pin(async {
                    hyper::Response::builder()
                        .status(200)
                        .body("ok".to_string())
                        .unwrap()
                })
            }),
        );
    }
    
    println!("=== Static Route Fast-Path Benchmark ===\n");
    
    // Benchmark static routes (should use string equality, fast)
    println!("Static routes (fast path - string equality):");
    let start = Instant::now();
    for _ in 0..10_000 {
        let _ = find_handler_with_params(&Method::GET, "/static/route_42");
    }
    let static_time = start.elapsed();
    println!("  10,000 lookups: {:.3} ms", static_time.as_secs_f64() * 1000.0);
    println!("  Per-lookup: {:.3} µs\n", static_time.as_secs_f64() * 1_000_000.0 / 10_000.0);
    
    // Benchmark parameterized routes (requires regex matching, slower)
    println!("Parameterized routes (slow path - regex):");
    let start = Instant::now();
    for i in 0..10_000 {
        let path = format!("/param/{}/sub_{}", i % 100, i % 50);
        let _ = find_handler_with_params(&Method::GET, &path);
    }
    let param_time = start.elapsed();
    println!("  10,000 lookups: {:.3} ms", param_time.as_secs_f64() * 1000.0);
    println!("  Per-lookup: {:.3} µs\n", param_time.as_secs_f64() * 1_000_000.0 / 10_000.0);
    
    // Summary
    let speedup = param_time.as_secs_f64() / static_time.as_secs_f64();
    println!("Performance improvement:");
    println!("  Static routes are {:.1}x faster than parameterized routes", speedup);
    println!("  Latency reduction: {:.2} µs per static route lookup", 
             (param_time.as_secs_f64() - static_time.as_secs_f64()) * 1_000_000.0 / 10_000.0);
}
