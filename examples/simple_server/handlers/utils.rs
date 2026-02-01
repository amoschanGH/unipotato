use unipotato::{Request, Body};
use std::collections::HashMap;
use crate::models::CreateUserRequest;

pub fn extract_id(req: &Request) -> u32 {
    // Get the path from the request URI
    let path = req.uri().path();
    
    // Split by '/' and get the last segment that looks like an ID
    path.split('/')
        .filter(|s| !s.is_empty())
        .last()
        .and_then(|s| s.parse::<u32>().ok())
        .unwrap_or(0)
}

pub fn extract_path_params(req: &Request, name: &str) -> u32 {
    req.extensions()
        .get::<HashMap<String, String>>()
        .and_then(|params| params.get(name))
        .and_then(|val| val.parse().ok())
        .unwrap_or(0)
}

pub async fn parse_user_request(req: Request) -> Result<CreateUserRequest, String> {
    let body = Body::from_incoming(req.into_body()).await?;
    body.json::<CreateUserRequest>()
}
