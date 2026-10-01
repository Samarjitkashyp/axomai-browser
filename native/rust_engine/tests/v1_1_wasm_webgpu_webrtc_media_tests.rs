//! Tests for Axomai Browser v1.1.0: WebAssembly, WebGPU, WebRTC, and Media Pipeline.

use axomai_engine::wasm_engine::{WasmInstance, WasmModule, WasmOpcode, WasmVal, WASM_MAGIC, WASM_VERSION};
use axomai_engine::webgpu_engine::{GpuCommandEncoder, GpuDevice, GpuPrimitiveTopology, WebGl2Context};
use axomai_engine::webrtc_engine::{RtcIceCandidate, RtcPeerConnection, RtcSignalingState};
use axomai_engine::media_decoder::{MediaContainerFormat, MediaDemuxer, MediaPlaybackPipeline, VideoCodec, AudioCodec};

#[test]
fn test_wasm_binary_header_validation() {
    let mut valid_bytes = vec![];
    valid_bytes.extend_from_slice(&WASM_MAGIC);
    valid_bytes.extend_from_slice(&WASM_VERSION);

    let module_res = WasmModule::from_bytes(&valid_bytes);
    assert!(module_res.is_ok());

    let invalid_bytes = vec![0x00, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00];
    let bad_res = WasmModule::from_bytes(&invalid_bytes);
    assert!(bad_res.is_err());
}

#[test]
fn test_wasm_vm_arithmetic_execution() {
    let mut module = WasmModule::empty();
    module.functions.push(axomai_engine::wasm_engine::WasmFunc {
        type_idx: 0,
        locals: vec![],
        instructions: vec![
            WasmOpcode::LocalGet(0),
            WasmOpcode::LocalGet(1),
            WasmOpcode::I32Add,
            WasmOpcode::I32Const(10),
            WasmOpcode::I32Mul,
            WasmOpcode::Return,
        ],
    });

    module.exports.insert(
        "compute".to_string(),
        axomai_engine::wasm_engine::WasmExport {
            name: "compute".to_string(),
            kind: 0,
            index: 0,
        },
    );

    let mut instance = WasmInstance::new(module);
    let result = instance.call_export("compute", &[WasmVal::I32(5), WasmVal::I32(7)]).unwrap();
    assert_eq!(result, Some(WasmVal::I32(120))); // (5 + 7) * 10 = 120
}

#[test]
fn test_webgpu_device_and_encoder() {
    let mut device = GpuDevice::new();
    let buffer_id = device.create_buffer(1024, 0x0020);
    assert_eq!(buffer_id, 1);

    let shader_id = device.create_shader_module(
        "@vertex fn vs_main() -> @builtin(position) vec4f { return vec4f(0.0); }",
        true,
    );
    assert_eq!(shader_id, 2);

    let pipeline_id = device.create_render_pipeline("vs_main", None, GpuPrimitiveTopology::TriangleList);
    assert_eq!(pipeline_id, 3);

    let mut encoder = GpuCommandEncoder::new();
    encoder.set_pipeline(pipeline_id);
    encoder.draw(3, 1, 0, 0);
    let commands = encoder.finish();
    assert_eq!(commands.len(), 2);
}

#[test]
fn test_webgl2_state_machine() {
    let mut gl = WebGl2Context::new(800, 600);
    gl.clear_color(0.2, 0.4, 0.8, 1.0);
    assert_eq!(gl.clear_color, [0.2, 0.4, 0.8, 1.0]);

    gl.bind_buffer("ARRAY_BUFFER", 42);
    assert_eq!(gl.bound_array_buffer, Some(42));

    gl.use_program(10);
    assert_eq!(gl.active_program, Some(10));

    gl.draw_arrays("TRIANGLES", 0, 3);
    assert_eq!(gl.draw_call_count, 1);
}

#[test]
fn test_webrtc_peer_connection_lifecycle() {
    let mut pc = RtcPeerConnection::new(vec!["stun:stun.l.google.com:19302".to_string()]);
    assert_eq!(pc.signaling_state, RtcSignalingState::Stable);

    let channel_id = pc.create_data_channel("chat").unwrap();
    assert_eq!(channel_id, 1);

    let mut channel = pc.data_channels.get_mut("chat").unwrap();
    assert!(channel.send("Hello Peer!").is_ok());
    assert_eq!(channel.outgoing_messages.len(), 1);

    let offer = pc.create_offer().unwrap();
    assert!(offer.sdp.contains("AxomaiWebRTC"));
    pc.set_local_description(offer).unwrap();
    assert_eq!(pc.signaling_state, RtcSignalingState::HaveLocalOffer);

    let candidate = RtcIceCandidate {
        candidate: "candidate:1 1 UDP 2130706431 192.168.1.1 50000 typ host".to_string(),
        sdp_mid: Some("0".to_string()),
        sdp_m_line_index: Some(0),
    };
    pc.add_ice_candidate(candidate);
    assert_eq!(pc.candidates.len(), 1);
}

#[test]
fn test_media_demuxer_and_pipeline() {
    // MP4 synthetic header test
    let mut mp4_bytes = vec![0x00, 0x00, 0x00, 0x18];
    mp4_bytes.extend_from_slice(b"ftypmp42");
    
    let format = MediaDemuxer::probe_format(&mp4_bytes);
    assert_eq!(format, MediaContainerFormat::Mp4);

    let tracks = MediaDemuxer::parse_metadata(&mp4_bytes).unwrap();
    assert_eq!(tracks.len(), 2);
    assert_eq!(tracks[0].video_codec, VideoCodec::H264);
    assert_eq!(tracks[1].audio_codec, AudioCodec::Aac);

    let mut pipeline = MediaPlaybackPipeline::new();
    assert!(pipeline.load_media(&mp4_bytes).is_ok());
    assert_eq!(pipeline.decoded_frames.len(), 1);

    pipeline.play();
    assert!(pipeline.is_playing);
    pipeline.seek(15.5);
    assert_eq!(pipeline.current_time_seconds, 15.5);
    pipeline.pause();
    assert!(!pipeline.is_playing);
}
