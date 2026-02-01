use unipotato::{Request, Response, handler::{html, json}, Unipotato, get, post, put, delete, routes, Query, Body};
use serde::{Serialize, Deserialize};
use std::sync::{Arc, Mutex, OnceLock};
use std::fs;
use std::path::Path;

// === Global Database ===

static DB: OnceLock<Arc<Mutex<Database>>> = OnceLock::new();

fn get_db() -> Arc<Mutex<Database>> {
    DB.get().unwrap().clone()
}

// === Storage Module ===

#[derive(Serialize, Deserialize, Clone, Default, Debug)]
struct Database {
    users: Vec<User>,
    next_user_id: u32,
}

impl Database {
    fn load() -> Self {
        if Path::new("database.json").exists() {
            let content = fs::read_to_string("database.json").unwrap_or_default();
            serde_json::from_str(&content).unwrap_or_else(|_| Self::default_data())
        } else {
            Self::default_data()
        }
    }

    fn save(&self) {
        let json = serde_json::to_string_pretty(self).unwrap();
        fs::write("database.json", json).ok();
    }

    fn default_data() -> Self {
        Self {
            users: vec![
                User { id: 1, name: "Alice".to_string(), email: "alice@example.com".to_string() },
                User { id: 2, name: "Bob".to_string(), email: "bob@example.com".to_string() },
                User { id: 3, name: "Charlie".to_string(), email: "charlie@example.com".to_string() },
            ],
            next_user_id: 4,
        }
    }
}

// === Data Models ===

