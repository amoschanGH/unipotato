//! Timing instrumentation utilities for benchmarking
//!
//! This module provides utilities to measure latency at various stages of request processing.
//! Used for classification of bottlenecks in Unipotato + rustpy stack.

use std::time::{Instant, Duration};
use std::sync::Mutex;
use once_cell::sync::Lazy;

/// Benchmark metrics for a single inference request
#[derive(Debug, Clone)]
pub struct RequestMetrics {
    pub request_start: Instant,
    pub routing_done: Option<Instant>,
    pub body_read_done: Option<Instant>,
    pub json_parse_done: Option<Instant>,
    pub spawn_block_enter: Option<Instant>,
    pub spawn_block_acquired: Option<Instant>,
    pub inference_done: Option<Instant>,
    pub response_serialize_done: Option<Instant>,
    pub response_send: Option<Instant>,
}

impl RequestMetrics {
    pub fn new() -> Self {
        Self {
            request_start: Instant::now(),
            routing_done: None,
            body_read_done: None,
            json_parse_done: None,
            spawn_block_enter: None,
            spawn_block_acquired: None,
            inference_done: None,
            response_serialize_done: None,
            response_send: None,
        }
    }

    /// Record total service time since request start
    pub fn total_duration(&self) -> Duration {
        if let Some(end) = self.response_send {
            end.duration_since(self.request_start)
        } else {
            self.request_start.elapsed()
        }
    }

    /// Framework path latency (routing + body read + json parse)
    pub fn framework_latency(&self) -> Duration {
        let end = self.json_parse_done.unwrap_or_else(Instant::now);
        end.duration_since(self.request_start)
    }

    /// Spawn blocking overhead (queue wait time before GIL acquisition)
    pub fn spawn_block_overhead(&self) -> Duration {
        match (self.spawn_block_enter, self.spawn_block_acquired) {
            (Some(enter), Some(acquired)) => acquired.duration_since(enter),
            _ => Duration::ZERO,
        }
    }

    /// Python execution time (GIL held + ML compute)
    pub fn python_execution_time(&self) -> Duration {
        match (self.spawn_block_acquired, self.inference_done) {
            (Some(start), Some(end)) => end.duration_since(start),
            _ => Duration::ZERO,
        }
    }

    /// Response assembly time (serialization + send)
    pub fn response_assembly_time(&self) -> Duration {
        match (self.inference_done, self.response_send) {
            (Some(start), Some(end)) => end.duration_since(start),
            _ => Duration::ZERO,
        }
    }

    /// Print human-readable timing summary
    pub fn print_summary(&self) {
        eprintln!(
            "⏱️  Breakdown: framework={:?}, spawn_block={:?}, python={:?}, response={:?}, total={:?}",
            self.framework_latency(),
            self.spawn_block_overhead(),
            self.python_execution_time(),
            self.response_assembly_time(),
            self.total_duration(),
        );
    }
}

impl Default for RequestMetrics {
    fn default() -> Self {
        Self::new()
    }
}

// Thread-local storage for current request metrics
thread_local! {
    static REQUEST_METRICS: Lazy<Mutex<Option<RequestMetrics>>> =
        Lazy::new(|| Mutex::new(None));
}

/// Start timing for a new request
pub fn start_request() {
    REQUEST_METRICS.with(|m| {
        *m.lock().unwrap() = Some(RequestMetrics::new());
    });
}

/// Get current request metrics
pub fn current_metrics() -> Option<RequestMetrics> {
    REQUEST_METRICS.with(|m| {
        if let Ok(locked) = m.try_lock() {
            locked.as_ref().cloned()
        } else {
            None
        }
    })
}

/// Record routing completion time
pub fn record_routing_done() {
    REQUEST_METRICS.with(|m| {
        if let Ok(mut locked) = m.try_lock() {
            if let Some(ref mut metrics) = *locked {
                metrics.routing_done = Some(Instant::now());
            }
        }
    });
}

/// Record body read completion time
pub fn record_body_read_done() {
    REQUEST_METRICS.with(|m| {
        if let Ok(mut locked) = m.try_lock() {
            if let Some(ref mut metrics) = *locked {
                metrics.body_read_done = Some(Instant::now());
            }
        }
    });
}

/// Record JSON parse completion time
pub fn record_json_parse_done() {
    REQUEST_METRICS.with(|m| {
        if let Ok(mut locked) = m.try_lock() {
            if let Some(ref mut metrics) = *locked {
                metrics.json_parse_done = Some(Instant::now());
            }
        }
    });
}

/// Record spawn_blocking entry time
pub fn record_spawn_block_enter() {
    REQUEST_METRICS.with(|m| {
        if let Ok(mut locked) = m.try_lock() {
            if let Some(ref mut metrics) = *locked {
                metrics.spawn_block_enter = Some(Instant::now());
            }
        }
    });
}

/// Record spawn_blocking GIL acquisition time
pub fn record_spawn_block_acquired() {
    REQUEST_METRICS.with(|m| {
        if let Ok(mut locked) = m.try_lock() {
            if let Some(ref mut metrics) = *locked {
                metrics.spawn_block_acquired = Some(Instant::now());
            }
        }
    });
}

/// Record inference completion time
pub fn record_inference_done() {
    REQUEST_METRICS.with(|m| {
        if let Ok(mut locked) = m.try_lock() {
            if let Some(ref mut metrics) = *locked {
                metrics.inference_done = Some(Instant::now());
            }
        }
    });
}

/// Record response serialization completion time
pub fn record_response_serialize_done() {
    REQUEST_METRICS.with(|m| {
        if let Ok(mut locked) = m.try_lock() {
            if let Some(ref mut metrics) = *locked {
                metrics.response_serialize_done = Some(Instant::now());
            }
        }
    });
}

/// Record response send completion time
pub fn record_response_send() {
    REQUEST_METRICS.with(|m| {
        if let Ok(mut locked) = m.try_lock() {
            if let Some(ref mut metrics) = *locked {
                metrics.response_send = Some(Instant::now());
                // Print timing summary when request completes
                if let Some(ref m) = *locked {
                    m.print_summary();
                }
            }
        }
    });
}
