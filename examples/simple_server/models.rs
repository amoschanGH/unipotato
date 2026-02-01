use serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct User {
    pub id: u32,
    pub name: String,
    pub email: String,
}

#[derive(Serialize)]
pub struct ApiResponse {
    pub status: String,
    pub message: String,
}

impl ApiResponse {
    pub fn success(message: impl Into<String>) -> Self {
        Self {
            status: "success".to_string(),
            message: message.into(),
        }
    }

    pub fn error(message: impl Into<String>) -> Self {
        Self {
            status: "error".to_string(),
            message: message.into(),
        }
    }
}

#[derive(Deserialize)]
pub struct CreateUserRequest {
    pub name: String,
    pub email: String,
}

#[derive(Deserialize)]
pub struct ContactFormData {
    pub email: String,
    pub message: Option<String>,
}
