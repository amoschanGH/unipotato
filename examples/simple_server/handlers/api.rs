use unipotato::{Request, Response, handler::json, get, post, put, delete};
use crate::models::{User, ApiResponse};
use crate::handlers::utils::{extract_id, parse_user_request};
use crate::get_db;

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
