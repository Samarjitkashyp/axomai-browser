//! Tests for Axomai Browser v1.2.0: Web Crypto, Web Audio API, Web Workers, WebAuthn, and WPT Runner.

use rust_engine::crypto_engine::{CryptoDigestAlgorithm, CryptoEngine};
use rust_engine::web_audio::{AudioContext, AudioContextState, OscillatorType, BiquadFilterType};
use rust_engine::worker_engine::{OffscreenCanvas, WorkerThreadPool, WorkerState};
use rust_engine::credentials_engine::{CredentialsManager, PublicKeyCredentialCreationOptions, PublicKeyCredentialRequestOptions, UserVerificationRequirement};
use rust_engine::wpt_runner::{WptAssertion, WptRunner, WptStatus};

#[test]
fn test_web_crypto_digest_and_cipher() {
    let mut crypto = CryptoEngine::new();
    
    // Random buffer fill
    let mut rand_buf = [0u8; 16];
    crypto.get_random_values(&mut rand_buf);
    assert!(rand_buf.iter().any(|&b| b != 0));

    // SHA-256 Digest
    let hash = crypto.digest(CryptoDigestAlgorithm::Sha256, b"AxomaiSecureEngine");
    assert_eq!(hash.len(), 32);

    // Key Generation and AES simulation
    let key = crypto.generate_key("AES-GCM", vec!["encrypt".to_string(), "decrypt".to_string()], 256);
    assert_eq!(key.raw_bytes.len(), 32);

    let plaintext = b"Confidential Browser Payload";
    let encrypted = crypto.encrypt(&key, plaintext).unwrap();
    assert_eq!(encrypted.len(), plaintext.len());

    let decrypted = crypto.decrypt(&key, &encrypted).unwrap();
    assert_eq!(&decrypted, plaintext);
}

#[test]
fn test_web_audio_graph_nodes_and_connections() {
    let mut audio_ctx = AudioContext::new(44100);
    assert_eq!(audio_ctx.state, AudioContextState::Running);

    let osc_id = audio_ctx.create_oscillator(OscillatorType::Sine, 440.0);
    let filter_id = audio_ctx.create_biquad_filter(BiquadFilterType::Lowpass, 1200.0);
    let gain_id = audio_ctx.create_gain(0.8);

    // Connect: Oscillator -> Filter -> Gain -> Destination
    assert!(audio_ctx.connect(osc_id, filter_id).is_ok());
    assert!(audio_ctx.connect(filter_id, gain_id).is_ok());
    assert!(audio_ctx.connect(gain_id, audio_ctx.destination_id).is_ok());

    audio_ctx.suspend();
    assert_eq!(audio_ctx.state, AudioContextState::Suspended);
    audio_ctx.resume();
    assert_eq!(audio_ctx.state, AudioContextState::Running);
}

#[test]
fn test_worker_thread_pool_and_offscreen_canvas() {
    let mut pool = WorkerThreadPool::new();
    let worker_id = pool.spawn("background_task.js");
    assert_eq!(worker_id, 1);

    assert!(pool.post_message_to_worker(worker_id, "start_computation").is_ok());
    let worker = pool.workers.get(&worker_id).unwrap();
    assert_eq!(worker.inbox.len(), 1);
    assert_eq!(worker.state, WorkerState::Running);

    pool.terminate_worker(worker_id);
    let terminated_worker = pool.workers.get(&worker_id).unwrap();
    assert_eq!(terminated_worker.state, WorkerState::Terminated);

    let canvas = OffscreenCanvas::new(800, 600, "webgpu");
    assert_eq!(canvas.frame_buffer.len(), 800 * 600 * 4);
    let bitmap = canvas.transfer_to_image_bitmap();
    assert_eq!(bitmap.len(), canvas.frame_buffer.len());
}

#[test]
fn test_webauthn_credential_creation_and_assertion() {
    let mut creds = CredentialsManager::new();

    let creation_opts = PublicKeyCredentialCreationOptions {
        rp_id: "axomai.org".to_string(),
        rp_name: "Axomai Browser".to_string(),
        user_id: vec![1, 2, 3, 4],
        user_name: "samarjit".to_string(),
        challenge: vec![0xDE, 0xAD, 0xBE, 0xEF],
    };

    let created_cred = creds.create_public_key_credential(creation_opts).unwrap();
    assert!(created_cred.id.contains("axomai.org"));
    assert!(created_cred.client_data_json.contains("webauthn.create"));

    let request_opts = PublicKeyCredentialRequestOptions {
        challenge: vec![0xCA, 0xFE, 0xBA, 0xBE],
        rp_id: "axomai.org".to_string(),
        user_verification: UserVerificationRequirement::Required,
    };

    let assertion = creds.get_public_key_credential(request_opts).unwrap();
    assert!(assertion.signature.is_some());
    assert!(assertion.client_data_json.contains("webauthn.get"));
}

#[test]
fn test_wpt_runner_and_report_generation() {
    let mut runner = WptRunner::new();

    runner.record_test(
        "/css/css-grid/grid-model-001.html",
        WptStatus::Pass,
        vec![WptAssertion {
            name: "Grid tracks sizing".to_string(),
            status: WptStatus::Pass,
            message: None,
        }],
        15,
    );

    runner.record_test(
        "/dom/traversal/closest-001.html",
        WptStatus::Pass,
        vec![WptAssertion {
            name: "Element.closest ancestor match".to_string(),
            status: WptStatus::Pass,
            message: None,
        }],
        8,
    );

    let report = runner.generate_report("Axomai W3C WPT Suite");
    assert_eq!(report.total_tests, 2);
    assert_eq!(report.passed_tests, 2);
    assert_eq!(report.pass_rate_percentage, 100.0);
}
