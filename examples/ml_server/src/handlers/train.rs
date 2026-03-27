//! Neural network training handler with pause/resume/stop control.
//!
//! Manages training lifecycle through REST endpoints, with async training tasks
//! that respect pause and stop signals. Uses rustpy-ml for Python ML integration.

use rustpy_ml::prelude::*;
use unipotato::{Request, Response, handler::{json, text}, post, get};
use serde::{Deserialize, Serialize};
use std::sync::{Arc, Mutex, MutexGuard};
use std::time::Instant;
use once_cell::sync::Lazy;
use base64::Engine as _;

use crate::training_state::{TrainingState, TrainingStatus, TrainingConfig, TrainingMetrics, TrainingControlFlags};

// ============================================================================
// Training Implementation Using rustpy-ml
// ============================================================================

/// Initialize Python ML environment - runs once
fn init_python_ml() -> rustpy_ml::Result<()> {
    static INIT: Lazy<()> = Lazy::new(|| {
        let _ = rustpy_ml::init_with_ml(&["numpy", "torch", "sklearn"]);
    });
    Lazy::force(&INIT);
    Ok(())
}

/// Define the trainer class in Python - this is a one-time setup
fn setup_trainer_class() -> rustpy_ml::Result<()> {
    py_exec!(r#"
class IrisTrainer:
    """Neural network trainer for Iris classification using PyTorch."""
    
    def __init__(self, hidden_nodes, lr, batch_size, seed=42):
        import numpy as np
        import torch
        import torch.nn as nn
        import torch.optim as optim
        from sklearn.datasets import load_iris
        from sklearn.model_selection import train_test_split
        from sklearn.preprocessing import StandardScaler
        import io
        
        # Initialize random seeds
        np.random.seed(seed)
        torch.manual_seed(seed)
        
        # Load and prepare data
        iris = load_iris()
        X = iris.data.astype(np.float32)
        y = iris.target.astype(np.int64)
        
        # Split: 60% train, 20% val, 20% test
        X_train_val, self.X_test, y_train_val, self.y_test = train_test_split(
            X, y, test_size=0.2, random_state=seed, stratify=y
        )
        self.X_train, self.X_val, self.y_train, self.y_val = train_test_split(
            X_train_val, y_train_val, test_size=0.25, random_state=seed, stratify=y_train_val
        )
        
        # Normalize features
        scaler = StandardScaler()
        self.X_train = scaler.fit_transform(self.X_train).astype(np.float32)
        self.X_val = scaler.transform(self.X_val).astype(np.float32)
        self.X_test = scaler.transform(self.X_test).astype(np.float32)
        
        # Convert to PyTorch tensors
        self.X_train_t = torch.tensor(self.X_train, dtype=torch.float32)
        self.y_train_t = torch.tensor(self.y_train, dtype=torch.long)
        self.X_val_t = torch.tensor(self.X_val, dtype=torch.float32)
        self.y_val_t = torch.tensor(self.y_val, dtype=torch.long)
        self.X_test_t = torch.tensor(self.X_test, dtype=torch.float32)
        self.y_test_t = torch.tensor(self.y_test, dtype=torch.long)
        
        # Build network dynamically
        layers = []
        prev_size = 4  # Iris has 4 features
        for hidden_size in hidden_nodes:
            layers.append(nn.Linear(prev_size, int(hidden_size)))
            layers.append(nn.ReLU())
            layers.append(nn.Dropout(0.2))
            prev_size = int(hidden_size)
        layers.append(nn.Linear(prev_size, 3))  # 3 classes
        
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
        self.io = io
    
    def train_epoch(self):
        """Train for one epoch and return (loss, accuracy)."""
        import torch
        import base64
        
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
        
        # Validate
        self.model.eval()
        with torch.no_grad():
            val_logits = self.model(self.X_val_t)
            val_pred = torch.argmax(val_logits, dim=1)
            val_acc = float((val_pred == self.y_val_t).float().mean().item())
            self.val_accs.append(val_acc)
        
        # Save best model
        if val_acc > self.best_acc:
            self.best_acc = val_acc
            buffer = self.io.BytesIO()
            torch.save(self.model.state_dict(), buffer)
            self.model_state = base64.b64encode(buffer.getvalue()).decode('utf-8')
        
        if train_loss < self.best_loss:
            self.best_loss = float(train_loss)
        
        return (float(train_loss), float(val_acc))
    
    def finalize(self):
        """Evaluate on test set and return all results as tuple."""
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
            self.model_state or ""
        )

# Trainer class definition flag
"#)?;
    Ok(())
}

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

