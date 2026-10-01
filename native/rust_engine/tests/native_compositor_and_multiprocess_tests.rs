use axomai_engine::{
    GpuQuad, NativeFramebuffer, NativeGpuCompositor, ProcessSupervisor, ProcessType, IpcMessage,
    AxomaiEngine,
};
use std::thread;
use std::time::Duration;

#[test]
fn test_native_gpu_compositor_direct_rasterization() {
    let mut compositor = NativeGpuCompositor::new(640, 480);
    
    let mut engine = AxomaiEngine::new();
    let sample_html = r#"
        <!DOCTYPE html>
        <html>
        <head>
            <style>
                body { background-color: #1a1a24; margin: 0; }
                .card { background-color: #e65100; width: 300px; height: 150px; margin: 20px; }
                h1 { color: #ffffff; font-size: 24px; }
            </style>
        </head>
        <body>
            <div class="card">
                <h1>Axomai Native GPU Surface</h1>
            </div>
        </body>
        </html>
    "#;

    let res = engine.load_html(sample_html, 640.0, 480.0);
    assert!(res.is_ok());
    assert!(!engine.display_list.is_empty());

    // Rasterize display list to native framebuffer
    let fb = compositor.rasterize(&engine.display_list);
    assert_eq!(fb.width, 640);
    assert_eq!(fb.height, 480);
    assert_eq!(fb.pixels.len(), 640 * 480 * 4);

    // Verify GPU quad vertex conversion
    let quads = compositor.extract_gpu_quads(&engine.display_list);
    assert!(!quads.is_empty());
    for quad in quads {
        assert_eq!(quad.vertices.len(), 4);
    }
}

#[test]
fn test_multi_process_ipc_orchestration() {
    let mut supervisor = ProcessSupervisor::new();
    
    // Spawn network, GPU, and 2 renderer processes (tabs)
    supervisor.spawn_network_process();
    supervisor.spawn_gpu_process();
    let tab_wikipedia = supervisor.spawn_renderer_process();
    let tab_assam = supervisor.spawn_renderer_process();

    assert_eq!(supervisor.active_process_count(), 4);

    // Dispatch load to Tab 1
    supervisor.render_in_tab(
        tab_wikipedia,
        "<html><body><h1>Wikipedia - The Free Encyclopedia</h1></body></html>",
        1024.0,
        768.0,
    );

    // Dispatch load to Tab 2
    supervisor.render_in_tab(
        tab_assam,
        "<html><body><h1>অসমীয়া ৱেব ব্ৰাউজাৰ - Axomai</h1></body></html>",
        1024.0,
        768.0,
    );

    // Fetch network asset
    supervisor.request_url(1001, "https://api.axomai.org/v1/news", tab_assam);

    // Wait for async execution
    thread::sleep(Duration::from_millis(100));

    let mut message_count = 0;
    while let Some(_msg) = supervisor.poll_message() {
        message_count += 1;
    }
    assert!(message_count > 0, "Supervisor should receive IPC responses from child processes");

    supervisor.shutdown_all();
}
