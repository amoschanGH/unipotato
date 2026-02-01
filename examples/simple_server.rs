use unipotato::{Request, Response, handler::{html, json}, Unipotato, get, post, routes, Query, Body};
use serde::{Serialize, Deserialize};

// === Data Models ===

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

#[derive(Deserialize)]
struct CreateUserRequest {
    name: String,
    email: String,
}

#[derive(Deserialize)]
struct ContactFormData {
    email: String,
    message: Option<String>,
}

// === Helper Functions ===

impl ApiResponse {
    fn success(message: impl Into<String>) -> Self {
        Self {
            status: "success".to_string(),
            message: message.into(),
        }
    }

    fn error(message: impl Into<String>) -> Self {
        Self {
            status: "error".to_string(),
            message: message.into(),
        }
    }
}

// === Root Routes ===

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

// === API Routes ===

mod api_routes {
    use super::*;
    
    #[get("/users")]
    pub async fn get_users(req: Request) -> Response {
        let query = Query::from_uri(req.uri());
        let limit = query.get("limit")
            .and_then(|s| s.parse::<usize>().ok())
            .unwrap_or(10);
        
        let users = get_mock_users();
        let limited_users: Vec<_> = users.into_iter().take(limit).collect();
        
        json(limited_users)
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
    pub async fn create_user(req: Request) -> Response {
        match parse_user_request(req).await {
            Ok(user_data) => {
                let response = ApiResponse::success(
                    format!("User {} created successfully", user_data.name)
                );
                json(response)
            }
            Err(error_msg) => {
                json(ApiResponse::error(error_msg))
            }
        }
    }

    #[post("/contact")]
    pub async fn contact_form(req: Request) -> Response {
        match parse_contact_form(req).await {
            Ok(email) => {
                let response = ApiResponse::success(
                    format!("Thank you {}! We'll contact you soon.", email)
                );
                json(response)
            }
            Err(error_msg) => {
                json(ApiResponse::error(error_msg))
            }
        }
    }

    #[post("/users/1")]
    pub async fn update_user(_req: Request) -> Response {
        json(ApiResponse::success("User updated successfully"))
    }

    #[get("/health")]
    pub async fn health_check(_req: Request) -> Response {
        json(ApiResponse {
            status: "ok".to_string(),
            message: "Server is running".to_string(),
        })
    }

    // === Helper Functions ===

    fn get_mock_users() -> Vec<User> {
        vec![
            User { id: 1, name: "Alice".to_string(), email: "alice@example.com".to_string() },
            User { id: 2, name: "Bob".to_string(), email: "bob@example.com".to_string() },
            User { id: 3, name: "Charlie".to_string(), email: "charlie@example.com".to_string() },
        ]
    }

    async fn parse_user_request(req: Request) -> Result<CreateUserRequest, String> {
        let body = Body::from_incoming(req.into_body()).await?;
        body.json::<CreateUserRequest>()
    }

    async fn parse_contact_form(req: Request) -> Result<String, String> {
        let body = Body::from_incoming(req.into_body()).await?;
        
        // Try JSON first
        if let Ok(form_data) = body.json::<ContactFormData>() {
            return Ok(form_data.email);
        }
        
        // Fallback to form data
        let form = body.form()?;
        form.get("email")
            .cloned()
            .ok_or_else(|| "Email field is required".to_string())
    }
}

// === Main ===

fn main() {
    Unipotato::launch(8080)
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