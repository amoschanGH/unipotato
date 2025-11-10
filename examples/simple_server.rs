use unipotato::{Request, Response, handler::{html, json}, Server, get, post, routes};
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

// Root routes module
mod root_routes {
    use super::*;
    
    #[get("/")]
    pub async fn index(_req: Request) -> Response {
        html("<h1>Welcome to Unipotato!</h1><p>Check out <a href='/about'>About</a> or <a href='/api/users'>Users API</a></p>")
    }

    #[get("/about")]
    pub async fn about(_req: Request) -> Response {
        html("<h1>About</h1><p>This is a simple web framework built with Rust</p>")
    }

    #[get("/contact")]
    pub async fn contact(_req: Request) -> Response {
        html("<h1>Contact Us</h1><form method='post' action='/api/contact'><input name='email' placeholder='Your email'/><button>Submit</button></form>")
    }
}

// API routes module
mod api_routes {
    use super::*;
    
    #[get("/users")]
    pub async fn get_users(_req: Request) -> Response {
        let users = vec![
            User { id: 1, name: "Alice".to_string(), email: "alice@example.com".to_string() },
            User { id: 2, name: "Bob".to_string(), email: "bob@example.com".to_string() },
            User { id: 3, name: "Charlie".to_string(), email: "charlie@example.com".to_string() },
        ];
        json(users)
    }

    #[get("/users/1")]
    pub async fn get_user(_req: Request) -> Response {
        let user = User {
            id: 1,
            name: "Alice".to_string(),
            email: "alice@example.com".to_string(),
        };
        json(user)
    }

    #[post("/users")]
    pub async fn create_user(_req: Request) -> Response {
        let response = ApiResponse {
            status: "success".to_string(),
            message: "User created successfully".to_string(),
        };
        json(response)
    }

    #[post("/contact")]
    pub async fn contact_form(_req: Request) -> Response {
        let response = ApiResponse {
            status: "success".to_string(),
            message: "Thank you for contacting us!".to_string(),
        };
        json(response)
    }

    #[post("/users/1")]
    pub async fn update_user(_req: Request) -> Response {
        let response = ApiResponse {
            status: "success".to_string(),
            message: "User updated successfully".to_string(),
        };
        json(response)
    }

    #[get("/health")]
    pub async fn health_check(_req: Request) -> Response {
        json(ApiResponse {
            status: "ok".to_string(),
            message: "Server is running".to_string(),
        })
    }
}

fn main() {
    Server::new(8000)
        .mount("/", routes![
            root_routes::index,
            root_routes::about,
            root_routes::contact,
        ])
        .mount("/api", routes![
            api_routes::get_users,
            api_routes::get_user,
            api_routes::create_user,
            api_routes::contact_form,
            api_routes::update_user,
            api_routes::health_check,
        ])
        .start();
}