//! Neural network training handler with pause/resume/stop control.
//!
//! Manages training lifecycle through REST endpoints, with async training tasks
//! that respect pause and stop signals.

use rustpy_ml::prelude::*;
use unipotato::{Request, Response, handler::{json, text}, post, get};
use serde::{Deserialize, Serialize};
use std::sync::{Arc, Mutex, MutexGuard};
use std::time::Instant;
use once_cell::sync::Lazy;
use base64::Engine as _;

use crate::training_state::{TrainingState, TrainingStatus, TrainingConfig, TrainingMetrics, TrainingControlFlags};

/// Request body for starting training
#[derive(Debug, Deserialize)]
pub struct StartTrainingRequest {
    pub hidden_layers: usize,
    pub nodes_per_layer: Vec<usize>,
    pub epochs: usize,
    pub learning_rate: f64,
    pub batch_size: usize,
}

/// Response wrapper
#[derive(Debug, Serialize)]
struct MessageResponse {
    message: String,
}

#[derive(Debug, Serialize)]
struct ErrorResponse {
    error: String,
}

/// Global training state - lazy initialization
static TRAINING_STATE: Lazy<Arc<Mutex<TrainingState>>> = Lazy::new(|| {
    Arc::new(Mutex::new(TrainingState::default()))
});

/// Global control flags - lazy initialization
static CONTROL_FLAGS: Lazy<Arc<TrainingControlFlags>> = Lazy::new(|| {
    Arc::new(TrainingControlFlags::default())
});

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

/// POST /train/start
/// Start training with given hyperparameters
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
        Err(e) => return json(ErrorResponse { error: format!("Parse error: {}", e) }),
    };

    // Validation
    if config.hidden_layers == 0 {
        return json(ErrorResponse { error: "hidden_layers must be >= 1".to_string() });
    }
    if config.nodes_per_layer.len() != config.hidden_layers {
        return json(ErrorResponse { 
            error: format!(
                "nodes_per_layer length ({}) must match hidden_layers ({})",
                config.nodes_per_layer.len(),
                config.hidden_layers
            )
        });
    }
    if config.epochs == 0 {
        return json(ErrorResponse { error: "epochs must be > 0".to_string() });
    }
    if config.learning_rate <= 0.0 {
        return json(ErrorResponse { error: "learning_rate must be > 0".to_string() });
    }

    let state = get_state();
    let mut st = lock_state(&state);

    // Check if already training
    match st.status {
        TrainingStatus::Running | TrainingStatus::Paused => {
            return json(ErrorResponse { error: "Training already in progress".to_string() });
        }
        _ => {}
    }

    // Update state
    st.status = TrainingStatus::Running;
    st.config = Some(TrainingConfig {
        hidden_layers: config.hidden_layers,
        nodes_per_layer: config.nodes_per_layer.clone(),
        epochs: config.epochs,
        learning_rate: config.learning_rate,
        batch_size: config.batch_size,
    });
    st.metrics = TrainingMetrics::default();
    st.error_message = None;
    st.model_path = None;

    drop(st); // Release lock before spawning task

    // Get flags for this task
    let flags = get_flags();
    flags.reset();

    // Spawn async training task
    let state_clone = state.clone();
    let flags_clone = flags.clone();
    
    tokio::spawn(async move {
        run_training(
            state_clone,
            config,
            flags_clone,
        )
        .await;
    });

    json(MessageResponse {
        message: "Training started".to_string(),
    })
}

/// POST /train/pause
/// Pause current training
#[post("/pause")]
pub async fn pause_training(_req: Request) -> Response {
    let state = get_state();
    let mut st = lock_state(&state);

    match st.status {
        TrainingStatus::Running => {
            st.status = TrainingStatus::Paused;
            let flags = get_flags();
            flags.set_pause(true);
            json(MessageResponse {
                message: "Training paused".to_string(),
            })
        }
        _ => {
            json(ErrorResponse {
                error: "Training not running".to_string(),
            })
        }
    }
}

/// POST /train/resume
/// Resume paused training
#[post("/resume")]
pub async fn resume_training(_req: Request) -> Response {
    let state = get_state();
    let mut st = lock_state(&state);

    match st.status {
        TrainingStatus::Paused => {
            st.status = TrainingStatus::Running;
            let flags = get_flags();
            flags.set_pause(false);
            json(MessageResponse {
                message: "Training resumed".to_string(),
            })
        }
        _ => {
            json(ErrorResponse {
                error: "Training not paused".to_string(),
            })
        }
    }
}

/// POST /train/stop
/// Stop current training
#[post("/stop")]
pub async fn stop_training(_req: Request) -> Response {
    let state = get_state();
    let mut st = lock_state(&state);

    match st.status {
        TrainingStatus::Running | TrainingStatus::Paused => {
            st.status = TrainingStatus::Stopped;
            let flags = get_flags();
            flags.set_stop(true);
            json(MessageResponse {
                message: "Training stopped".to_string(),
            })
        }
        _ => {
            json(ErrorResponse {
                error: "Training not active".to_string(),
            })
        }
    }
}

