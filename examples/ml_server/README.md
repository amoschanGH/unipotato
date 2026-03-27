# ml_server

A [Unipotato](https://github.com/amoschanGH/unipotato) web server.

## Running

```bash
cargo run
```

Server starts on <http://localhost:8080>.

## Enabled features

- HTML templates via `handlers/template.rs`
- Python integration via `rustpy` / PyO3 (`handlers/py.rs`)

## Project layout

```
src/
  main.rs          – entry point
  routes.rs        – mount routes here
  handlers/
    mod.rs         – re-exports
    root.rs        – root / health handlers
    template.rs    – HTML template handler
    py.rs          – Python integration handler
templates/
  index.html       – example HTML template
```
