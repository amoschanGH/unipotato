//! MNIST CNN training handler with pause/resume/stop control.
//!
//! Uses rustpy-ml to run PyTorch + torchvision training and supports
//! drawing inference plus checkpoint upload/reload.

use rustpy_ml::prelude::*;
use unipotato::{
    get, post,
    handler::{json, text},
    Request, Response,
};

use base64::Engine as _;
use once_cell::sync::Lazy;
use pyo3::Python;
use pyo3::types::PyList;
use serde::{Deserialize, Serialize};
use std::sync::{Arc, Mutex, MutexGuard};
use std::time::Instant;

use crate::training_state::{
    InferenceResult, TrainingConfig, TrainingControlFlags, TrainingMetrics, TrainingState,
    TrainingStatus,
};
use crate::timing::{
    start_request, record_body_read_done, record_json_parse_done,
    record_spawn_block_enter, record_spawn_block_acquired, record_inference_done,
    record_response_serialize_done, record_response_send
};

// ============================================================================
// Training Implementation Using rustpy-ml
// ============================================================================

/// Initialize Python ML environment - runs once.
fn init_python_ml() -> rustpy_ml::Result<()> {
    static INIT: Lazy<()> = Lazy::new(|| {
        let _ = rustpy_ml::init_with_ml(&["numpy", "torch", "torchvision"]);
    });
    Lazy::force(&INIT);
    Ok(())
}

