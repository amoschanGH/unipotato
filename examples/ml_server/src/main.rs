mod routes;
pub mod handlers;
pub mod training_state;

use unipotato::Unipotato;

fn main() {
    println!("🥔 Starting ml_server…");
    println!("📍 Server: http://localhost:8080");
    println!("🧠 MNIST Dashboard: http://localhost:8080/training");


    let app = Unipotato::launch(8080);
    routes::setup_routes(app).start();
}
