use hyper::{Response, body::Incoming, service::service_fn};
use hyper_util::rt::TokioIo;
use tokio::net::TcpListener;
use std::convert::Infallible;
#[cfg(debug_assertions)]
use std::path::{Path, PathBuf};
#[cfg(debug_assertions)]
use std::process::{Child, Command};
#[cfg(debug_assertions)]
use std::sync::mpsc;
#[cfg(debug_assertions)]
use std::time::{Duration, Instant};
#[cfg(debug_assertions)]
use notify::{Event, EventKind, RecommendedWatcher, RecursiveMode, Watcher};
use crate::{
    handler::not_found, 
    route::{collect_routes, find_handler_with_params, HandlerMatch},
    request::Request,
    log_debug, log_error, log_info, log_request, log_response, log_success,
};

/// Main server struct that handles HTTP requests
pub struct Unipotato {
    port: u16,
}

#[cfg(debug_assertions)]
const DEV_RELOAD_ENV: &str = "UNIPOTATO_DEV_RELOAD";
#[cfg(debug_assertions)]
const DEV_RELOAD_CHILD_ENV: &str = "UNIPOTATO_DEV_RELOAD_CHILD";
#[cfg(debug_assertions)]
const DEV_RELOAD_DEBOUNCE_MS: u64 = 400;

impl Unipotato {
    /// Create a new server instance
    pub fn launch(port: u16) -> Self {
        Self { port }
    }

    /// Get the port number the server is configured to run on
    pub fn port(&self) -> u16 {
        self.port
    }

    pub fn mount<F>(self, base: &str, routes_fn: F) -> Self 
    where 
        F: FnOnce()
    {
        crate::route::mount(base, routes_fn);
        self
    }