/// Define trainer class and helper functions in Python.
fn setup_trainer_class() -> rustpy_ml::Result<()> {
    py_exec!(
        r#"
import base64
import io
import numpy as np
import torch
import torch.nn as nn
import torch.optim as optim
from torch.utils.data import DataLoader, random_split
from torchvision import datasets, transforms

class MnistCnnTrainer:
    """CNN trainer for MNIST classification."""

    def __init__(self, conv1, conv2, kernel_size, dense_units, lr, batch_size, seed=42):
        import numpy as np
        import torch
        import torch.nn as nn
        import torch.optim as optim
        from torch.utils.data import DataLoader, random_split
        from torchvision import datasets, transforms

        self.conv1 = int(conv1)
        self.conv2 = int(conv2)
        self.kernel_size = int(kernel_size)
        self.dense_units = int(dense_units)
        self.lr = float(lr)
        self.batch_size = int(batch_size)
        self.seed = int(seed)

        torch.manual_seed(self.seed)
        np.random.seed(self.seed)

        transform = transforms.Compose([
            transforms.ToTensor(),
        ])

        full_train = datasets.MNIST(
            root="data",
            train=True,
            download=True,
            transform=transform,
        )
        test_set = datasets.MNIST(
            root="data",
            train=False,
            download=True,
            transform=transform,
        )

        train_size = 55000
        val_size = len(full_train) - train_size
        generator = torch.Generator().manual_seed(self.seed)
        train_set, val_set = random_split(full_train, [train_size, val_size], generator=generator)

        self.train_loader = DataLoader(train_set, batch_size=self.batch_size, shuffle=True)
        self.val_loader = DataLoader(val_set, batch_size=self.batch_size, shuffle=False)
        self.test_loader = DataLoader(test_set, batch_size=self.batch_size, shuffle=False)

        self.model = self._build_model()
        self.criterion = nn.CrossEntropyLoss()
        self.optimizer = optim.Adam(self.model.parameters(), lr=self.lr)

        self.train_losses = []
        self.val_accs = []
        self.best_loss = float("inf")
        self.best_acc = 0.0
        self.best_model_b64 = ""

    def _build_model(self):
        import torch.nn as nn

        padding = self.kernel_size // 2

        class CnnNet(nn.Module):
            def __init__(self, conv1, conv2, kernel_size, dense_units, padding):
                super().__init__()
                self.features = nn.Sequential(
                    nn.Conv2d(1, conv1, kernel_size=kernel_size, padding=padding),
                    nn.ReLU(),
                    nn.MaxPool2d(2),
                    nn.Conv2d(conv1, conv2, kernel_size=kernel_size, padding=padding),
                    nn.ReLU(),
                    nn.MaxPool2d(2),
                )
                self.classifier = nn.Sequential(
                    nn.Flatten(),
                    nn.Linear(conv2 * 7 * 7, dense_units),
                    nn.ReLU(),
                    nn.Dropout(0.25),
                    nn.Linear(dense_units, 10),
                )

            def forward(self, x):
                x = self.features(x)
                x = self.classifier(x)
                return x

        return CnnNet(self.conv1, self.conv2, self.kernel_size, self.dense_units, padding)

    def _checkpoint_dict(self):
        return {
            "model_state": self.model.state_dict(),
            "conv1": self.conv1,
            "conv2": self.conv2,
            "kernel_size": self.kernel_size,
            "dense_units": self.dense_units,
            "seed": self.seed,
            "best_loss": self.best_loss,
            "best_acc": self.best_acc,
        }

    def _encode_checkpoint(self):
        import base64
        import io
        import torch

        buffer = io.BytesIO()
        torch.save(self._checkpoint_dict(), buffer)
        return base64.b64encode(buffer.getvalue()).decode("utf-8")

    @classmethod
    def from_checkpoint_b64(cls, model_b64, lr=0.001, batch_size=64):
        import base64
        import io
        import torch

        raw = base64.b64decode(model_b64)
        checkpoint = torch.load(io.BytesIO(raw), map_location="cpu", weights_only=False)

        trainer = cls(
            int(checkpoint["conv1"]),
            int(checkpoint["conv2"]),
            int(checkpoint["kernel_size"]),
            int(checkpoint["dense_units"]),
            float(lr),
            int(batch_size),
            int(checkpoint.get("seed", 42)),
        )

        trainer.model.load_state_dict(checkpoint["model_state"])
        trainer.model.eval()
        trainer.best_loss = float(checkpoint.get("best_loss", trainer.best_loss))
        trainer.best_acc = float(checkpoint.get("best_acc", trainer.best_acc))
        trainer.best_model_b64 = model_b64
        return trainer

    def start_epoch(self):
        self.model.train()
        self._epoch_iter = iter(self.train_loader)
        self._epoch_running_loss = 0.0
        self._epoch_batches = 0

    def _finalize_epoch(self):
        import torch

        train_loss = self._epoch_running_loss / max(self._epoch_batches, 1)
        self.train_losses.append(train_loss)

        self.model.eval()
        correct = 0
        total = 0

        with torch.no_grad():
            for xb, yb in self.val_loader:
                logits = self.model(xb)
                pred = torch.argmax(logits, dim=1)
                correct += int((pred == yb).sum().item())
                total += int(yb.size(0))

        val_acc = float(correct / max(total, 1))
        self.val_accs.append(val_acc)

        if train_loss < self.best_loss:
            self.best_loss = train_loss

        if val_acc >= self.best_acc:
            self.best_acc = val_acc
            self.best_model_b64 = self._encode_checkpoint()

        return float(train_loss), float(val_acc)

    def train_next_batch(self):
        try:
            xb, yb = next(self._epoch_iter)
        except StopIteration:
            train_loss, val_acc = self._finalize_epoch()
            return True, train_loss, val_acc

        self.optimizer.zero_grad()
        logits = self.model(xb)
        loss = self.criterion(logits, yb)
        loss.backward()
        self.optimizer.step()

        self._epoch_running_loss += float(loss.item())
        self._epoch_batches += 1
        return False, 0.0, 0.0

    def infer_from_pixels(self, pixels):
        import numpy as np
        import torch

        arr = np.asarray(pixels, dtype=np.float32).reshape(28, 28)
        arr = np.clip(arr, 0.0, 1.0)

        x = torch.tensor(arr, dtype=torch.float32).unsqueeze(0).unsqueeze(0)

        self.model.eval()
        with torch.no_grad():
            logits = self.model(x)
            probs = torch.softmax(logits, dim=1).squeeze(0)

        pred = int(torch.argmax(probs).item())
        confidence = float(probs[pred].item())
        probabilities = [float(v) for v in probs.tolist()]

        return pred, confidence, probabilities

    def finalize(self):
        import torch

        self.model.eval()
        correct = 0
        total = 0

        with torch.no_grad():
            for xb, yb in self.test_loader:
                logits = self.model(xb)
                pred = torch.argmax(logits, dim=1)
                correct += int((pred == yb).sum().item())
                total += int(yb.size(0))

        test_acc = float(correct / max(total, 1))

        if not self.best_model_b64:
            self.best_model_b64 = self._encode_checkpoint()

        return (
            [float(v) for v in self.train_losses],
            [float(v) for v in self.val_accs],
            float(self.best_loss),
            float(self.best_acc),
            float(test_acc),
            self.best_model_b64,
        )

def infer_from_checkpoint(model_b64, pixels):
    trainer = MnistCnnTrainer.from_checkpoint_b64(model_b64)
    return trainer.infer_from_pixels(pixels)

def validate_checkpoint(model_b64):
    _ = MnistCnnTrainer.from_checkpoint_b64(model_b64)
    return True
"#
    )?;

    Ok(())
}

