#[macro_export]
macro_rules! register_get {
    ($path:expr, $handler:ident) => {
        {
            use std::sync::Arc;
            use hyper::{Request, Response, Method, body::Incoming};
            let handler_arc = Arc::new(|req: Request<Incoming>| {
                Box::pin($handler(req)) as std::pin::Pin<Box<dyn std::future::Future<Output = Response<String>> + Send>>
            });
            $crate::Route::new(Method::GET, $path, handler_arc);
        }
    };
}

#[macro_export]
macro_rules! register_post {
    ($path:expr, $handler:ident) => {
        {
            use std::sync::Arc;
            use hyper::{Request, Response, Method, body::Incoming};
            let handler_arc = Arc::new(|req: Request<Incoming>| {
                Box::pin($handler(req)) as std::pin::Pin<Box<dyn std::future::Future<Output = Response<String>> + Send>>
            });
            $crate::Route::new(Method::POST, $path, handler_arc);
        }
    };
}