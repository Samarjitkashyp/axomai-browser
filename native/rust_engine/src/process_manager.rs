use std::io::{BufRead, BufReader, Write as IoWrite};
use std::process::{Child, Command, Stdio};
use std::sync::mpsc::{channel, Receiver, Sender};
use std::sync::{Arc, Mutex};
use std::thread::{self, JoinHandle};
use std::time::Instant;

use serde::{Deserialize, Serialize};

use crate::adblock_engine::AdBlockEngine;
use crate::engine::AxomaiEngine;
use crate::native_compositor::NativeGpuCompositor;
use crate::painter::DisplayCommand;

/// Execution runtime mode for child processes
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ExecutionMode {
    ThreadWorker,
    OsChildProcess,
}

/// Process Identifiers within Axomai Multi-Process Architecture
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ProcessType {
    BrowserMain,
    Renderer(u32),
    Network,
    Gpu,
}

/// Status of a child process
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ProcessStatus {
    Starting,
    Running,
    Busy,
    Recovering,
    Terminated,
}

/// IPC Messages exchanged across isolated process boundaries.
/// Serializable via JSON for real cross-process stdin/stdout communication.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum IpcMessage {
    FetchRequest {
        req_id: u64,
        url: String,
        renderer_id: u32,
    },
    FetchResponse {
        req_id: u64,
        status: u16,
        body: String,
        blocked: bool,
    },
    RenderHtml {
        renderer_id: u32,
        html: String,
        viewport_w: f32,
        viewport_h: f32,
    },
    SubmitDisplayList {
        renderer_id: u32,
        commands: Vec<DisplayCommand>,
        width: u32,
        height: u32,
    },
    FramePresented {
        renderer_id: u32,
        frame_index: u64,
        duration_ms: f32,
    },
    Heartbeat {
        process_type: ProcessType,
        timestamp: u64,
    },
    Terminate,
}

impl IpcMessage {
    /// Serialize to a single JSON line for pipe transport
    pub fn to_wire(&self) -> String {
        serde_json::to_string(self).unwrap_or_default()
    }

    /// Deserialize from a JSON line received on a pipe
    pub fn from_wire(line: &str) -> Option<Self> {
        serde_json::from_str(line).ok()
    }
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

    /// Spawns the dedicated Network Process.
    /// In OsChildProcess mode, spawns a real OS process with stdin/stdout IPC.
    /// In ThreadWorker mode, uses an in-process thread.
    pub fn spawn_network_process(&mut self) {
        match self.default_mode {
            ExecutionMode::OsChildProcess => {
                self.spawn_network_os_process();
            }
            ExecutionMode::ThreadWorker => {
                self.spawn_network_thread();
            }
        }
    }

    fn spawn_network_thread(&mut self) {
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

    fn spawn_network_os_process(&mut self) {
        let executable = std::env::current_exe().unwrap_or_default();
        let (tx, internal_rx) = channel::<IpcMessage>();
        let main_tx = self.main_tx.clone();

        let child_result = Command::new(&executable)
            .args(["--subprocess", "network"])
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit())
            .spawn();

        match child_result {
            Ok(mut child) => {
                let child_stdin = child.stdin.take().expect("Failed to open child stdin");
                let child_stdout = child.stdout.take().expect("Failed to open child stdout");
                let child_arc = Arc::new(Mutex::new(child));

                // Writer thread: forwards IpcMessage from channel → child's stdin as JSON lines
                let stdin_handle = {
                    let mut writer = std::io::BufWriter::new(child_stdin);
                    thread::spawn(move || {
                        while let Ok(msg) = internal_rx.recv() {
                            let is_terminate = matches!(msg, IpcMessage::Terminate);
                            let line = msg.to_wire();
                            if writeln!(writer, "{}", line).is_err() {
                                break;
                            }
                            if writer.flush().is_err() {
                                break;
                            }
                            if is_terminate {
                                break;
                            }
                        }
                    })
                };

                // Reader thread: reads JSON lines from child's stdout → main_tx
                let reader_handle = thread::spawn(move || {
                    let reader = BufReader::new(child_stdout);
                    for line in reader.lines() {
                        match line {
                            Ok(l) => {
                                if let Some(msg) = IpcMessage::from_wire(&l) {
                                    if main_tx.send(msg).is_err() {
                                        break;
                                    }
                                }
                            }
                            Err(_) => break,
                        }
                    }
                });

                // Combine both handles into one supervisor thread
                let handle = thread::spawn(move || {
                    let _ = stdin_handle.join();
                    let _ = reader_handle.join();
                });

                let mut procs = self.processes.lock().unwrap();
                procs.push(ManagedProcess {
                    process_type: ProcessType::Network,
                    status: ProcessStatus::Running,
                    execution_mode: ExecutionMode::OsChildProcess,
                    last_heartbeat: Instant::now(),
                    tx,
                    thread_handle: Some(handle),
                    os_child: Some(child_arc),
                });
            }
            Err(_) => {
                // Fallback to thread mode if OS process spawn fails
                self.spawn_network_thread();
            }
        }
    }

