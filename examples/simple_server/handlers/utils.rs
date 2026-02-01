use unipotato::{Request, Body};
use std::collections::HashMap;
use crate::models::CreateUserRequest;

pub fn extract_id(req: &Request) -> u32 {
    req.extensions()
        .get::<HashMap<String, String>>()
        .and_then(|params| params.get("id"))
        .and_then(|val| val.parse().ok())
        .unwrap_or(0)
}

pub async fn parse_user_request(req: Request) -> Result<CreateUserRequest, String> {
    let body = Body::from_incoming(req.into_body()).await?;
    body.json::<CreateUserRequest>()
}
