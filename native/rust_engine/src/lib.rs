pub mod accessibility_engine;
pub mod adblock_engine;
pub mod ai_assistant;
pub mod android_bridge;
pub mod capture_studio;
pub mod credentials_engine;
pub mod crypto_engine;
pub mod css_parser;
pub mod download_manager;
pub mod engine;
pub mod ffi;
pub mod glyph_atlas;
pub mod html_parser;
pub mod js_engine;
pub mod layout;
pub mod media_decoder;
pub mod native_compositor;
pub mod network;
pub mod painter;
pub mod performance_limiter;
pub mod permissions_engine;
pub mod pip_engine;
pub mod process_manager;
pub mod reader_mode;
pub mod split_view;
pub mod sync_engine;
pub mod theme_engine;
pub mod updater_engine;
pub mod vector_engine;
pub mod vpn_doh_engine;
pub mod wasm_engine;
pub mod web3_wallet;
pub mod web_audio;
pub mod webgpu_engine;
pub mod webrtc_engine;
pub mod worker_engine;
pub mod workspaces_engine;
pub mod wpt_runner;

pub use accessibility_engine::{AccessibilityTree, AccessibleNode, AriaRole};
pub use adblock_engine::{AdBlockEngine, AdBlockStats};
pub use ai_assistant::{AiAssistant, SupportedLanguage, TranslationEngine};
pub use capture_studio::{AnnotationType, CaptureStudio};
pub use credentials_engine::{CredentialsManager, PublicKeyCredential};
pub use crypto_engine::{CryptoDigestAlgorithm, CryptoEngine, CryptoKey};
pub use css_parser::CSSParser;
pub use download_manager::{DownloadItem, DownloadManager, DownloadState};
pub use engine::AxomaiEngine;
pub use ffi::*;
pub use html_parser::HTMLParser;
pub use js_engine::V8JSEngine;
pub use media_decoder::{MediaDemuxer, MediaPlaybackPipeline};
pub use native_compositor::{
    GpuBufferStream, GpuHardwareDevice, GpuPipelineDescriptor, GpuQuad, GpuSwapchainPresenter,
    GpuVertex, NativeFramebuffer, NativeGpuCompositor,
};
#[cfg(feature = "wgpu-backend")]
pub use native_compositor::{WgpuRenderer, WgpuVertex};
pub use performance_limiter::ResourceLimiter;
pub use permissions_engine::{PermissionName, PermissionState, PermissionsManager};
pub use pip_engine::PipEngine;
pub use process_manager::{
    ExecutionMode, IpcMessage, ManagedProcess, ProcessStatus, ProcessSupervisor, ProcessType,
    run_subprocess,
};
pub use reader_mode::{ReaderArticle, ReaderModeEngine};
pub use split_view::{SplitLayoutMode, SplitViewEngine};
pub use sync_engine::{SyncDataType, SyncEngine, SyncRecord};
pub use theme_engine::{ColorMode, HeritagePreset, ThemeEngine};
pub use updater_engine::{ReleaseManifest, UpdateChannel, UpdateStatus, UpdaterEngine};
pub use vector_engine::{PdfDocument, SvgParser, SvgPathCommand};
pub use vpn_doh_engine::{DohProvider, ProxyProtocol, VpnDohEngine};
pub use wasm_engine::{WasmInstance, WasmModule, WasmVal};
pub use web3_wallet::{SupportedChain, Web3Wallet};
pub use web_audio::{AudioContext, AudioNode, OscillatorType};
pub use webgpu_engine::{GpuCanvasContext, GpuDevice, WebGl2Context};
pub use webrtc_engine::{RtcDataChannel, RtcPeerConnection};
pub use worker_engine::{DedicatedWorker, OffscreenCanvas, WorkerThreadPool};
pub use workspaces_engine::{TabHibernationState, WorkspaceManager};
pub use wpt_runner::{WptReport, WptRunner, WptStatus};

#[no_mangle]
pub extern "C" fn rust_parse_css_count(css_text: *const std::os::raw::c_char) -> usize {
    if css_text.is_null() {
        return 0;
    }
    let c_str = unsafe { std::ffi::CStr::from_ptr(css_text) };
    if let Ok(str_slice) = c_str.to_str() {
        let parser = CSSParser::new(str_slice);
        return parser.parse().len();
    }
    0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rust_css_parser() {
        let css = "h1 { color: red; } p.intro { font-size: 16px; }";
        let parser = CSSParser::new(css);
        let rules = parser.parse();
        assert_eq!(rules.len(), 2);
    }

    #[test]
    fn test_rust_html_parser() {
        let html = "<html><body><div class='title'>Hello Rust</div></body></html>";
        let dom = HTMLParser::new(html).parse();
        assert!(dom.borrow().children.len() > 0);
    }

    #[test]
    fn test_rust_engine_pipeline() {
        let mut engine = AxomaiEngine::new();
        let html = "<html><body><h1>Axomai Engine</h1><p>Running on Rust!</p></body></html>";
        let res = engine.load_html(html, 800.0, 600.0);
        assert!(res.is_ok());
        assert!(engine.display_list.len() > 0);
    }
}
