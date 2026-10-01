use std::process::{Child, Command, Stdio};
use std::sync::mpsc::{channel, Receiver, Sender};
use std::sync::{Arc, Mutex};
use std::thread::{self, JoinHandle};
use std::time::Instant;

use crate::adblock_engine::AdBlockEngine;
use crate::engine::AxomaiEngine;
use crate::native_compositor::NativeGpuCompositor;
use crate::painter::DisplayCommand;

/// Execution runtime mode for child processes
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExecutionMode {
    ThreadWorker,
    OsChildProcess,
}

/// Process Identifiers within Axomai Multi-Process Architecture
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ProcessType {
    BrowserMain,
    Renderer(u32), // tab_id
    Network,
    Gpu,
}

/// Status of a child process
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProcessStatus {
    Starting,
    Running,
    Busy,
    Recovering,
    Terminated,
}

/// IPC Messages exchanged across isolated process boundaries
#[derive(Debug, Clone)]
pub enum IpcMessage {
    // Browser Main -> Network
    FetchRequest {
        req_id: u64,
        url: String,
        renderer_id: u32,
    },
    // Network -> Browser Main / Renderer
    FetchResponse {
        req_id: u64,
        status: u16,
        body: String,
        blocked: bool,
    },
    // Browser Main -> Renderer
    RenderHtml {
        renderer_id: u32,
        html: String,
        viewport_w: f32,
        viewport_h: f32,
    },
    // Renderer -> GPU
    SubmitDisplayList {
        renderer_id: u32,
        commands: Vec<DisplayCommand>,
        width: u32,
        height: u32,
    },
    // GPU -> Browser Main
    FramePresented {
        renderer_id: u32,
        frame_index: u64,
        duration_ms: f32,
    },
    // Heartbeats
    Heartbeat {
        process_type: ProcessType,
        timestamp: u64,
    },
    // Shutdown
    Terminate,
}

/// Process descriptor monitored by the Supervisor
pub struct ManagedProcess {
    pub process_type: ProcessType,
    pub status: ProcessStatus,
    pub execution_mode: ExecutionMode,
    pub last_heartbeat: Instant,
    pub tx: Sender<IpcMessage>,
    pub thread_handle: Option<JoinHandle<()>>,
    pub os_child: Option<Arc<Mutex<Child>>>,
}

/// Axomai Multi-Process Supervisor
pub struct ProcessSupervisor {
    processes: Arc<Mutex<Vec<ManagedProcess>>>,
    main_tx: Sender<IpcMessage>,
    main_rx: Arc<Mutex<Receiver<IpcMessage>>>,
    next_renderer_id: u32,
    is_active: bool,
    pub default_mode: ExecutionMode,
}

impl ProcessSupervisor {
    pub fn new() -> Self {
        let (tx, rx) = channel();
        Self {
            processes: Arc::new(Mutex::new(Vec::new())),
            main_tx: tx,
            main_rx: Arc::new(Mutex::new(rx)),
            next_renderer_id: 1,
            is_active: true,
            default_mode: ExecutionMode::OsChildProcess,
        }
    }

    /// Spawns the dedicated Network Process with real HTTP fetching and status code propagation
    pub fn spawn_network_process(&mut self) {
        let (tx, rx) = channel::<IpcMessage>();
        let main_tx = self.main_tx.clone();
        let adblock = AdBlockEngine::new();

        let handle = thread::spawn(move || {
            let mut adblock = adblock;
            while let Ok(msg) = rx.recv() {
                match msg {
                    IpcMessage::FetchRequest {
                        req_id,
                        url,
                        renderer_id: _,
                    } => {
                        let blocked = adblock.should_block_url(&url);
                        let response = if blocked {
                            IpcMessage::FetchResponse {
                                req_id,
                                status: 403,
                                body: "Blocked by Axomai AdBlock Engine".to_string(),
                                blocked: true,
                            }
                        } else {
                            match crate::network::URL::parse(&url) {
                                Ok(parsed_url) => {
                                    let (headers, body) = parsed_url.request();
                                    let status_code = headers
                                        .get("status")
                                        .and_then(|s| s.parse::<u16>().ok())
                                        .unwrap_or(200);

                                    IpcMessage::FetchResponse {
                                        req_id,
                                        status: status_code,
                                        body,
                                        blocked: false,
                                    }
                                }
                                Err(err) => IpcMessage::FetchResponse {
                                    req_id,
                                    status: 400,
                                    body: format!(
                                        "<html><body><h1>URL Parse Error</h1><p>{}</p></body></html>",
                                        err
                                    ),
                                    blocked: false,
                                },
                            }
                        };
                        let _ = main_tx.send(response);
                    }
                    IpcMessage::Terminate => break,
                    _ => {}
                }
            }
        });

        let mut procs = self.processes.lock().unwrap();
        procs.push(ManagedProcess {
            process_type: ProcessType::Network,
            status: ProcessStatus::Running,
            execution_mode: ExecutionMode::ThreadWorker,
            last_heartbeat: Instant::now(),
            tx,
            thread_handle: Some(handle),
            os_child: None,
        });
    }

