#[macro_export]
macro_rules! get {
    ($path:expr, $handler:expr) => {
        {
            use std::sync::Arc;
            let handler_arc = Arc::new(|req: hyper::Request<hyper::body::Incoming>| {
                Box::pin($handler(req)) as std::pin::Pin<Box<dyn std::future::Future<Output = hyper::Response<String>> + Send>>
            });
            $crate::Route::new(hyper::Method::GET, $path, handler_arc);
        }
    };
}

#[macro_export]
macro_rules! post {
    ($path:expr, $handler:expr) => {
        {
            use std::sync::Arc;
            let handler_arc = Arc::new(|req: hyper::Request<hyper::body::Incoming>| {
                Box::pin($handler(req)) as std::pin::Pin<Box<dyn std::future::Future<Output = hyper::Response<String>> + Send>>
            });
            $crate::Route::new(hyper::Method::POST, $path, handler_arc);
        }
    };
}