#[derive(Debug, Deserialize)]
pub struct StartTrainingRequest {
    pub conv_channels_1: usize,
    pub conv_channels_2: usize,
    pub kernel_size: usize,
    pub dense_units: usize,
    pub epochs: usize,
    pub learning_rate: f64,
    pub batch_size: usize,
}

#[derive(Debug, Deserialize)]
pub struct InferDrawingRequest {
    pub pixels: Vec<f64>,
}

#[derive(Debug, Deserialize)]
pub struct UploadModelRequest {
    pub data: String,
}

#[derive(Debug, Serialize)]
struct MessageResponse {
    message: String,
}

#[derive(Debug, Serialize)]
struct ErrorResponse {
    error: String,
}

#[derive(Debug, Serialize)]
struct ModelDownload {
    filename: String,
    data: String,
}

// ============================================================================
// State Management
// ============================================================================

static TRAINING_STATE: Lazy<Arc<Mutex<TrainingState>>> =
    Lazy::new(|| Arc::new(Mutex::new(TrainingState::default())));

static CONTROL_FLAGS: Lazy<Arc<TrainingControlFlags>> =
    Lazy::new(|| Arc::new(TrainingControlFlags::default()));

fn get_state() -> Arc<Mutex<TrainingState>> {
    TRAINING_STATE.clone()
}

fn get_flags() -> Arc<TrainingControlFlags> {
    CONTROL_FLAGS.clone()
}

fn lock_state<'a>(state: &'a Arc<Mutex<TrainingState>>) -> MutexGuard<'a, TrainingState> {
    match state.lock() {
        Ok(guard) => guard,
        Err(poisoned) => poisoned.into_inner(),
    }
}

// ============================================================================
// REST Endpoints
// ============================================================================

#[post("/start")]
pub async fn start_training(req: Request) -> Response {
    let body = match req.into_body().await {
        Ok(b) => match b.as_str() {
            Ok(s) => s.to_string(),
            Err(_) => return text("Invalid UTF-8 in request body".to_string()),
        },
        Err(e) => return text(format!("Failed to read body: {}", e)),
    };

    let config: StartTrainingRequest = match serde_json::from_str(&body) {
        Ok(c) => c,
        Err(e) => {
            return json(ErrorResponse {
                error: format!("Parse error: {}", e),
            });
        }
    };

    if config.conv_channels_1 == 0 || config.conv_channels_2 == 0 {
        return json(ErrorResponse {
            error: "conv channels must be > 0".to_string(),
        });
    }
    if config.kernel_size == 0 {
        return json(ErrorResponse {
            error: "kernel_size must be > 0".to_string(),
        });
    }
    if config.dense_units == 0 {
        return json(ErrorResponse {
            error: "dense_units must be > 0".to_string(),
        });
    }
    if config.epochs == 0 {
        return json(ErrorResponse {
            error: "epochs must be > 0".to_string(),
        });
    }
    if config.learning_rate <= 0.0 {
        return json(ErrorResponse {
            error: "learning_rate must be > 0".to_string(),
        });
    }

    let state = get_state();
    let mut st = lock_state(&state);

    match st.status {
        TrainingStatus::Running | TrainingStatus::Paused => {
            return json(ErrorResponse {
                error: "Training already in progress".to_string(),
            });
        }
        _ => {}
    }

    st.status = TrainingStatus::Running;
    st.config = Some(TrainingConfig {
        conv_channels_1: config.conv_channels_1,
        conv_channels_2: config.conv_channels_2,
        kernel_size: config.kernel_size,
        dense_units: config.dense_units,
        epochs: config.epochs,
        learning_rate: config.learning_rate,
        batch_size: config.batch_size,
    });
    st.metrics = TrainingMetrics::default();
    st.error_message = None;
    st.model_path = None;
    st.model_loaded = false;
    st.model_source = None;
    st.last_inference = None;

    drop(st);

    let flags = get_flags();
    flags.reset();

    let state_clone = state.clone();
    let flags_clone = flags.clone();

    tokio::spawn(async move {
        run_training_task(state_clone, config, flags_clone).await;
    });

    json(MessageResponse {
        message: "Training started".to_string(),
    })
}

