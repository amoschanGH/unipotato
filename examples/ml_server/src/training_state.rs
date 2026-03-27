use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TrainingStatus {
    Idle,
    Running,
    Paused,
    Completed,
    Stopped,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrainingConfig {
    pub hidden_layers: usize,
    pub nodes_per_layer: Vec<usize>,
    pub epochs: usize,
    pub learning_rate: f64,
    pub batch_size: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrainingMetrics {
    pub current_epoch: usize,
    pub best_loss: f64,
    pub best_accuracy: f64,
    pub train_loss_history: Vec<f64>,
    pub val_accuracy_history: Vec<f64>,
    pub elapsed_seconds: f64,
    pub training_logs: Vec<String>,
}

impl Default for TrainingMetrics {
    fn default() -> Self {
        Self {
            current_epoch: 0,
            best_loss: f64::INFINITY,
            best_accuracy: 0.0,
            train_loss_history: Vec::new(),
            val_accuracy_history: Vec::new(),
            elapsed_seconds: 0.0,
            training_logs: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrainingState {
    pub status: TrainingStatus,
    pub config: Option<TrainingConfig>,
    pub metrics: TrainingMetrics,
    pub error_message: Option<String>,
    pub model_path: Option<String>,
    pub model_data: Option<Vec<u8>>,
}

impl Default for TrainingState {
    fn default() -> Self {
        Self {
            status: TrainingStatus::Idle,
            config: None,
            metrics: TrainingMetrics::default(),
            error_message: None,
            model_path: None,
            model_data: None,
        }
    }
}

/// Control flags for training (pause/stop)
pub struct TrainingControlFlags {
    pub pause_flag: Arc<AtomicBool>,
    pub stop_flag: Arc<AtomicBool>,
}

impl TrainingControlFlags {
    pub fn new() -> Self {
        Self {
            pause_flag: Arc::new(AtomicBool::new(false)),
            stop_flag: Arc::new(AtomicBool::new(false)),
        }
    }

    pub fn set_pause(&self, paused: bool) {
        self.pause_flag.store(paused, Ordering::Release);
    }

    pub fn is_paused(&self) -> bool {
        self.pause_flag.load(Ordering::Acquire)
    }

    pub fn set_stop(&self, stop: bool) {
        self.stop_flag.store(stop, Ordering::Release);
    }

    pub fn should_stop(&self) -> bool {
        self.stop_flag.load(Ordering::Acquire)
    }

    pub fn reset(&self) {
        self.pause_flag.store(false, Ordering::Release);
        self.stop_flag.store(false, Ordering::Release);
    }
}

impl Default for TrainingControlFlags {
    fn default() -> Self {
        Self::new()
    }
}