    /// Spawns the dedicated GPU Compositor Process
    pub fn spawn_gpu_process(&mut self) {
        let (tx, rx) = channel::<IpcMessage>();
        let main_tx = self.main_tx.clone();

        let handle = thread::spawn(move || {
            let mut frame_count = 0u64;
            while let Ok(msg) = rx.recv() {
                match msg {
                    IpcMessage::SubmitDisplayList {
                        renderer_id,
                        commands,
                        width,
                        height,
                    } => {
                        let start = Instant::now();
                        let mut compositor = NativeGpuCompositor::new(width, height);
                        let _frame_idx = compositor.present(&commands);
                        frame_count += 1;
                        let duration = start.elapsed().as_secs_f32() * 1000.0;

                        let _ = main_tx.send(IpcMessage::FramePresented {
                            renderer_id,
                            frame_index: frame_count,
                            duration_ms: duration,
                        });
                    }
                    IpcMessage::Terminate => break,
                    _ => {}
                }
            }
        });

        let mut procs = self.processes.lock().unwrap();
        procs.push(ManagedProcess {
            process_type: ProcessType::Gpu,
            status: ProcessStatus::Running,
            execution_mode: ExecutionMode::ThreadWorker,
            last_heartbeat: Instant::now(),
            tx,
            thread_handle: Some(handle),
            os_child: None,
        });
    }

    /// Spawns a dedicated Tab Renderer Process
    pub fn spawn_renderer_process(&mut self) -> u32 {
        let renderer_id = self.next_renderer_id;
        self.next_renderer_id += 1;

        let (tx, rx) = channel::<IpcMessage>();
        let main_tx = self.main_tx.clone();

        let handle = thread::spawn(move || {
            let mut engine = AxomaiEngine::new();
            while let Ok(msg) = rx.recv() {
                match msg {
                    IpcMessage::RenderHtml {
                        renderer_id: id,
                        html,
                        viewport_w,
                        viewport_h,
                    } => {
                        let _ = engine.load_html(&html, viewport_w, viewport_h);
                        let cmds = engine.display_list.clone();
                        let _ = main_tx.send(IpcMessage::SubmitDisplayList {
                            renderer_id: id,
                            commands: cmds,
                            width: viewport_w as u32,
                            height: viewport_h as u32,
                        });
                    }
                    IpcMessage::Terminate => break,
                    _ => {}
                }
            }
        });

        let mut procs = self.processes.lock().unwrap();
        procs.push(ManagedProcess {
            process_type: ProcessType::Renderer(renderer_id),
            status: ProcessStatus::Running,
            execution_mode: ExecutionMode::ThreadWorker,
            last_heartbeat: Instant::now(),
            tx,
            thread_handle: Some(handle),
            os_child: None,
        });

        renderer_id
    }

    /// Spawns a real operating system child process
    pub fn spawn_os_child_process(&mut self, proc_type: ProcessType, executable: &str, args: &[&str]) -> Result<(), String> {
        let (tx, _rx) = channel::<IpcMessage>();
        
        let child = Command::new(executable)
            .args(args)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .spawn()
            .map_err(|e| format!("Failed to spawn OS process {}: {}", executable, e))?;

        let mut procs = self.processes.lock().unwrap();
        procs.push(ManagedProcess {
            process_type: proc_type,
            status: ProcessStatus::Running,
            execution_mode: ExecutionMode::OsChildProcess,
            last_heartbeat: Instant::now(),
            tx,
            thread_handle: None,
            os_child: Some(Arc::new(Mutex::new(child))),
        });

        Ok(())
    }

