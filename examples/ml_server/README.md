# ml_server

MNIST CNN training and inference server powered by [Unipotato](https://github.com/amoschanGH/unipotato) and `rustpy-ml`.

## Running

```bash
cargo run
```

Server starts on <http://localhost:8080>.

Dashboard: <http://localhost:8080/training>

## Python requirements

The training backend uses Python packages through `rustpy-ml`:

- `numpy`
- `torch`
- `torchvision`

MNIST is downloaded automatically on first training run.

## Training API

- `POST /train/start`
- `POST /train/pause`
- `POST /train/resume`
- `POST /train/stop`
- `GET /train/status`
- `POST /train/reset`
- `GET /train/model` (download checkpoint)
- `POST /train/model/upload` (upload checkpoint bytes)
- `POST /train/model/load` (activate uploaded checkpoint)
- `POST /train/infer-drawing` (predict from normalized 28x28 pixels)

## Features

- Background MNIST CNN training with pause/resume/stop.
- Live metrics with loss/accuracy charts and logs.
- Draw-to-infer UI with auto-center and normalization.
- Checkpoint download, upload, and reload.