    /// Spawns the dedicated GPU Compositor Process
    pub fn spawn_gpu_process(&mut self) {
        match self.default_mode {
            ExecutionMode::OsChildProcess => {
                self.spawn_gpu_os_process();
            }
            ExecutionMode::ThreadWorker => {
                self.spawn_gpu_thread();
            }
        }
    }

    fn spawn_gpu_thread(&mut self) {
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

    fn spawn_gpu_os_process(&mut self) {
        let executable = std::env::current_exe().unwrap_or_default();
        let (tx, internal_rx) = channel::<IpcMessage>();
        let main_tx = self.main_tx.clone();

        let child_result = Command::new(&executable)
            .args(["--subprocess", "gpu"])
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit())
            .spawn();

        match child_result {
            Ok(mut child) => {
                let child_stdin = child.stdin.take().expect("Failed to open child stdin");
                let child_stdout = child.stdout.take().expect("Failed to open child stdout");
                let child_arc = Arc::new(Mutex::new(child));

                let stdin_handle = {
                    let mut writer = std::io::BufWriter::new(child_stdin);
                    thread::spawn(move || {
                        while let Ok(msg) = internal_rx.recv() {
                            let is_terminate = matches!(msg, IpcMessage::Terminate);
                            let line = msg.to_wire();
                            if writeln!(writer, "{}", line).is_err() {
                                break;
                            }
                            if writer.flush().is_err() {
                                break;
                            }
                            if is_terminate {
                                break;
                            }
                        }
                    })
                };

                let reader_handle = thread::spawn(move || {
                    let reader = BufReader::new(child_stdout);
                    for line in reader.lines() {
                        match line {
                            Ok(l) => {
                                if let Some(msg) = IpcMessage::from_wire(&l) {
                                    if main_tx.send(msg).is_err() {
                                        break;
                                    }
                                }
                            }
                            Err(_) => break,
                        }
                    }
                });

                let handle = thread::spawn(move || {
                    let _ = stdin_handle.join();
                    let _ = reader_handle.join();
                });

                let mut procs = self.processes.lock().unwrap();
                procs.push(ManagedProcess {
                    process_type: ProcessType::Gpu,
                    status: ProcessStatus::Running,
                    execution_mode: ExecutionMode::OsChildProcess,
                    last_heartbeat: Instant::now(),
                    tx,
                    thread_handle: Some(handle),
                    os_child: Some(child_arc),
                });
            }
            Err(_) => {
                self.spawn_gpu_thread();
            }
        }
    }

    /// Spawns a dedicated Tab Renderer Process
    pub fn spawn_renderer_process(&mut self) -> u32 {
        let renderer_id = self.next_renderer_id;
        self.next_renderer_id += 1;

        match self.default_mode {
            ExecutionMode::OsChildProcess => {
                self.spawn_renderer_os_process(renderer_id);
            }
            ExecutionMode::ThreadWorker => {
                self.spawn_renderer_thread(renderer_id);
            }
        }

        renderer_id
    }