#[post("/pause")]
pub async fn pause_training(_req: Request) -> Response {
    let state = get_state();
    let mut st = lock_state(&state);

    match st.status {
        TrainingStatus::Running => {
            st.status = TrainingStatus::Paused;
            get_flags().set_pause(true);
            json(MessageResponse {
                message: "Training paused".to_string(),
            })
        }
        _ => json(ErrorResponse {
            error: "Training not running".to_string(),
        }),
    }
}

#[post("/resume")]
pub async fn resume_training(_req: Request) -> Response {
    let state = get_state();
    let mut st = lock_state(&state);

    match st.status {
        TrainingStatus::Paused => {
            st.status = TrainingStatus::Running;
            get_flags().set_pause(false);
            json(MessageResponse {
                message: "Training resumed".to_string(),
            })
        }
        _ => json(ErrorResponse {
            error: "Training not paused".to_string(),
        }),
    }
}

#[post("/stop")]
pub async fn stop_training(_req: Request) -> Response {
    let state = get_state();
    let mut st = lock_state(&state);

    match st.status {
        TrainingStatus::Running | TrainingStatus::Paused => {
            st.status = TrainingStatus::Stopped;
            get_flags().set_stop(true);
            json(MessageResponse {
                message: "Training stopped".to_string(),
            })
        }
        _ => json(ErrorResponse {
            error: "Training not active".to_string(),
        }),
    }
}

#[get("/status")]
pub async fn get_status(_req: Request) -> Response {
    let state = get_state();
    let st = lock_state(&state);
    json(st.clone())
}

#[post("/reset")]
pub async fn reset_training(_req: Request) -> Response {
    let state = get_state();
    let mut st = lock_state(&state);

    match st.status {
        TrainingStatus::Running | TrainingStatus::Paused => {
            return json(ErrorResponse {
                error: "Cannot reset while training is active. Stop first.".to_string(),
            });
        }
        _ => {}
    }

    *st = TrainingState::default();
    get_flags().reset();

    json(MessageResponse {
        message: "Training state reset".to_string(),
    })
}

#[get("/model")]
pub async fn download_model(_req: Request) -> Response {
    let state = get_state();
    let st = lock_state(&state);

    if let Some(model_data) = &st.model_data {
        let encoded = base64::engine::general_purpose::STANDARD.encode(model_data);
        json(ModelDownload {
            filename: "mnist_cnn_model.pt".to_string(),
            data: encoded,
        })
    } else {
        json(ErrorResponse {
            error: "No model available for download".to_string(),
        })
    }
}

#[post("/model/upload")]
pub async fn upload_model(req: Request) -> Response {
    let body = match req.into_body().await {
        Ok(b) => match b.as_str() {
            Ok(s) => s.to_string(),
            Err(_) => return text("Invalid UTF-8 in request body".to_string()),
        },
        Err(e) => return text(format!("Failed to read body: {}", e)),
    };

    let payload: UploadModelRequest = match serde_json::from_str(&body) {
        Ok(c) => c,
        Err(e) => {
            return json(ErrorResponse {
                error: format!("Parse error: {}", e),
            });
        }
    };

    let model_bytes = match base64::engine::general_purpose::STANDARD.decode(payload.data) {
        Ok(b) => b,
        Err(_) => {
            return json(ErrorResponse {
                error: "Model data is not valid base64".to_string(),
            });
        }
    };

    let state = get_state();
    let mut st = lock_state(&state);

    if matches!(st.status, TrainingStatus::Running | TrainingStatus::Paused) {
        return json(ErrorResponse {
            error: "Cannot upload while training is active".to_string(),
        });
    }

    st.model_data = Some(model_bytes);
    st.model_path = Some("mnist_cnn_model.pt".to_string());
    st.model_loaded = false;
    st.model_source = Some("uploaded".to_string());

    json(MessageResponse {
        message: "Model uploaded. Call /train/model/load to validate and activate it.".to_string(),
    })
}

