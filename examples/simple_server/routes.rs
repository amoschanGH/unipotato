use unipotato::{Unipotato, routes};
use crate::handlers::{root, api};

pub fn setup_routes(app: Unipotato) -> Unipotato {
    app.mount("/", routes![
        root::index,
        root::about,
    ])
    .mount("/api", routes![
        api::get_users,
        api::get_user,
        api::create_user,
        api::update_user,
        api::delete_user,
        api::health_check,
        api::slow_endpoint,
        // Multi-param examples
        api::get_user_post,
        api::get_post_comment,
        api::get_product_by_category,
    ])
}
