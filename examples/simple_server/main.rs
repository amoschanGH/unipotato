mod models;
mod database;
mod routes;
pub mod handlers;

use unipotato::{Unipotato, routes};
use database::Database;
use std::sync::{Arc, Mutex, OnceLock};

// Import handlers directly
use handlers::root;
use handlers::api;

static DB: OnceLock<Arc<Mutex<Database>>> = OnceLock::new();

pub fn get_db() -> Arc<Mutex<Database>> {
    DB.get().unwrap().clone()
}

fn main() {
    println!("🥔 Starting Unipotato CRUD Server...");
    println!("📍 Server running at: http://localhost:8000");
    println!("💾 Data will be stored in: database.json");
    println!("\n");

    // Initialize global database
    let db = Arc::new(Mutex::new(Database::load()));
    DB.set(db).expect("Failed to initialize database");

    Unipotato::launch(8000)
        .mount("/", routes![
            root::index,
            root::about,
        ])
        .mount("/api", routes![
            api::get_users,
            api::get_user,
            api::create_user,
            api::update_user,
            api::delete_user,
            api::health_check,
        ])
        .start();
}