// ============================================================================
// State Management
// ============================================================================

/// Global training state
static TRAINING_STATE: Lazy<Arc<Mutex<TrainingState>>> = Lazy::new(|| {
    Arc::new(Mutex::new(TrainingState::default()))
});

/// Global control flags
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

// ============================================================================
// REST Endpoints
// ============================================================================

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

    // Validate input
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

    match st.status {
        TrainingStatus::Running | TrainingStatus::Paused => {
            return json(ErrorResponse { error: "Training already in progress".to_string() });
        }
        _ => {}
    }

    // Reset and configure state
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

    drop(st);

    // Spawn training task
    let state_clone = state.clone();
    let flags_clone = get_flags().clone();
    
    tokio::spawn(async move {
        run_training_task(state_clone, config, flags_clone).await;
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

// ============================================================================
// Core Training Logic - Simplified using rustpy-ml
// ============================================================================

/// Run training task in background with control signal support
async fn run_training_task(
    state: Arc<Mutex<TrainingState>>,
    config: StartTrainingRequest,
    flags: Arc<TrainingControlFlags>,
) {
    let start_time = Instant::now();

    // Initialize Python and setup trainer once
    if let Err(e) = init_python_ml() {
        update_state_error(&state, format!("Failed to init Python: {}", e));
        return;
    }

    if let Err(e) = setup_trainer_class() {
        update_state_error(&state, format!("Failed to setup trainer class: {}", e));
        return;
    }

    // Build nodes list for Python
    let nodes_list = format!(
        "[{}]",
        config
            .nodes_per_layer
            .iter()
            .map(|n| n.to_string())
            .collect::<Vec<_>>()
            .join(", ")
    );

    // Create trainer instance
    let init_trainer = format!(
        "trainer = IrisTrainer({}, {}, {})",
        nodes_list, config.learning_rate, config.batch_size
    );

    if let Err(e) = py_exec!(init_trainer.as_str()) {
        update_state_error(&state, format!("Trainer init failed: {}", e));
        return;
    }

    // Training epoch loop
    for epoch in 0..config.epochs {
        // Check stop signal
        if flags.should_stop() {
            let mut st = lock_state(&state);
            st.status = TrainingStatus::Stopped;
            st.metrics.elapsed_seconds = start_time.elapsed().as_secs_f64();
            return;
        }

        // Handle pause signal
        while flags.is_paused() && !flags.should_stop() {
            tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
        }

        // Run one epoch of training
        match python!(-> (f64, f64), "trainer.train_epoch()") {
            Ok((train_loss, val_acc)) => {
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
            }
            Err(e) => {
                update_state_error(&state, format!("Epoch {} failed: {}", epoch + 1, e));
                return;
            }
        }
    }

    // Finalize training and get results
    match python!(-> (Vec<f64>, Vec<f64>, f64, f64, f64, String), "trainer.finalize()") {
        Ok((train_losses, val_accs, best_loss, best_acc, _test_acc, model_b64)) => {
            let mut st = lock_state(&state);
            st.status = TrainingStatus::Completed;
            st.metrics.elapsed_seconds = start_time.elapsed().as_secs_f64();
            st.metrics.train_loss_history = train_losses;
            st.metrics.val_accuracy_history = val_accs;
            st.metrics.best_loss = best_loss;
            st.metrics.best_accuracy = best_acc;

            // Decode and store model
            if !model_b64.is_empty() {
                if let Ok(model_bytes) =
                    base64::engine::general_purpose::STANDARD.decode(&model_b64)
                {
                    st.model_data = Some(model_bytes);
                    st.model_path = Some("iris_model.pt".to_string());
                }
            }

            st.metrics.training_logs.push(format!(
                "✓ Training complete! Loss: {:.4}, Val Acc: {:.2}%",
                best_loss,
                best_acc * 100.0
            ));
        }
        Err(e) => {
            update_state_error(&state, format!("Finalization failed: {}", e));
        }
    }
}

/// Helper to update state with error status
fn update_state_error(state: &Arc<Mutex<TrainingState>>, error: String) {
    let mut st = lock_state(state);
    st.status = TrainingStatus::Completed;
    st.error_message = Some(error);
}
