#[macro_export]
macro_rules! routes {
    ($($method:ident($path:expr, $handler:ident)),* $(,)?) => {
        {
            $(
                $method!($path, $handler);
            )*
        }
    };
    ($($module:ident :: $handler:ident),* $(,)?) => {
        || {
            $(
                {
                    // Reference the function to ensure module is loaded  
                    let _ = $module::$handler as fn(::unipotato::Request) -> _;
                    // Call registration using the generated struct
                    $crate::__register_route!($module, $handler);
                }
            )*
        }
    };
}

#[macro_export]
macro_rules! __register_route {
    ($module:ident, $handler:ident) => {
        paste::paste! {
            $module::[<__ROUTE_INFO_ $handler:upper>]::register();
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