    /// Start the server and begin listening for requests
    pub fn start(self) {
        #[cfg(debug_assertions)]
        if self.should_run_dev_supervisor() {
            self.run_dev_supervisor();
            return;
        }

        crate::banner::print_banner();
        
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async move {
            log_info!("Starting Unipotato server...");
            
            if let Err(e) = self.launch_internal().await {
                self.handle_startup_error(e);
            }
        });
    }

    /// Handle startup errors with helpful messages
    fn handle_startup_error(&self, error: Box<dyn std::error::Error>) {
        log_error!("Failed to start server: {}", error);
        log_error!("This might be because port {} is already in use.", self.port);
        log_info!("Try killing any existing processes on port {}:", self.port);
        log_info!("  lsof -ti:{} | xargs kill -9", self.port);
        std::process::exit(1);
    }

    /// Internal method to launch the server and accept connections
    async fn launch_internal(&self) -> Result<(), Box<dyn std::error::Error>> {
        let addr = format!("0.0.0.0:{}", self.port);
        let listener = TcpListener::bind(&addr).await?;
        
        self.log_registered_routes();
        log_success!("Unipotato listening on http://localhost:{}", self.port);
        
        self.accept_connections(listener).await
    }

    /// Log all registered routes for debugging
    fn log_registered_routes(&self) {
        let routes = collect_routes();
        log_info!("Registered {} routes:", routes.len());
        
        for route in &routes {
            log_debug!("  {} {}", route.method, route.path);
        }
    }

    /// Accept and handle incoming connections
    async fn accept_connections(&self, listener: TcpListener) -> Result<(), Box<dyn std::error::Error>> {
        loop {
            let (stream, _) = listener.accept().await?;
            let io = TokioIo::new(stream);
            
            tokio::task::spawn(async move {
                if let Err(err) = Self::serve_connection(io).await {
                    if Self::is_benign_connection_error(&err) {
                        log_debug!("Client closed connection: {:?}", err);
                    } else {
                        log_error!("Error serving connection: {:?}", err);
                    }
                }
            });
        }
    }

    fn is_benign_connection_error(err: &hyper::Error) -> bool {
        let message = err.to_string().to_ascii_lowercase();
        message.contains("incompletemessage")
            || message.contains("connection closed before message completed")
            || message.contains("connection reset by peer")
            || message.contains("broken pipe")
    }

    /// Serve a single HTTP connection
    async fn serve_connection(io: TokioIo<tokio::net::TcpStream>) -> Result<(), hyper::Error> {
        let service = service_fn(router);
        hyper::server::conn::http1::Builder::new()
            .serve_connection(io, service)
            .await
    }

    #[cfg(debug_assertions)]
    fn should_run_dev_supervisor(&self) -> bool {
        Self::env_flag_enabled(DEV_RELOAD_ENV) && !Self::env_flag_enabled(DEV_RELOAD_CHILD_ENV)
    }

    #[cfg(debug_assertions)]
    fn env_flag_enabled(name: &str) -> bool {
        match std::env::var(name) {
            Ok(value) => Self::is_truthy_flag(&value),
            Err(_) => false,
        }
    }

    #[cfg(debug_assertions)]
    fn is_truthy_flag(value: &str) -> bool {
        matches!(
            value.trim().to_ascii_lowercase().as_str(),
            "1" | "true" | "yes" | "on"
        )
    }

    #[cfg(debug_assertions)]
    fn run_dev_supervisor(&self) {
        let context = match DevReloadContext::discover() {
            Some(ctx) => ctx,
            None => {
                log_error!("Auto-reload requested, but current executable is not under a Cargo target directory.");
                log_error!("Falling back to normal server startup.");
                self.start_child_server();
                return;
            }
        };

        log_info!("Dev auto-reload enabled.");
        log_info!("Watching project at {}", context.cargo_root.display());

        let mut child = match context.spawn_child() {
            Ok(child) => child,
            Err(err) => {
                log_error!("Failed to spawn server child process: {}", err);
                std::process::exit(1);
            }
        };

        let (tx, rx) = mpsc::channel::<notify::Result<Event>>();
        let mut watcher = match notify::recommended_watcher(move |result| {
            let _ = tx.send(result);
        }) {
            Ok(watcher) => watcher,
            Err(err) => {
                log_error!("Failed to initialize file watcher: {}", err);
                let _ = child.kill();
                let _ = child.wait();
                std::process::exit(1);
            }
        };

        if let Err(err) = context.register_watchers(&mut watcher) {
            log_error!("Failed to register watch paths: {}", err);
            let _ = child.kill();
            let _ = child.wait();
            std::process::exit(1);
        }

        loop {
            if let Ok(Some(status)) = child.try_wait() {
                if !status.success() {
                    log_error!("Server process exited with status {}.", status);
                }
                log_info!("Server process stopped. Exiting dev supervisor.");
                return;
            }

            if Self::wait_for_relevant_change(&rx).is_none() {
                continue;
            }

            log_info!("Source change detected. Building...");
            if !context.build() {
                log_error!("Build failed. Server keeps running previous executable.");
                continue;
            }

            log_success!("Build succeeded. Restarting server...");

            if let Err(err) = child.kill() {
                log_error!("Failed to stop child process: {}", err);
            }
            let _ = child.wait();

            child = match context.spawn_child() {
                Ok(next_child) => next_child,
                Err(err) => {
                    log_error!("Failed to restart server child process: {}", err);
                    std::process::exit(1);
                }
            };
        }
    }

    #[cfg(debug_assertions)]
    fn start_child_server(&self) {
        crate::banner::print_banner();

        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async move {
            log_info!("Starting Unipotato server...");

            if let Err(e) = self.launch_internal().await {
                self.handle_startup_error(e);
            }
        });
    }

    #[cfg(debug_assertions)]
    fn wait_for_relevant_change(rx: &mpsc::Receiver<notify::Result<Event>>) -> Option<()> {
        let first = match rx.recv_timeout(Duration::from_millis(250)) {
            Ok(event) => event,
            Err(mpsc::RecvTimeoutError::Timeout) => return None,
            Err(mpsc::RecvTimeoutError::Disconnected) => return None,
        };

        if let Ok(ref event) = first {
            if !Self::is_relevant_event(event) {
                return None;
            }
        }

        let started = Instant::now();
        while started.elapsed() < Duration::from_millis(DEV_RELOAD_DEBOUNCE_MS) {
            match rx.recv_timeout(Duration::from_millis(50)) {
                Ok(Ok(_)) => continue,
                Ok(Err(_)) => continue,
                Err(mpsc::RecvTimeoutError::Timeout) => break,
                Err(mpsc::RecvTimeoutError::Disconnected) => break,
            }
        }

        match first {
            Ok(event) if Self::is_relevant_event(&event) => Some(()),
            _ => None,
        }
    }

    #[cfg(debug_assertions)]
    fn is_relevant_event(event: &Event) -> bool {
        let is_path_relevant = event.paths.iter().any(|path| Self::is_relevant_path(path));
        if !is_path_relevant {
            return false;
        }

        matches!(
            event.kind,
            EventKind::Create(_) | EventKind::Modify(_) | EventKind::Remove(_)
        )
    }

    #[cfg(debug_assertions)]
    fn is_relevant_path(path: &Path) -> bool {
        let text = path.to_string_lossy();
        if text.contains("/target/") || text.ends_with("/target") {
            return false;
        }

        match path.extension().and_then(|ext| ext.to_str()) {
            Some("rs") | Some("toml") => true,
            _ => path.file_name().and_then(|name| name.to_str()) == Some("Cargo.toml"),
        }
    }
}

