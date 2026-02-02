mod models;
mod database;
mod routes;
pub mod handlers;

use unipotato::Unipotato;
use database::Database;
use std::sync::{Arc, Mutex, OnceLock};

static DB: OnceLock<Arc<Mutex<Database>>> = OnceLock::new();

pub fn get_db() -> Arc<Mutex<Database>> {
    DB.get().unwrap().clone()
}

fn main() {
    println!("🥔 Starting Unipotato CRUD Server...");
    println!("📍 Server running at: http://localhost:8080");
    println!("💾 Data will be stored in: database.json");
    println!("\n");

    // Initialize global database
    let db = Arc::new(Mutex::new(Database::load()));
    DB.set(db).expect("Failed to initialize database");

    let app = Unipotato::launch(8080);
    routes::setup_routes(app).start();
}