    /// Dispatches an IPC message to a target process
    pub fn send_to(&self, target: ProcessType, msg: IpcMessage) -> bool {
        let procs = self.processes.lock().unwrap();
        for p in procs.iter() {
            if p.process_type == target {
                return p.tx.send(msg).is_ok();
            }
        }
        false
    }

    /// Dispatches render work to a specific renderer
    pub fn render_in_tab(&self, renderer_id: u32, html: &str, width: f32, height: f32) -> bool {
        self.send_to(
            ProcessType::Renderer(renderer_id),
            IpcMessage::RenderHtml {
                renderer_id,
                html: html.to_string(),
                viewport_w: width,
                viewport_h: height,
            },
        )
    }

    /// Dispatches URL fetch to Network process
    pub fn request_url(&self, req_id: u64, url: &str, renderer_id: u32) -> bool {
        self.send_to(
            ProcessType::Network,
            IpcMessage::FetchRequest {
                req_id,
                url: url.to_string(),
                renderer_id,
            },
        )
    }

    /// Reads incoming messages from supervised processes
    pub fn poll_message(&self) -> Option<IpcMessage> {
        let rx = self.main_rx.lock().unwrap();
        rx.try_recv().ok()
    }

    /// Counts active managed processes
    pub fn active_process_count(&self) -> usize {
        let procs = self.processes.lock().unwrap();
        procs.iter().filter(|p| p.status == ProcessStatus::Running).count()
    }

    /// Shuts down all supervised child processes cleanly
    pub fn shutdown_all(&mut self) {
        let mut procs = self.processes.lock().unwrap();
        for p in procs.iter_mut() {
            let _ = p.tx.send(IpcMessage::Terminate);
            p.status = ProcessStatus::Terminated;
            if let Some(ref os_child_arc) = p.os_child {
                if let Ok(mut child) = os_child_arc.lock() {
                    let _ = child.kill();
                }
            }
        }
        self.is_active = false;
    }
}

impl Drop for ProcessSupervisor {
    fn drop(&mut self) {
        if self.is_active {
            self.shutdown_all();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_process_supervisor_lifecycle() {
        let mut supervisor = ProcessSupervisor::new();
        supervisor.spawn_network_process();
        supervisor.spawn_gpu_process();
        let tab1 = supervisor.spawn_renderer_process();

        assert_eq!(supervisor.active_process_count(), 3);

        // Send a network request with about: scheme
        assert!(supervisor.request_url(101, "about:blank", tab1));
        thread::sleep(Duration::from_millis(60));

        let mut found_response = false;
        while let Some(msg) = supervisor.poll_message() {
            if let IpcMessage::FetchResponse { req_id, status, .. } = msg {
                if req_id == 101 {
                    assert_eq!(status, 200);
                    found_response = true;
                }
            }
        }
        assert!(found_response);

        // Test Render flow
        assert!(supervisor.render_in_tab(tab1, "<h1>Multi-Process Axomai</h1>", 800.0, 600.0));
        thread::sleep(Duration::from_millis(60));

        supervisor.shutdown_all();
    }

    #[test]
    fn test_adblock_in_network_process() {
        let mut supervisor = ProcessSupervisor::new();
        supervisor.spawn_network_process();
        let tab = supervisor.spawn_renderer_process();

        // Ad URL should be blocked
        supervisor.request_url(202, "https://doubleclick.net/ad.js", tab);
        thread::sleep(Duration::from_millis(60));

        let mut blocked_detected = false;
        while let Some(msg) = supervisor.poll_message() {
            if let IpcMessage::FetchResponse { req_id, status, blocked, .. } = msg {
                if req_id == 202 {
                    assert_eq!(status, 403);
                    assert!(blocked);
                    blocked_detected = true;
                }
            }
        }
        assert!(blocked_detected);

        supervisor.shutdown_all();
    }
}