#[post("/model/load")]
pub async fn load_model(_req: Request) -> Response {
    if let Err(e) = init_python_ml() {
        return json(ErrorResponse {
            error: format!("Failed to init Python: {}", e),
        });
    }
    if let Err(e) = setup_trainer_class() {
        return json(ErrorResponse {
            error: format!("Failed to setup trainer class: {}", e),
        });
    }

    let state = get_state();
    
    let model_data = {
        let st = lock_state(&state);

        if matches!(st.status, TrainingStatus::Running | TrainingStatus::Paused) {
            return json(ErrorResponse {
                error: "Cannot load model while training is active".to_string(),
            });
        }

        if st.model_data.is_none() {
            return json(ErrorResponse {
                error: "No uploaded/trained model available".to_string(),
            });
        }

        st.model_data.clone().unwrap()
    };

    let model_b64 = base64::engine::general_purpose::STANDARD.encode(&model_data);
    let load_cmd = format!("trainer = MnistCnnTrainer.from_checkpoint_b64('{}')", model_b64);

    let load_result = tokio::task::spawn_blocking(move || {
        py_exec!(load_cmd.as_str())
    }).await;

    let load_outcome: std::result::Result<(), String> = match load_result {
        Ok(inner) => inner.map_err(|e| e.to_string()),
        Err(e) => Err(format!("Join error while loading checkpoint: {}", e)),
    };

    if let Err(e) = load_outcome {
        return json(ErrorResponse {
            error: format!("Failed to load checkpoint: {}", e),
        });
    }

    let load_cmd_pyo3 = format!(
        "import __main__; __main__.trainer_pyo3 = MnistCnnTrainer.from_checkpoint_b64('{}')",
        model_b64
    );
    let load_pyo3_result = tokio::task::spawn_blocking(move || {
        py_exec!(load_cmd_pyo3.as_str())
    }).await;

    let load_pyo3_outcome: std::result::Result<(), String> = match load_pyo3_result {
        Ok(inner) => inner.map_err(|e| e.to_string()),
        Err(e) => Err(format!("Join error while loading PyO3 trainer: {}", e)),
    };

    if let Err(e) = load_pyo3_outcome {
        return json(ErrorResponse {
            error: format!("Failed to load PyO3 trainer: {}", e),
        });
    }

    let valid_result = tokio::task::spawn_blocking(|| {
        python!(-> bool, "trainer is not None")
    }).await;

    let valid = match valid_result {
        Ok(Ok(ok)) => ok,
        _ => false,
    };

    if !valid {
        return json(ErrorResponse {
            error: "Checkpoint validation failed".to_string(),
        });
    }

    let mut st = lock_state(&state);
    st.model_loaded = true;
    if st.model_source.is_none() {
        st.model_source = Some("uploaded".to_string());
    }

    json(MessageResponse {
        message: "Model loaded for inference".to_string(),
    })
}

