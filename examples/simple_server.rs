use unipotato::{Request, Response, handler::{html, json}, Server, get, post};
use serde::Serialize;

#[derive(Serialize)]
struct User {
    id: u32,
    name: String,
    email: String,
}

#[derive(Serialize)]
struct ApiResponse {
    status: String,
    message: String,
}

#[get("/")]
async fn index(_req: Request) -> Response {
    html("<h1>Welcome to Unipotato!</h1><p>Check out <a href='/about'>About</a> or <a href='/api/users'>Users API</a></p>")
}

#[get("/about")]
async fn about(_req: Request) -> Response {
    html("<h1>About</h1><p>This is a simple web framework built with Rust</p>")
}

#[get("/contact")]
async fn contact(_req: Request) -> Response {
    html("<h1>Contact Us</h1><form method='post' action='/api/contact'><input name='email' placeholder='Your email'/><button>Submit</button></form>")
}

#[get("/api/users")]
async fn get_users(_req: Request) -> Response {
    let users = vec![
        User { id: 1, name: "Alice".to_string(), email: "alice@example.com".to_string() },
        User { id: 2, name: "Bob".to_string(), email: "bob@example.com".to_string() },
        User { id: 3, name: "Charlie".to_string(), email: "charlie@example.com".to_string() },
    ];
    json(users)
}

#[get("/api/users/1")]
async fn get_user(_req: Request) -> Response {
    let user = User {
        id: 1,
        name: "Alice".to_string(),
        email: "alice@example.com".to_string(),
    };
    json(user)
}

#[post("/api/users")]
async fn create_user(_req: Request) -> Response {
    let response = ApiResponse {
        status: "success".to_string(),
        message: "User created successfully".to_string(),
    };
    json(response)
}

#[post("/api/contact")]
async fn contact_form(_req: Request) -> Response {
    let response = ApiResponse {
        status: "success".to_string(),
        message: "Thank you for contacting us!".to_string(),
    };
    json(response)
}

#[post("/api/users/1")]
async fn update_user(_req: Request) -> Response {
    let response = ApiResponse {
        status: "success".to_string(),
        message: "User updated successfully".to_string(),
    };
    json(response)
}

// #[delete("/api/users/1")]
// async fn delete_user(_req: Request) -> Response {
//     let response = ApiResponse {
//         status: "success".to_string(),
//         message: "User deleted successfully".to_string(),
//     };
//     json(response)
// }

#[get("/api/health")]
async fn health_check(_req: Request) -> Response {
    json(ApiResponse {
        status: "ok".to_string(),
        message: "Server is running".to_string(),
    })
}

fn main() {
    Server::new(8000).start();
}