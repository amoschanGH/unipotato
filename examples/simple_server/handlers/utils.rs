use unipotato::Request;
use crate::models::CreateUserRequest;

pub async fn parse_user_request(req: Request) -> Result<CreateUserRequest, String> {
    let body = req.into_body().await?;
    body.json::<CreateUserRequest>()
}