#[post("/infer-drawing")]
pub async fn infer_drawing(req: Request) -> Response {
    start_request();

    let body = match req.into_body().await {
        Ok(b) => match b.as_str() {
            Ok(s) => {
                record_body_read_done();
                s.to_string()
            },
            Err(_) => return text("Invalid UTF-8 in request body".to_string()),
        },
        Err(e) => return text(format!("Failed to read body: {}", e)),
    };

    let payload: InferDrawingRequest = match serde_json::from_str(&body) {
        Ok(c) => {
            record_json_parse_done();
            c
        },
        Err(e) => {
            return json(ErrorResponse {
                error: format!("Parse error: {}", e),
            });
        }
    };

    if payload.pixels.len() != 28 * 28 {
        return json(ErrorResponse {
            error: "pixels must contain exactly 784 normalized values".to_string(),
        });
    }

    if let Err(e) = init_python_ml() {
        return json(ErrorResponse {
            error: format!("Failed to init Python: {}", e),
        });
    }
    if let Err(e) = setup_trainer_class() {
        return json(ErrorResponse {
            error: format!("Failed to setup trainer class: {}", e),
        });
    }

    let state = get_state();
    
    let model_source = {
        let st = lock_state(&state);

        if !st.model_loaded {
            return json(ErrorResponse {
                error: "No active model loaded. Train or load a model first.".to_string(),
            });
        }

        if st.model_data.is_none() {
            return json(ErrorResponse {
                error: "No model bytes available".to_string(),
            });
        }

        st.model_source.clone()
    };

    let pixels_json = match serde_json::to_string(&payload.pixels) {
        Ok(v) => v,
        Err(e) => {
            return json(ErrorResponse {
                error: format!("Failed to serialize drawing pixels: {}", e),
            });
        }
    };

    if model_source.as_deref() == Some("trained") {
        let has_trainer_result = tokio::task::spawn_blocking(|| {
            python!(-> bool, "'trainer' in globals() and trainer is not None")
        }).await;

        let has_trainer = match has_trainer_result {
            Ok(Ok(ok)) => ok,
            _ => false,
        };

        if !has_trainer {
            let model_data = {
                let st = lock_state(&state);
                match &st.model_data {
                    Some(bytes) => bytes.clone(),
                    None => {
                        return json(ErrorResponse {
                            error: "No model bytes available".to_string(),
                        });
                    }
                }
            };

            let model_b64 = base64::engine::general_purpose::STANDARD.encode(&model_data);
            let load_cmd = format!("trainer = MnistCnnTrainer.from_checkpoint_b64('{}')", model_b64);

            let load_result = tokio::task::spawn_blocking(move || {
                py_exec!(load_cmd.as_str())
            }).await;

            let load_outcome: std::result::Result<(), String> = match load_result {
                Ok(inner) => inner.map_err(|e| e.to_string()),
                Err(e) => Err(format!("Join error while restoring trainer: {}", e)),
            };

            if let Err(e) = load_outcome {
                return json(ErrorResponse {
                    error: format!("Failed to restore trainer: {}", e),
                });
            }
        }
    }

    let infer_cmd = format!("trainer.infer_from_pixels({})", pixels_json);
    record_spawn_block_enter();
    let infer_result = tokio::task::spawn_blocking(move || {
        record_spawn_block_acquired();
        let result = python!(-> (i64, f64, Vec<f64>), infer_cmd.as_str());
        record_inference_done();
        result
    }).await;

    let prediction = match infer_result {
        Ok(Ok((pred, conf, probs))) => InferenceResult {
            predicted_digit: pred.max(0) as usize,
            confidence: conf,
            probabilities: probs,
        },
        _ => {
            return json(ErrorResponse {
                error: "Inference failed".to_string(),
            });
        }
    };

    let mut st = lock_state(&state);
    st.last_inference = Some(prediction.clone());
    record_response_serialize_done();
    let response = json(prediction);
    record_response_send();
    response
}