#[derive(Serialize, Deserialize, Clone, Debug)]
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
        html(r#"
            <!DOCTYPE html>
            <html>
            <head>
                <title>Unipotato CRUD App</title>
                <style>
                    body { font-family: Arial, sans-serif; max-width: 800px; margin: 0 auto; padding: 20px; }
                    h1 { color: #ff6b35; }
                    .user-item { background: #f5f5f5; padding: 15px; margin: 10px 0; border-radius: 5px; }
                    .user-item strong { color: #333; }
                    button { background: #ff6b35; color: white; border: none; padding: 8px 15px; margin: 5px; cursor: pointer; border-radius: 3px; }
                    button:hover { background: #ff5722; }
                    .delete-btn { background: #dc3545; }
                    .delete-btn:hover { background: #c82333; }
                    input, button { margin: 5px 0; padding: 8px; }
                    input { width: 200px; }
                    form { margin: 20px 0; padding: 20px; background: #f9f9f9; border-radius: 5px; }
                </style>
            </head>
            <body>
                <h1>Unipotato CRUD Application</h1>
                <p>Complete CRUD operations with JSON file storage</p>
                
                <h2>Users</h2>
                <div id="users-list"></div>
                
                <h2>Create New User</h2>
                <form onsubmit="createUser(event)">
                    <input type="text" id="name" placeholder="Name" required><br>
                    <input type="email" id="email" placeholder="Email" required><br>
                    <button type="submit">Create User</button>
                </form>

                <script>
                    async function loadUsers() {
                        const res = await fetch('/api/users');
                        const users = await res.json();
                        const html = users.map(u => `
                            <div class="user-item">
                                <strong>${u.name}</strong> - ${u.email}
                                <div>
                                    <button onclick="editUser(${u.id}, '${u.name}', '${u.email}')">Edit</button>
                                    <button class="delete-btn" onclick="deleteUser(${u.id})">Delete</button>
                                </div>
                            </div>
                        `).join('');
                        document.getElementById('users-list').innerHTML = html;
                    }

                    async function createUser(e) {
                        e.preventDefault();
                        const name = document.getElementById('name').value;
                        const email = document.getElementById('email').value;
                        await fetch('/api/users', {
                            method: 'POST',
                            headers: {'Content-Type': 'application/json'},
                            body: JSON.stringify({name, email})
                        });
                        document.getElementById('name').value = '';
                        document.getElementById('email').value = '';
                        loadUsers();
                    }

                    async function editUser(id, name, email) {
                        const newName = prompt('Enter new name:', name);
                        const newEmail = prompt('Enter new email:', email);
                        if (newName && newEmail) {
                            await fetch('/api/users/' + id, {
                                method: 'PUT',
                                headers: {'Content-Type': 'application/json'},
                                body: JSON.stringify({name: newName, email: newEmail})
                            });
                            loadUsers();
                        }
                    }

                    async function deleteUser(id) {
                        if (confirm('Are you sure you want to delete this user?')) {
                            await fetch('/api/users/' + id, {method: 'DELETE'});
                            loadUsers();
                        }
                    }

                    loadUsers();
                </script>
            </body>
            </html>
        "#)
    }

    #[get("/about")]
    pub async fn about(_req: Request) -> Response {
        html("<h1>About</h1><p>This is a simple CRUD app with JSON storage</p><a href='/'>Back</a>")
    }

    #[get("/contact")]
    pub async fn contact(_req: Request) -> Response {
        html("<h1>Contact Us</h1><form method='post' action='/api/contact'><input name='email' placeholder='Your email'/><button>Submit</button></form>")
    }
}

// === API Routes ===

mod api_routes {
    use super::*;
    use std::collections::HashMap;
    
    #[get("/users")]
    pub async fn get_users(_req: Request) -> Response {
        let db = get_db();
        let data = db.lock().unwrap();
        json(data.users.clone())
    }

    #[get("/users/:id")]
    pub async fn get_user(req: Request) -> Response {
        let db = get_db();
        let data = db.lock().unwrap();
        let id = extract_id(&req);
        
        match data.users.iter().find(|u| u.id == id) {
            Some(user) => json(user.clone()),
            None => json(ApiResponse::error("User not found"))
        }
    }

    #[post("/users")]
    pub async fn create_user(req: Request) -> Response {
        let db = get_db();
        match parse_user_request(req).await {
            Ok(user_data) => {
                let mut data = db.lock().unwrap();
                let new_user = User {
                    id: data.next_user_id,
                    name: user_data.name,
                    email: user_data.email,
                };
                data.next_user_id += 1;
                data.users.push(new_user.clone());
                data.save();
                json(new_user)
            }
            Err(error_msg) => json(ApiResponse::error(error_msg))
        }
    }

    #[put("/users/:id")]
    pub async fn update_user(req: Request) -> Response {
        let db = get_db();
        let id = extract_id(&req);
        
        match parse_user_request(req).await {
            Ok(user_data) => {
                let mut data = db.lock().unwrap();
                if let Some(user) = data.users.iter_mut().find(|u| u.id == id) {
                    user.name = user_data.name;
                    user.email = user_data.email;
                    data.save();
                    json(ApiResponse::success(format!("User {} updated", id)))
                } else {
                    json(ApiResponse::error("User not found"))
                }
            }
            Err(error_msg) => json(ApiResponse::error(error_msg))
        }
    }

    #[delete("/users/:id")]
    pub async fn delete_user(req: Request) -> Response {
        let db = get_db();
        let id = extract_id(&req);
        let mut data = db.lock().unwrap();
        
        if let Some(pos) = data.users.iter().position(|u| u.id == id) {
            data.users.remove(pos);
            data.save();
            json(ApiResponse::success(format!("User {} deleted", id)))
        } else {
            json(ApiResponse::error("User not found"))
        }
    }

    #[get("/health")]
    pub async fn health_check(_req: Request) -> Response {
        json(ApiResponse {
            status: "ok".to_string(),
            message: "Server is running".to_string(),
        })
    }

    // === Helper Functions ===

    fn extract_id(req: &Request) -> u32 {
        // Get from extensions (set by router)
        req.extensions()
            .get::<HashMap<String, String>>()
            .and_then(|params| params.get("id"))
            .and_then(|val| val.parse().ok())
            .unwrap_or(0)
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
    println!("🥔 Starting Unipotato CRUD Server...");
    println!("📍 Server running at: http://localhost:8000");
    println!("💾 Data will be stored in: database.json");
    println!("\n");

    // Initialize global database
    let db = Arc::new(Mutex::new(Database::load()));
    DB.set(db).expect("Failed to initialize database");

    Unipotato::launch(8000)
        .mount("/", routes![
            root_routes::index,
            root_routes::about,
        ])
        .mount("/api", routes![
            api_routes::get_users,
            api_routes::get_user,
            api_routes::create_user,
            api_routes::update_user,
            api_routes::delete_user,
            api_routes::health_check,
        ])
        .start();
}