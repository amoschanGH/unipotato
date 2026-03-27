//! Python integration handler via `rustpy-ml`.
//!
//! Make sure Python 3 is installed on the target machine.

use rustpy_ml::prelude::*;
use unipotato::{Request, Response, handler::{json, text}, get};
use serde::Serialize;

#[derive(Serialize)]
struct PyResult {
    result: String,
}

/// GET /py/run
/// Runs a trivial inline Python expression and returns the result as JSON.
#[get("/run")]
pub async fn run_script(_req: Request) -> Response {
    rustpy_ml::init().ok();

    let outcome: rustpy_ml::Result<String> = (|| {
        let val: String = python!(-> String, "str(1 + 1)")?;
        Ok(val)
    })();

    match outcome {
        Ok(val) => json(PyResult { result: val }),
        Err(e)  => text(format!("Python error: {e}")),
    }
}