#[cfg(debug_assertions)]
struct DevReloadContext {
    cargo_root: PathBuf,
    executable: PathBuf,
    executable_args: Vec<String>,
    build_target: BuildTarget,
    watch_paths: Vec<PathBuf>,
}

#[cfg(debug_assertions)]
enum BuildTarget {
    Example(String),
    Bin(String),
    Generic,
}

#[cfg(debug_assertions)]
impl DevReloadContext {
    fn discover() -> Option<Self> {
        let executable = std::env::current_exe().ok()?;
        let executable_args = std::env::args().skip(1).collect::<Vec<_>>();
        let cargo_root = Self::find_cargo_root(&executable)?;
        let build_target = Self::detect_build_target(&cargo_root, &executable);
        let watch_paths = Self::collect_watch_paths(&cargo_root);

        Some(Self {
            cargo_root,
            executable,
            executable_args,
            build_target,
            watch_paths,
        })
    }

    fn find_cargo_root(path: &Path) -> Option<PathBuf> {
        let mut cursor = path.parent();
        while let Some(dir) = cursor {
            if dir.join("Cargo.toml").exists() {
                return Some(dir.to_path_buf());
            }
            cursor = dir.parent();
        }
        None
    }

    fn detect_build_target(cargo_root: &Path, executable: &Path) -> BuildTarget {
        let target_debug = cargo_root.join("target").join("debug");

        if let Ok(relative) = executable.strip_prefix(&target_debug) {
            let parts = relative
                .components()
                .map(|part| part.as_os_str().to_string_lossy().to_string())
                .collect::<Vec<_>>();

            if parts.len() >= 2 && parts[0] == "examples" {
                return BuildTarget::Example(parts[1].clone());
            }

            if parts.len() == 1 && !parts[0].is_empty() {
                return BuildTarget::Bin(parts[0].clone());
            }
        }

        BuildTarget::Generic
    }

    fn collect_watch_paths(cargo_root: &Path) -> Vec<PathBuf> {
        let mut paths = Vec::new();

        for rel in ["src", "examples", "tests"] {
            let candidate = cargo_root.join(rel);
            if candidate.exists() {
                paths.push(candidate);
            }
        }

        let manifest = cargo_root.join("Cargo.toml");
        if manifest.exists() {
            paths.push(manifest);
        }

        paths
    }

    fn register_watchers(&self, watcher: &mut RecommendedWatcher) -> notify::Result<()> {
        for path in &self.watch_paths {
            let mode = if path.is_dir() {
                RecursiveMode::Recursive
            } else {
                RecursiveMode::NonRecursive
            };
            watcher.watch(path, mode)?;
        }
        Ok(())
    }

    fn build(&self) -> bool {
        let mut command = Command::new("cargo");
        command.current_dir(&self.cargo_root);

        match &self.build_target {
            BuildTarget::Example(name) => {
                command.arg("build").arg("--example").arg(name);
            }
            BuildTarget::Bin(name) => {
                command.arg("build").arg("--bin").arg(name);
            }
            BuildTarget::Generic => {
                command.arg("build");
            }
        }

        match command.status() {
            Ok(status) => status.success(),
            Err(err) => {
                log_error!("Failed to run cargo build: {}", err);
                false
            }
        }
    }

    fn spawn_child(&self) -> std::io::Result<Child> {
        Command::new(&self.executable)
            .args(&self.executable_args)
            .env(DEV_RELOAD_CHILD_ENV, "1")
            .env(DEV_RELOAD_ENV, "0")
            .spawn()
    }
}

#[cfg(all(test, debug_assertions))]
mod tests {
    use super::Unipotato;

    #[test]
    fn dev_flag_parser_understands_common_truthy_values() {
        assert!(Unipotato::is_truthy_flag("true"));
        assert!(Unipotato::is_truthy_flag("1"));
        assert!(Unipotato::is_truthy_flag("YES"));
        assert!(!Unipotato::is_truthy_flag("off"));
        assert!(!Unipotato::is_truthy_flag(""));
    }
}

/// Route incoming requests to registered handlers
async fn router(req: hyper::Request<Incoming>) -> Result<Response<String>, Infallible> {
    let method = req.method().clone();
    let path = req.uri().path().to_string();
    
    log_request!(method.as_str(), &path);
    
    let response = match find_handler_with_params(&method, &path) {
        Some(HandlerMatch { handler, params }) => {
            // Create our custom Request with extracted path parameters
            let custom_req = Request::with_params(req, params);
            handler(custom_req).await
        }
        None => not_found(),
    };
    
    let status = response.status().as_u16();
    log_response!(status, &path);
    
    Ok(response)
}