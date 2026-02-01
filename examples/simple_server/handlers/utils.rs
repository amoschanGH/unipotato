use unipotato::Request;
use crate::models::CreateUserRequest;

/// Extract an ID from a path parameter
/// Uses the new req.param() API for clean parameter extraction
pub fn extract_id(req: &Request) -> u32 {
    // Try to get "id" parameter from the route pattern
    req.param_as::<u32>("id").unwrap_or_else(|| {
        // Fallback: parse from path for backwards compatibility
        let path = req.uri().path();
        path.split('/')
            .filter(|s| !s.is_empty())
            .last()
            .and_then(|s| s.parse::<u32>().ok())
            .unwrap_or(0)
    })
}

/// Extract a named path parameter as u32
pub fn extract_path_param(req: &Request, name: &str) -> u32 {
    req.param_as::<u32>(name).unwrap_or(0)
}

pub async fn parse_user_request(req: Request) -> Result<CreateUserRequest, String> {
    let body = req.into_body().await?;
    body.json::<CreateUserRequest>()
}
