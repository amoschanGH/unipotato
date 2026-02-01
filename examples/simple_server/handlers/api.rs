use unipotato::{Request, Response, handler::json, get, post, put, delete, log_info};
use crate::models::{User, ApiResponse};
use crate::handlers::utils::{parse_user_request};
use crate::get_db;

#[get("/users")]
pub async fn get_users(_req: Request) -> Response {
    let db = get_db();
    let data = db.lock().unwrap();
    log_info!("Fetching all users");
    json(data.users.clone())
}

#[get("/users/<id>")]
pub async fn get_user(req: Request) -> Response {
    let db = get_db();
    let data = db.lock().unwrap();
    
    // NEW: Use req.param() to get path parameter directly!
    let id: u32 = req.param_as("id").unwrap_or(0);
    log_info!("Fetching user with id: {}", id);
    
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

#[put("/users/<ouob>")]
pub async fn update_user(req: Request) -> Response {
    let db = get_db();
    let id: u32 = req.param_as("ouob").unwrap_or(0);
    
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

#[delete("/users/<id>")]
pub async fn delete_user(req: Request) -> Response {
    let db = get_db();
    let id: u32 = req.param_as("id").unwrap_or(0);
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

/// Slow endpoint to test async behavior - sleeps for 3 seconds
#[get("/slow")]
pub async fn slow_endpoint(_req: Request) -> Response {
    log_info!("Starting slow request (3 second delay)...");
    tokio::time::sleep(std::time::Duration::from_secs(3)).await;
    log_info!("Slow request completed!");
    json(ApiResponse::success("Slow response after 3 seconds".to_string()))
}