#[post("/infer-drawing-pyo3")]
pub async fn infer_drawing_pyo3(req: Request) -> Response {
    start_request();

    let body = match req.into_body().await {
        Ok(b) => match b.as_str() {
            Ok(s) => {
                record_body_read_done();
                s.to_string()
            },
            Err(_) => return text("Invalid UTF-8 in request body".to_string()),
        },
        Err(e) => return text(format!("Failed to read body: {}", e)),
    };

    let payload: InferDrawingRequest = match serde_json::from_str(&body) {
        Ok(c) => {
            record_json_parse_done();
            c
        },
        Err(e) => {
            return json(ErrorResponse {
                error: format!("Parse error: {}", e),
            });
        }
    };

    if payload.pixels.len() != 28 * 28 {
        return json(ErrorResponse {
            error: "pixels must contain exactly 784 normalized values".to_string(),
        });
    }

    if let Err(e) = init_python_ml() {
        return json(ErrorResponse {
            error: format!("Failed to init Python: {}", e),
        });
    }
    if let Err(e) = setup_trainer_class() {
        return json(ErrorResponse {
            error: format!("Failed to setup trainer class: {}", e),
        });
    }

    let state = get_state();

    {
        let st = lock_state(&state);

        if !st.model_loaded {
            return json(ErrorResponse {
                error: "No active model loaded. Train or load a model first.".to_string(),
            });
        }

        if st.model_data.is_none() {
            return json(ErrorResponse {
                error: "No model bytes available".to_string(),
            });
        }
    };

    let has_pyo3_trainer_result = tokio::task::spawn_blocking(|| {
        Python::with_gil(|py| {
            match py.import("__main__") {
                Ok(main_mod) => main_mod.getattr("trainer_pyo3").is_ok(),
                Err(_) => false,
            }
        })
    }).await;

    let has_pyo3_trainer = match has_pyo3_trainer_result {
        Ok(ok) => ok,
        Err(_) => false,
    };

    if !has_pyo3_trainer {
        let model_data = {
            let st = lock_state(&state);
            match &st.model_data {
                Some(bytes) => bytes.clone(),
                None => {
                    return json(ErrorResponse {
                        error: "No model bytes available".to_string(),
                    });
                }
            }
        };

        let model_b64 = base64::engine::general_purpose::STANDARD.encode(&model_data);
        let load_cmd = format!(
            "import __main__; __main__.trainer_pyo3 = MnistCnnTrainer.from_checkpoint_b64('{}')",
            model_b64
        );

        let load_result = tokio::task::spawn_blocking(move || {
            py_exec!(load_cmd.as_str())
        }).await;

        let load_outcome: std::result::Result<(), String> = match load_result {
            Ok(inner) => inner.map_err(|e| e.to_string()),
            Err(e) => Err(format!("Join error while restoring PyO3 trainer: {}", e)),
        };

        if let Err(e) = load_outcome {
            return json(ErrorResponse {
                error: format!("Failed to restore PyO3 trainer: {}", e),
            });
        }
    }

    let pixels = payload.pixels.clone();
    record_spawn_block_enter();
    let infer_result = tokio::task::spawn_blocking(move || {
        record_spawn_block_acquired();
        let result = Python::with_gil(|py| -> std::result::Result<InferenceResult, String> {
            let globals = py
                .import("__main__")
                .map_err(|e| format!("Failed to import __main__: {}", e))?;
            let trainer = globals
                .getattr("trainer_pyo3")
                .map_err(|e| format!("Trainer not initialized: {}", e))?;

            let py_pixels = PyList::new(py, &pixels)
                .map_err(|e| format!("Failed to build Python list: {}", e))?;
            let output = trainer
                .call_method1("infer_from_pixels", (py_pixels,))
                .map_err(|e| format!("Inference call failed: {}", e))?;

            let (pred, conf, probs): (i64, f64, Vec<f64>) = output
                .extract()
                .map_err(|e| format!("Failed to extract inference result: {}", e))?;

            Ok(InferenceResult {
                predicted_digit: pred.max(0) as usize,
                confidence: conf,
                probabilities: probs,
            })
        });
        record_inference_done();
        result
    }).await;

    let prediction = match infer_result {
        Ok(Ok(value)) => value,
        Ok(Err(e)) => {
            return json(ErrorResponse {
                error: format!("Inference failed: {}", e),
            });
        }
        Err(_) => {
            return json(ErrorResponse {
                error: "Inference failed".to_string(),
            });
        }
    };

    let mut st = lock_state(&state);
    st.last_inference = Some(prediction.clone());
    record_response_serialize_done();
    let response = json(prediction);
    record_response_send();
    response
}

// ============================================================================
// Core Training Logic
// ============================================================================

