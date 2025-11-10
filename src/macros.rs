#[macro_export]
macro_rules! get {
    ($path:expr, $handler:ident) => {
        {
            use std::sync::Arc;
            use hyper::{Request, Response, Method, body::Incoming};
            let base = $crate::route::get_mount_base();
            let full_path = if base.is_empty() {
                $path.to_string()
            } else {
                format!("{}{}", base, $path)
            };
            let handler_arc = Arc::new(|req: Request<Incoming>| {
                Box::pin($handler(req)) as std::pin::Pin<Box<dyn std::future::Future<Output = Response<String>> + Send>>
            });
            $crate::Route::new(Method::GET, full_path, handler_arc);
        }
    };
}

#[macro_export]
macro_rules! post {
    ($path:expr, $handler:ident) => {
        {
            use std::sync::Arc;
            use hyper::{Request, Response, Method, body::Incoming};
            let base = $crate::route::get_mount_base();
            let full_path = if base.is_empty() {
                $path.to_string()
            } else {
                format!("{}{}", base, $path)
            };
            let handler_arc = Arc::new(|req: Request<Incoming>| {
                Box::pin($handler(req)) as std::pin::Pin<Box<dyn std::future::Future<Output = Response<String>> + Send>>
            });
            $crate::Route::new(Method::POST, full_path, handler_arc);
        }
    };
}

#[macro_export]
macro_rules! routes {
    ($($method:ident($path:expr, $handler:ident)),* $(,)?) => {
        {
            $(
                $method!($path, $handler);
            )*
        }
    };
}

#[macro_export]
macro_rules! launch {
    () => {
        {
            let rt = tokio::runtime::Runtime::new().unwrap();
            rt.block_on(async {
                println!("Starting server...");
                if let Err(e) = $crate::server::launch().await {
                    eprintln!("Failed to start server: {}", e);
                    eprintln!("This might be because port 8000 is already in use.");
                    eprintln!("Try killing any existing processes on port 8000:");
                    eprintln!("  lsof -ti:8000 | xargs kill -9");
                    std::process::exit(1);
                }
            });
        }
    };
}