use unipotato::{Unipotato, routes};
use crate::handlers::root;
use crate::handlers::template;
use crate::handlers::py;
use crate::handlers::train;
pub fn setup_routes(app: Unipotato) -> Unipotato {
    app.mount("/", routes![
        root::index,
        root::health,
    ])
    .mount("/template", routes![
        template::index,
    ])
    .mount("/training", routes![
        template::training_dashboard,
    ])
    .mount("/py", routes![
        py::run_script,
    ])
    .mount("/train", routes![
        train::start_training,
        train::pause_training,
        train::resume_training,
        train::stop_training,
        train::get_status,
        train::reset_training,
        train::download_model,
    ])
}
