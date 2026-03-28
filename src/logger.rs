use std::time::SystemTime;

// ANSI Color codes
pub const RESET: &str = "\x1b[0m";
pub const BOLD: &str = "\x1b[1m";
pub const DIM: &str = "\x1b[2m";

// Foreground colors
pub const BLACK: &str = "\x1b[30m";
pub const RED: &str = "\x1b[31m";
pub const GREEN: &str = "\x1b[32m";
pub const YELLOW: &str = "\x1b[33m";
pub const BLUE: &str = "\x1b[34m";
pub const MAGENTA: &str = "\x1b[35m";
pub const CYAN: &str = "\x1b[36m";
pub const WHITE: &str = "\x1b[37m";
pub const GRAY: &str = "\x1b[90m";

// Bright colors
pub const BRIGHT_RED: &str = "\x1b[91m";
pub const BRIGHT_GREEN: &str = "\x1b[92m";
pub const BRIGHT_YELLOW: &str = "\x1b[93m";
pub const BRIGHT_BLUE: &str = "\x1b[94m";
pub const BRIGHT_MAGENTA: &str = "\x1b[95m";
pub const BRIGHT_CYAN: &str = "\x1b[96m";

pub fn get_timestamp() -> String {
    let now = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap();
    let datetime = chrono::DateTime::from_timestamp(now.as_secs() as i64, 0)
        .unwrap()
        .with_timezone(&chrono::Local);
    datetime.format("%H:%M:%S").to_string()
}

pub fn should_log_http() -> bool {
    match std::env::var("UNIPOTATO_LOG_HTTP") {
        Ok(v) => {
            let v = v.trim().to_ascii_lowercase();
            !(v == "0" || v == "false" || v == "off" || v == "no")
        }
        Err(_) => true,
    }
}

#[macro_export]
macro_rules! log_info {
    ($($arg:tt)*) => {{
        let timestamp = $crate::logger::get_timestamp();
        println!(
            "{}[INFO]{} {}[{}]{} {}",
            $crate::logger::BRIGHT_CYAN,
            $crate::logger::RESET,
            $crate::logger::GRAY,
            timestamp,
            $crate::logger::RESET,
            format!($($arg)*)
        );
    }};
}

#[macro_export]
macro_rules! log_success {
    ($($arg:tt)*) => {{
        let timestamp = $crate::logger::get_timestamp();
        println!(
            "{}✓ [SUCCESS]{} {}[{}]{} {}",
            $crate::logger::BRIGHT_GREEN,
            $crate::logger::RESET,
            $crate::logger::GRAY,
            timestamp,
            $crate::logger::RESET,
            format!($($arg)*)
        );
    }};
}

#[macro_export]
macro_rules! log_warning {
    ($($arg:tt)*) => {{
        let timestamp = $crate::logger::get_timestamp();
        println!(
            "{}⚠ [WARNING]{} {}[{}]{} {}",
            $crate::logger::BRIGHT_YELLOW,
            $crate::logger::RESET,
            $crate::logger::GRAY,
            timestamp,
            $crate::logger::RESET,
            format!($($arg)*)
        );
    }};
}

#[macro_export]
macro_rules! log_error {
    ($($arg:tt)*) => {{
        let timestamp = $crate::logger::get_timestamp();
        eprintln!(
            "{}✗ [ERROR]{} {}[{}]{} {}",
            $crate::logger::BRIGHT_RED,
            $crate::logger::RESET,
            $crate::logger::GRAY,
            timestamp,
            $crate::logger::RESET,
            format!($($arg)*)
        );
    }};
}

#[macro_export]
macro_rules! log_debug {
    ($($arg:tt)*) => {{
        let timestamp = $crate::logger::get_timestamp();
        println!(
            "{}[DEBUG]{} {}[{}]{} {}",
            $crate::logger::MAGENTA,
            $crate::logger::RESET,
            $crate::logger::GRAY,
            timestamp,
            $crate::logger::RESET,
            format!($($arg)*)
        );
    }};
}

#[macro_export]
macro_rules! log_request {
    ($method:expr, $path:expr) => {{
        if $crate::logger::should_log_http() {
            let timestamp = $crate::logger::get_timestamp();
            println!();
            println!("{}╔════════════════════════════════════════════════════════════════╗{}", 
                $crate::logger::BLUE, $crate::logger::RESET);
            println!("{}║{} {}→ API REQUEST{} {}[{}]{}                                       {}║{}", 
                $crate::logger::BLUE, $crate::logger::RESET,
                $crate::logger::BRIGHT_CYAN, $crate::logger::RESET,
                $crate::logger::GRAY, timestamp, $crate::logger::RESET,
                $crate::logger::BLUE, $crate::logger::RESET);
            println!("{}║{} Method: {}{:<54}{} {}║{}", 
                $crate::logger::BLUE, $crate::logger::RESET,
                $crate::logger::BRIGHT_YELLOW, format!("{}", $method), $crate::logger::RESET,
                $crate::logger::BLUE, $crate::logger::RESET);
            println!("{}║{} Path:   {}{:<54}{} {}║{}", 
                $crate::logger::BLUE, $crate::logger::RESET,
                $crate::logger::WHITE, $path, $crate::logger::RESET,
                $crate::logger::BLUE, $crate::logger::RESET);
            println!("{}╚════════════════════════════════════════════════════════════════╝{}", 
                $crate::logger::BLUE, $crate::logger::RESET);
        }
    }};
}

#[macro_export]
macro_rules! log_response {
    ($status:expr, $path:expr) => {{
        if $crate::logger::should_log_http() {
            let timestamp = $crate::logger::get_timestamp();
            let status_color = if $status >= 200 && $status < 300 {
                $crate::logger::BRIGHT_GREEN
            } else if $status >= 400 && $status < 500 {
                $crate::logger::BRIGHT_YELLOW
            } else if $status >= 500 {
                $crate::logger::BRIGHT_RED
            } else {
                $crate::logger::CYAN
            };
            
            println!("{}╔════════════════════════════════════════════════════════════════╗{}", 
                $crate::logger::BLUE, $crate::logger::RESET);
            println!("{}║{} {}← API RESPONSE{} {}[{}]{}                                      {}║{}", 
                $crate::logger::BLUE, $crate::logger::RESET,
                $crate::logger::BRIGHT_CYAN, $crate::logger::RESET,
                $crate::logger::GRAY, timestamp, $crate::logger::RESET,
                $crate::logger::BLUE, $crate::logger::RESET);
            println!("{}║{} Status: {}{:<54}{} {}║{}", 
                $crate::logger::BLUE, $crate::logger::RESET,
                status_color, format!("{}", $status), $crate::logger::RESET,
                $crate::logger::BLUE, $crate::logger::RESET);
            println!("{}║{} Path:   {}{:<54}{} {}║{}", 
                $crate::logger::BLUE, $crate::logger::RESET,
                $crate::logger::WHITE, $path, $crate::logger::RESET,
                $crate::logger::BLUE, $crate::logger::RESET);
            println!("{}╚════════════════════════════════════════════════════════════════╝{}", 
                $crate::logger::BLUE, $crate::logger::RESET);
            println!();
        }
    }};
}

#[macro_export]
macro_rules! log_section {
    ($title:expr) => {{
        println!();
        println!("{}════════════════════ {} ════════════════════{}", 
            $crate::logger::CYAN, $title, $crate::logger::RESET);
    }};
}

#[macro_export]
macro_rules! log_divider {
    () => {{
        println!("{}────────────────────────────────────────────────────────────────{}", 
            $crate::logger::GRAY, $crate::logger::RESET);
    }};
}