async fn run_training_task(
    state: Arc<Mutex<TrainingState>>,
    config: StartTrainingRequest,
    flags: Arc<TrainingControlFlags>,
) {
    let start_time = Instant::now();

    // Run blockingPython initialization in blocking thread pool
    let init_result = tokio::task::spawn_blocking(|| {
        if let Err(e) = init_python_ml() {
            return Err(format!(
                "Failed to init Python. Ensure numpy/torch/torchvision are installed: {}",
                e
            ));
        }

        if let Err(e) = setup_trainer_class() {
            return Err(format!("Failed to setup trainer class: {}", e));
        }

        Ok(())
    }).await;

    if let Err(e) = init_result {
        update_state_error(&state, format!("Task spawn failed: {}", e));
        return;
    }

    if let Err(e) = init_result.unwrap() {
        update_state_error(&state, e);
        return;
    }

    let init_trainer = format!(
        "trainer = MnistCnnTrainer({}, {}, {}, {}, {}, {})",
        config.conv_channels_1,
        config.conv_channels_2,
        config.kernel_size,
        config.dense_units,
        config.learning_rate,
        config.batch_size
    );

    let init_trainer_result = tokio::task::spawn_blocking(move || {
        py_exec!(init_trainer.as_str())
    }).await;

    if let Err(e) = init_trainer_result {
        update_state_error(&state, format!("Task spawn failed: {}", e));
        return;
    }

    if let Err(e) = init_trainer_result.unwrap() {
        update_state_error(&state, format!("Trainer init failed: {}", e));
        return;
    }

    for epoch in 0..config.epochs {
        // Check for graceful shutdown
        if flags.should_stop() || crate::training_state::should_shutdown() {
            let mut st = lock_state(&state);
            st.status = TrainingStatus::Stopped;
            st.metrics.elapsed_seconds = start_time.elapsed().as_secs_f64();
            return;
        }

        // Start epoch in blocking thread pool
        let start_epoch_result = tokio::task::spawn_blocking(|| {
            py_exec!("trainer.start_epoch()")
        }).await;

        if let Err(e) = start_epoch_result {
            update_state_error(&state, format!("Task spawn failed: {}", e));
            return;
        }

        if let Err(e) = start_epoch_result.unwrap() {
            update_state_error(&state, format!("Failed to start epoch {}: {}", epoch + 1, e));
            return;
        }

        loop {
            if flags.should_stop() || crate::training_state::should_shutdown() {
                let mut st = lock_state(&state);
                st.status = TrainingStatus::Stopped;
                st.metrics.elapsed_seconds = start_time.elapsed().as_secs_f64();
                st.metrics.training_logs.push(format!(
                    "Training stopped at epoch {}/{}",
                    epoch + 1,
                    config.epochs
                ));
                return;
            }

            // Check pause flag and wait if paused (non-blocking)
            while flags.is_paused() && !flags.should_stop() && !crate::training_state::should_shutdown() {
                tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
            }

            if flags.should_stop() || crate::training_state::should_shutdown() {
                let mut st = lock_state(&state);
                st.status = TrainingStatus::Stopped;
                st.metrics.elapsed_seconds = start_time.elapsed().as_secs_f64();
                st.metrics.training_logs.push(format!(
                    "Training stopped at epoch {}/{}",
                    epoch + 1,
                    config.epochs
                ));
                return;
            }

            // Train one batch in blocking thread pool (allows pause/stop checks between batches)
            let batch_result = tokio::task::spawn_blocking(|| {
                python!(-> (bool, f64, f64), "trainer.train_next_batch()")
            }).await;

            if let Err(e) = batch_result {
                update_state_error(&state, format!("Task spawn failed: {}", e));
                return;
            }

            match batch_result.unwrap() {
                Ok((done, train_loss, val_acc)) => {
                    if done {
                        let mut st = lock_state(&state);
                        st.metrics.current_epoch = epoch + 1;
                        st.metrics.train_loss_history.push(train_loss);
                        st.metrics.val_accuracy_history.push(val_acc);
                        st.metrics.elapsed_seconds = start_time.elapsed().as_secs_f64();
                        st.metrics.training_logs.push(format!(
                            "[Epoch {}/{}] Loss: {:.4} | Val Acc: {:.2}%",
                            epoch + 1,
                            config.epochs,
                            train_loss,
                            val_acc * 100.0
                        ));
                        break;
                    }
                }
                Err(e) => {
                    update_state_error(&state, format!("Epoch {} failed: {}", epoch + 1, e));
                    return;
                }
            }
        }
    }

    // Finalize in blocking thread pool
    let finalize_result = tokio::task::spawn_blocking(|| {
        python!(-> (Vec<f64>, Vec<f64>, f64, f64, f64, String), "trainer.finalize()")
    }).await;

    if let Err(e) = finalize_result {
        update_state_error(&state, format!("Task spawn failed: {}", e));
        return;
    }

    match finalize_result.unwrap() {
        Ok((train_losses, val_accs, best_loss, best_acc, test_acc, model_b64)) => {
            let mut st = lock_state(&state);
            st.status = TrainingStatus::Completed;
            st.metrics.elapsed_seconds = start_time.elapsed().as_secs_f64();
            st.metrics.train_loss_history = train_losses;
            st.metrics.val_accuracy_history = val_accs;
            st.metrics.best_loss = best_loss;
            st.metrics.best_accuracy = best_acc;

            if !model_b64.is_empty() {
                if let Ok(model_bytes) = base64::engine::general_purpose::STANDARD.decode(&model_b64) {
                    st.model_data = Some(model_bytes);
                    st.model_path = Some("mnist_cnn_model.pt".to_string());
                    st.model_loaded = true;
                    st.model_source = Some("trained".to_string());
                }
            }

            st.metrics.training_logs.push(format!(
                "✓ Training complete! Best Loss: {:.4}, Best Val Acc: {:.2}%, Test Acc: {:.2}%",
                best_loss,
                best_acc * 100.0,
                test_acc * 100.0
            ));
        }
        Err(e) => {
            update_state_error(&state, format!("Finalization failed: {}", e));
        }
    }
}

fn update_state_error(state: &Arc<Mutex<TrainingState>>, error: String) {
    let mut st = lock_state(state);
    st.status = TrainingStatus::Completed;
    st.error_message = Some(error);
    st.model_loaded = false;
}