/// GET /train/status
/// Get current training status and metrics
#[get("/status")]
pub async fn get_status(_req: Request) -> Response {
    let state = get_state();
    let st = lock_state(&state);
    json(st.clone())
}

/// POST /train/reset
/// Reset training state
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
    let flags = get_flags();
    flags.reset();

    json(MessageResponse {
        message: "Training state reset".to_string(),
    })
}

/// GET /train/model
/// Download trained model file (returns base64 encoded)
#[get("/model")]
pub async fn download_model(_req: Request) -> Response {
    let state = get_state();
    let st = lock_state(&state);

    if let Some(model_data) = &st.model_data {
        let encoded = base64::engine::general_purpose::STANDARD.encode(model_data);
        #[derive(Serialize)]
        struct ModelDownload {
            filename: String,
            data: String,
        }
        json(ModelDownload {
            filename: "iris_model.pt".to_string(),
            data: encoded,
        })
    } else {
        json(ErrorResponse {
            error: "No model available for download".to_string(),
        })
    }
}

/// Internal function to run training in background
async fn run_training(
    state: Arc<Mutex<TrainingState>>,
    config: StartTrainingRequest,
    flags: Arc<TrainingControlFlags>,
) {
    let start_time = Instant::now();
    
    // Initialize Python runtime and preload ML dependencies
    if let Err(e) = rustpy_ml::init_with_ml(&["numpy", "torch", "sklearn"]) {
        let mut st = lock_state(&state);
        st.status = TrainingStatus::Completed;
        st.error_message = Some(format!("Failed to initialize Python runtime: {}", e));
        return;
    }

    let nodes_literal = format!(
        "[{}]",
        config
            .nodes_per_layer
            .iter()
            .map(|n| n.to_string())
            .collect::<Vec<_>>()
            .join(", ")
    );

    // Define training class with per-epoch methods
    let define_trainer = py_exec!(r#"
class IrisTrainer:
    def __init__(self, nodes, lr, batch_size, seed=42):
        import numpy as np
        import torch
        import torch.nn as nn
        import torch.optim as optim
        from sklearn.datasets import load_iris
        from sklearn.model_selection import train_test_split
        from sklearn.preprocessing import StandardScaler
        
        np.random.seed(seed)
        torch.manual_seed(seed)
        
        iris = load_iris()
        X = iris.data.astype(np.float32)
        y = iris.target.astype(np.int64)
        
        X_train_val, self.X_test, y_train_val, self.y_test = train_test_split(
            X, y, test_size=0.2, random_state=seed, stratify=y
        )
        self.X_train, self.X_val, self.y_train, self.y_val = train_test_split(
            X_train_val, y_train_val, test_size=0.25, random_state=seed, stratify=y_train_val
        )
        
        scaler = StandardScaler()
        self.X_train = scaler.fit_transform(self.X_train).astype(np.float32)
        self.X_val = scaler.transform(self.X_val).astype(np.float32)
        self.X_test = scaler.transform(self.X_test).astype(np.float32)
        
        self.X_train_t = torch.tensor(self.X_train, dtype=torch.float32)
        self.y_train_t = torch.tensor(self.y_train, dtype=torch.long)
        self.X_val_t = torch.tensor(self.X_val, dtype=torch.float32)
        self.y_val_t = torch.tensor(self.y_val, dtype=torch.long)
        self.X_test_t = torch.tensor(self.X_test, dtype=torch.float32)
        self.y_test_t = torch.tensor(self.y_test, dtype=torch.long)
        
        # Build network
        layers = []
        prev_size = 4
        for hidden_size in nodes:
            layers.append(nn.Linear(prev_size, int(hidden_size)))
            layers.append(nn.ReLU())
            layers.append(nn.Dropout(0.2))
            prev_size = int(hidden_size)
        layers.append(nn.Linear(prev_size, 3))
        
        class DynamicNet(nn.Module):
            def __init__(self, seq_layers):
                super().__init__()
                self.net = nn.Sequential(*seq_layers)
            def forward(self, x):
                return self.net(x)
        
        self.model = DynamicNet(layers)
        self.criterion = nn.CrossEntropyLoss()
        self.optimizer = optim.Adam(self.model.parameters(), lr=lr)
        
        self.train_losses = []
        self.val_accs = []
        self.best_loss = float("inf")
        self.best_acc = 0.0
        self.model_state = None
        self.batch_size = int(batch_size)
    
    def train_epoch(self, epoch):
        import torch
        import base64
        import io
        
        self.model.train()
        idx = torch.randperm(self.X_train_t.size(0))
        x_epoch = self.X_train_t[idx]
        y_epoch = self.y_train_t[idx]
        
        epoch_loss = 0.0
        steps = 0
        for i in range(0, x_epoch.size(0), self.batch_size):
            xb = x_epoch[i:i + self.batch_size]
            yb = y_epoch[i:i + self.batch_size]
            
            self.optimizer.zero_grad()
            logits = self.model(xb)
            loss = self.criterion(logits, yb)
            loss.backward()
            self.optimizer.step()
            
            epoch_loss += float(loss.item())
            steps += 1
        
        train_loss = epoch_loss / max(steps, 1)
        self.train_losses.append(float(train_loss))
        
        self.model.eval()
        with torch.no_grad():
            val_logits = self.model(self.X_val_t)
            val_pred = torch.argmax(val_logits, dim=1)
            val_acc = float((val_pred == self.y_val_t).float().mean().item())
            self.val_accs.append(val_acc)
        
        if train_loss < self.best_loss:
            self.best_loss = float(train_loss)
            
        if val_acc > self.best_acc:
            self.best_acc = val_acc
            # Save model state as base64
            buffer = io.BytesIO()
            torch.save(self.model.state_dict(), buffer)
            self.model_state = base64.b64encode(buffer.getvalue()).decode('utf-8')
        
        return (float(train_loss), float(val_acc))
    
    def get_final_results(self):
        import torch
        
        with torch.no_grad():
            test_logits = self.model(self.X_test_t)
            test_pred = torch.argmax(test_logits, dim=1)
            test_acc = float((test_pred == self.y_test_t).float().mean().item())
        
        return (
            [float(v) for v in self.train_losses],
            [float(v) for v in self.val_accs],
            float(self.best_loss),
            float(self.best_acc),
            float(test_acc),
            self.model_state  # base64 encoded model
        )
"#);

    if let Err(e) = define_trainer {
        let mut st = lock_state(&state);
        st.status = TrainingStatus::Completed;
        st.error_message = Some(format!("Training setup error: {}", e));
        return;
    }

    // Create trainer instance
    let init_code = format!(
        "trainer = IrisTrainer({}, {}, {})",
        nodes_literal,
        config.learning_rate,
        config.batch_size,
    );

    if let Err(e) = py_exec!(init_code.as_str()) {
        let mut st = lock_state(&state);
        st.status = TrainingStatus::Completed;
        st.error_message = Some(format!("Trainer init error: {}", e));
        return;
    }

    // Training loop - epoch by epoch
    for epoch in 0..config.epochs {
        // Check stop flag
        if flags.should_stop() {
            let mut st = lock_state(&state);
            st.status = TrainingStatus::Stopped;
            st.metrics.elapsed_seconds = start_time.elapsed().as_secs_f64();
            break;
        }

        // Check pause flag - busy wait
        while flags.is_paused() && !flags.should_stop() {
            tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
        }

        // Train one epoch
        let epoch_code = format!("trainer.train_epoch({})", epoch);
        match python!(-> (f64, f64), epoch_code.as_str()) {
            Ok((train_loss, val_acc)) => {
                let mut st = lock_state(&state);
                st.metrics.current_epoch = epoch + 1;
                st.metrics.train_loss_history.push(train_loss);
                st.metrics.val_accuracy_history.push(val_acc);
                st.metrics.elapsed_seconds = start_time.elapsed().as_secs_f64();
                
                let log_msg = format!("[Epoch {}/{}] Loss: {:.4} | Val Acc: {:.2}%", 
                    epoch + 1, config.epochs, train_loss, val_acc * 100.0);
                st.metrics.training_logs.push(log_msg);
            }
            Err(e) => {
                let mut st = lock_state(&state);
                st.status = TrainingStatus::Completed;
                st.error_message = Some(format!("Epoch {} error: {}", epoch, e));
                return;
            }
        }
    }

    // Get final results
    match python!(-> (Vec<f64>, Vec<f64>, f64, f64, f64, String), "trainer.get_final_results()") {
        Ok((train_losses, val_accs, best_loss, best_acc, _test_acc, model_b64)) => {
            let mut st = lock_state(&state);
            st.status = TrainingStatus::Completed;
            st.metrics.current_epoch = train_losses.len();
            st.metrics.best_loss = best_loss;
            st.metrics.best_accuracy = best_acc;
            st.metrics.train_loss_history = train_losses;
            st.metrics.val_accuracy_history = val_accs;
            st.metrics.elapsed_seconds = start_time.elapsed().as_secs_f64();
            
            // Decode model
            if let Ok(model_bytes) = base64::engine::general_purpose::STANDARD.decode(&model_b64) {
                st.model_data = Some(model_bytes);
                st.model_path = Some("model.pt".to_string());
            }
            
            st.metrics.training_logs.push(format!("Training complete! Best Loss: {:.4}, Best Accuracy: {:.2}%", 
                best_loss, best_acc * 100.0));
        }
        Err(e) => {
            let mut st = lock_state(&state);
            st.status = TrainingStatus::Completed;
            st.error_message = Some(format!("Final results error: {}", e));
        }
    }
}
