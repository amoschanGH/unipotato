# Unipotato

Unipotato is a macro-based Rust backend framework focused on simple routing, async handlers, and clean request parsing.

It is designed for Rust developers who want to build HTTP backends with minimal boilerplate while keeping full control over application logic.

## Contents

1. What You Get
2. Installation
3. Quick Start
4. Route Registration Model
5. Request Handling
6. Response Helpers
7. Auto Reload for Development
8. Running the Example Project
9. API Reference Summary
10. Notes and Limitations
11. License

## 1. What You Get

- Attribute-based routes: get, post, put, delete, patch
- Mountable route groups using a base path
- Path parameter extraction with named placeholders
- Query and body parsing helpers
- Async request handling on Tokio + Hyper
- Dev auto reload mode (rebuild and restart)

## 2. Installation

Add Unipotato to your Cargo.toml dependencies:

    [dependencies]
    unipotato = "0.1.3"

## 3. Quick Start

### 3.1 Minimal Server

    use unipotato::{Unipotato, Request, Response, handler, get, routes};

    #[get("/")]
    pub async fn index(_req: Request) -> Response {
        handler::text("Hello from Unipotato")
    }

    fn main() {
        Unipotato::launch(8080)
            .mount("/", routes![index])
            .start();
    }

### 3.2 Grouped Routes

    use unipotato::{Unipotato, routes};
    use crate::handlers::{root, api};

    fn main() {
        Unipotato::launch(8080)
            .mount("/", routes![
                root::index,
                root::about,
            ])
            .mount("/api", routes![
                api::get_users,
                api::create_user,
            ])
            .start();
    }

## 4. Route Registration Model

Unipotato uses attribute macros on async handler functions.

Supported route attributes:

- get("/path")
- post("/path")
- put("/path")
- delete("/path")
- patch("/path")

Path parameters use angle-bracket placeholders.

Example:

    #[get("/users/<id>")]
    pub async fn get_user(req: Request) -> Response {
        let id: u32 = req.param_as("id").unwrap_or(0);
        handler::text(format!("User {}", id))
    }

You register handlers by mounting with routes!.

    app.mount("/api", routes![api::get_user, api::get_users])

## 5. Request Handling

Unipotato wraps Hyper requests in a custom Request type.

### 5.1 Path Params

- req.param("name") -> Option<&str>
- req.param_as::<T>("name") -> Option<T>
- req.params() -> HashMap<String, String>

Example:

    let user_id: u32 = req.param_as("user_id").unwrap_or(0);

### 5.2 Query Params

    let query = req.query();
    let page = query.get_or("page", "1");
    let has_filter = query.has("filter");

### 5.3 Body Parsing

Body parsing is async because it consumes the incoming stream.

    pub async fn create_user(req: Request) -> Response {
        match req.into_body().await {
            Ok(body) => {
                let parsed: Result<serde_json::Value, _> = body.json();
                match parsed {
                    Ok(v) => handler::json(v),
                    Err(e) => handler::text(format!("Invalid JSON: {}", e)),
                }
            }
            Err(e) => handler::text(format!("Body error: {}", e)),
        }
    }

Body helper methods:

- body.as_str()
- body.as_bytes()
- body.json::<T>()
- body.form()
- body.is_empty()
- body.len()

## 6. Response Helpers

Use helper functions from handler module for common content types.

- handler::text(...)
- handler::html(...)
- handler::json(...)
- handler::not_found()

All handlers return unipotato::Response.

## 7. Auto Reload for Development

Unipotato includes development auto reload support to improve backend DX.

This is a rebuild + restart workflow, not in-process hot swap.

### 7.1 Enable Auto Reload

    UNIPOTATO_DEV_RELOAD=1 cargo run --bin your_app

If you run an example target:

    UNIPOTATO_DEV_RELOAD=1 cargo run --example simple_server

### 7.2 Behavior

When enabled in debug builds:

1. Unipotato starts a supervisor process.
2. Supervisor watches project files.
3. On relevant change, it runs cargo build.
4. If build succeeds, it restarts the child server.
5. If build fails, current child keeps running.

### 7.3 Watched Paths

- src
- examples
- tests
- Cargo.toml

### 7.4 Notes

- Default behavior is unchanged when auto reload is not enabled.
- Existing API flow remains the same: launch -> mount -> start.
- In-memory state is reset after each restart.

## 8. Running the Example Project

This repository includes a CRUD example server.

    cargo run --example simple_server

Then open:

- http://localhost:8080
- http://localhost:8080/about
- http://localhost:8080/api/health

## 9. API Reference Summary

Core types and exports:

- Unipotato
- Request
- Query
- Body
- Response
- Route
- Handler

Core server methods:

- Unipotato::launch(port)
- .mount(base, routes)
- .start()
- .port()

Macros:

- launch!()
- launch!(port)
- routes![module::handler, ...]
- get, post, put, delete, patch

## 10. Notes and Limitations

- Route matching supports path placeholders with angle brackets.
- Route registration happens through macro-driven setup at startup.
- Auto reload is intended for local development, not production process management.

## 11. License

Apache-2.0
