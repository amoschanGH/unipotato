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
    ($port:expr) => {
        $crate::Server::new($port)
    };
    () => {
        $crate::Server::new(8000)
    };
}