    fn spawn_renderer_thread(&mut self, renderer_id: u32) {
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
    }

    fn spawn_renderer_os_process(&mut self, renderer_id: u32) {
        let executable = std::env::current_exe().unwrap_or_default();
        let (tx, internal_rx) = channel::<IpcMessage>();
        let main_tx = self.main_tx.clone();

        let child_result = Command::new(&executable)
            .args(["--subprocess", "renderer"])
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit())
            .spawn();

        match child_result {
            Ok(mut child) => {
                let child_stdin = child.stdin.take().expect("Failed to open child stdin");
                let child_stdout = child.stdout.take().expect("Failed to open child stdout");
                let child_arc = Arc::new(Mutex::new(child));

                let stdin_handle = {
                    let mut writer = std::io::BufWriter::new(child_stdin);
                    thread::spawn(move || {
                        while let Ok(msg) = internal_rx.recv() {
                            let is_terminate = matches!(msg, IpcMessage::Terminate);
                            let line = msg.to_wire();
                            if writeln!(writer, "{}", line).is_err() {
                                break;
                            }
                            if writer.flush().is_err() {
                                break;
                            }
                            if is_terminate {
                                break;
                            }
                        }
                    })
                };

                let reader_handle = thread::spawn(move || {
                    let reader = BufReader::new(child_stdout);
                    for line in reader.lines() {
                        match line {
                            Ok(l) => {
                                if let Some(msg) = IpcMessage::from_wire(&l) {
                                    if main_tx.send(msg).is_err() {
                                        break;
                                    }
                                }
                            }
                            Err(_) => break,
                        }
                    }
                });

                let handle = thread::spawn(move || {
                    let _ = stdin_handle.join();
                    let _ = reader_handle.join();
                });

                let mut procs = self.processes.lock().unwrap();
                procs.push(ManagedProcess {
                    process_type: ProcessType::Renderer(renderer_id),
                    status: ProcessStatus::Running,
                    execution_mode: ExecutionMode::OsChildProcess,
                    last_heartbeat: Instant::now(),
                    tx,
                    thread_handle: Some(handle),
                    os_child: Some(child_arc),
                });
            }
            Err(_) => {
                self.spawn_renderer_thread(renderer_id);
            }
        }
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
                    let _ = child.wait();
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

// ─── Subprocess Worker Entry Points ───
// When the binary is invoked with `--subprocess <role>`, these run the
// isolated worker loop: read IpcMessage JSON lines from stdin, process,
// write response JSON lines to stdout.

/// Network subprocess worker: reads FetchRequest from stdin, writes FetchResponse to stdout
pub fn run_network_subprocess() {
    let stdin = std::io::stdin();
    let stdout = std::io::stdout();
    let reader = BufReader::new(stdin.lock());
    let mut writer = std::io::BufWriter::new(stdout.lock());
    let mut adblock = AdBlockEngine::new();

    for line in reader.lines() {
        let line = match line {
            Ok(l) => l,
            Err(_) => break,
        };
        let msg = match IpcMessage::from_wire(&line) {
            Some(m) => m,
            None => continue,
        };

        match msg {
            IpcMessage::FetchRequest { req_id, url, .. } => {
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
                            body: format!("<html><body><h1>URL Parse Error</h1><p>{}</p></body></html>", err),
                            blocked: false,
                        },
                    }
                };
                let out = response.to_wire();
                let _ = writeln!(writer, "{}", out);
                let _ = writer.flush();
            }
            IpcMessage::Terminate => break,
            _ => {}
        }
    }
}

/// GPU compositor subprocess worker
pub fn run_gpu_subprocess() {
    let stdin = std::io::stdin();
    let stdout = std::io::stdout();
    let reader = BufReader::new(stdin.lock());
    let mut writer = std::io::BufWriter::new(stdout.lock());
    let mut frame_count = 0u64;

    for line in reader.lines() {
        let line = match line {
            Ok(l) => l,
            Err(_) => break,
        };
        let msg = match IpcMessage::from_wire(&line) {
            Some(m) => m,
            None => continue,
        };

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

                let response = IpcMessage::FramePresented {
                    renderer_id,
                    frame_index: frame_count,
                    duration_ms: duration,
                };
                let out = response.to_wire();
                let _ = writeln!(writer, "{}", out);
                let _ = writer.flush();
            }
            IpcMessage::Terminate => break,
            _ => {}
        }
    }
}

