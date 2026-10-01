//! Web Workers Multithreading & OffscreenCanvas Engine for Axomai Browser.
//! Implements HTML5 Dedicated Web Workers and OffscreenCanvas graphics rendering.

use std::collections::HashMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WorkerState {
    Initializing,
    Running,
    Terminated,
}

#[derive(Debug, Clone)]
pub struct WorkerMessage {
    pub data: String,
    pub transferable_buffers: Vec<Vec<u8>>,
}

#[derive(Debug, Clone)]
pub struct OffscreenCanvas {
    pub width: u32,
    pub height: u32,
    pub context_type: String, // "2d", "webgl2", "webgpu"
    pub frame_buffer: Vec<u8>,
}

impl OffscreenCanvas {
    pub fn new(width: u32, height: u32, context_type: &str) -> Self {
        let size = (width * height * 4) as usize;
        OffscreenCanvas {
            width,
            height,
            context_type: context_type.to_string(),
            frame_buffer: vec![0u8; size],
        }
    }

    pub fn transfer_to_image_bitmap(&self) -> Vec<u8> {
        self.frame_buffer.clone()
    }
}

pub struct DedicatedWorker {
    pub id: u32,
    pub script_url: String,
    pub state: WorkerState,
    pub inbox: Vec<WorkerMessage>,
    pub outbox: Vec<WorkerMessage>,
}

impl DedicatedWorker {
    pub fn new(id: u32, script_url: &str) -> Self {
        DedicatedWorker {
            id,
            script_url: script_url.to_string(),
            state: WorkerState::Running,
            inbox: Vec::new(),
            outbox: Vec::new(),
        }
    }

    pub fn post_message(&mut self, data: &str, transferables: Vec<Vec<u8>>) {
        if self.state == WorkerState::Running {
            self.inbox.push(WorkerMessage {
                data: data.to_string(),
                transferable_buffers: transferables,
            });
        }
    }

    pub fn emit_response(&mut self, data: &str) {
        if self.state == WorkerState::Running {
            self.outbox.push(WorkerMessage {
                data: data.to_string(),
                transferable_buffers: Vec::new(),
            });
        }
    }

    pub fn terminate(&mut self) {
        self.state = WorkerState::Terminated;
    }
}

pub struct WorkerThreadPool {
    next_worker_id: u32,
    pub workers: HashMap<u32, DedicatedWorker>,
}

impl WorkerThreadPool {
    pub fn new() -> Self {
        WorkerThreadPool {
            next_worker_id: 1,
            workers: HashMap::new(),
        }
    }

    pub fn spawn(&mut self, script_url: &str) -> u32 {
        let id = self.next_worker_id;
        self.next_worker_id += 1;
        let worker = DedicatedWorker::new(id, script_url);
        self.workers.insert(id, worker);
        id
    }

    pub fn post_message_to_worker(&mut self, worker_id: u32, data: &str) -> Result<(), String> {
        if let Some(worker) = self.workers.get_mut(&worker_id) {
            worker.post_message(data, Vec::new());
            Ok(())
        } else {
            Err(format!("Worker {} not found", worker_id))
        }
    }

    pub fn terminate_worker(&mut self, worker_id: u32) {
        if let Some(worker) = self.workers.get_mut(&worker_id) {
            worker.terminate();
        }
    }

    pub fn terminate_all(&mut self) {
        for (_, worker) in self.workers.iter_mut() {
            worker.terminate();
        }
    }
}
