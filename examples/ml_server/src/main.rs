mod routes;
pub mod handlers;
pub mod training_state;

use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;
use std::time::Duration;
use unipotato::Unipotato;

static SHUTDOWN_TRIGGERED: AtomicBool = AtomicBool::new(false);

fn main() {
    println!("🥔 Starting ml_server…");
    println!("📍 Server: http://localhost:8080");
    println!("🧠 MNIST Dashboard: http://localhost:8080/training");

    ctrlc::set_handler(|| {
        let was_set = SHUTDOWN_TRIGGERED.swap(true, Ordering::AcqRel);
        if !was_set {
            println!("\n⚠️  Shutdown signal received. Stopping training and server...");
            training_state::set_shutdown();
            // Give background training a short chance to observe shutdown.
            thread::sleep(Duration::from_millis(300));
            std::process::exit(0);
        } else {
            // Second signal: exit immediately.
            std::process::exit(130);
        }
    })
    .expect("failed to install Ctrl+C handler");

    let app = Unipotato::launch(8080);
    routes::setup_routes(app).start();

    println!("✓ Server stopped");
}