/// Renderer subprocess worker
pub fn run_renderer_subprocess() {
    let stdin = std::io::stdin();
    let stdout = std::io::stdout();
    let reader = BufReader::new(stdin.lock());
    let mut writer = std::io::BufWriter::new(stdout.lock());
    let mut engine = AxomaiEngine::new();

    for line in reader.lines() {
        let line = match line {
            Ok(l) => l,
            Err(_) => break,
        };
        let msg = match IpcMessage::from_wire(&line) {
            Some(m) => m,
            None => continue,
        };

        match msg {
            IpcMessage::RenderHtml {
                renderer_id,
                html,
                viewport_w,
                viewport_h,
            } => {
                let _ = engine.load_html(&html, viewport_w, viewport_h);
                let cmds = engine.display_list.clone();
                let response = IpcMessage::SubmitDisplayList {
                    renderer_id,
                    commands: cmds,
                    width: viewport_w as u32,
                    height: viewport_h as u32,
                };
                let out = response.to_wire();
                let _ = writeln!(writer, "{}", out);
                let _ = writer.flush();
            }
            IpcMessage::Terminate => break,
            _ => {}
        }
    }
}

/// Dispatch to the correct subprocess worker based on role argument
pub fn run_subprocess(role: &str) {
    match role {
        "network" => run_network_subprocess(),
        "gpu" => run_gpu_subprocess(),
        "renderer" => run_renderer_subprocess(),
        _ => {
            eprintln!("[Axomai] Unknown subprocess role: {}", role);
            std::process::exit(1);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[test]
    fn test_ipc_message_serialization() {
        let msg = IpcMessage::FetchRequest {
            req_id: 42,
            url: "https://example.com".to_string(),
            renderer_id: 1,
        };
        let wire = msg.to_wire();
        let roundtrip = IpcMessage::from_wire(&wire).unwrap();
        if let IpcMessage::FetchRequest { req_id, url, renderer_id } = roundtrip {
            assert_eq!(req_id, 42);
            assert_eq!(url, "https://example.com");
            assert_eq!(renderer_id, 1);
        } else {
            panic!("Deserialization produced wrong variant");
        }
    }

    #[test]
    fn test_ipc_message_terminate_roundtrip() {
        let msg = IpcMessage::Terminate;
        let wire = msg.to_wire();
        let roundtrip = IpcMessage::from_wire(&wire);
        assert!(matches!(roundtrip, Some(IpcMessage::Terminate)));
    }

    #[test]
    fn test_process_supervisor_thread_mode() {
        let mut supervisor = ProcessSupervisor::new();
        supervisor.default_mode = ExecutionMode::ThreadWorker;

        supervisor.spawn_network_process();
        supervisor.spawn_gpu_process();
        let tab1 = supervisor.spawn_renderer_process();

        assert_eq!(supervisor.active_process_count(), 3);

        // Verify all are ThreadWorker mode
        {
            let procs = supervisor.processes.lock().unwrap();
            for p in procs.iter() {
                assert_eq!(p.execution_mode, ExecutionMode::ThreadWorker);
            }
        }

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

        assert!(supervisor.render_in_tab(tab1, "<h1>Multi-Process Axomai</h1>", 800.0, 600.0));
        thread::sleep(Duration::from_millis(60));

        supervisor.shutdown_all();
    }

    #[test]
    fn test_adblock_in_network_process() {
        let mut supervisor = ProcessSupervisor::new();
        supervisor.default_mode = ExecutionMode::ThreadWorker;
        supervisor.spawn_network_process();
        let tab = supervisor.spawn_renderer_process();

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
