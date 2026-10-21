//! Phase 3 verification spikes and runtime integration tests
//! (docs/roadmap.md Phase 3; design addenda 1 and 7).
//!
//! These tests require:
//! - runtimes fetched via scripts/packaging/fetch-llama-server.ps1
//! - a small GGUF model at ../../../models/qwen2.5-0.5b-instruct-q4_k_m.gguf
//!
//! Run: cargo test --test runtime_spikes -- --test-threads=1 --nocapture

use std::path::PathBuf;
use std::time::{Duration, Instant};

use omnira_lib::gguf;
use omnira_lib::runtime;
use omnira_lib::types::RuntimeVariant;

fn model_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../models/qwen2.5-0.5b-instruct-q4_k_m.gguf")
}

fn have_model() -> bool {
    model_path().is_file()
}

#[test]
fn gguf_inspect_valid_model() {
    if !have_model() {
        eprintln!("SKIP: test model not present");
        return;
    }
    let info = gguf::inspect(&model_path()).expect("valid GGUF must pass inspection");
    println!(
        "gguf: version={} ctx={:?} name={:?}",
        info.version, info.trained_context_length, info.model_name
    );
    assert!(info.trained_context_length.unwrap_or(0) > 0);
}

#[test]
fn gguf_inspect_rejects_garbage() {
    let tmp = std::env::temp_dir().join("omnira-not-a-model.gguf");
    std::fs::write(&tmp, b"definitely not a gguf file").unwrap();
    let err = gguf::inspect(&tmp).expect_err("garbage must be rejected");
    assert_eq!(err.code, omnira_lib::errors::ErrorCode::ModelFormatInvalid);
    std::fs::remove_file(&tmp).ok();
}

#[test]
fn gguf_inspect_rejects_truncated_metadata_value() {
    let tmp = std::env::temp_dir().join("omnira-truncated-metadata.gguf");
    let mut bytes = Vec::new();
    bytes.extend_from_slice(&0x4655_4747u32.to_le_bytes()); // GGUF
    bytes.extend_from_slice(&3u32.to_le_bytes()); // version
    bytes.extend_from_slice(&0u64.to_le_bytes()); // tensor count
    bytes.extend_from_slice(&1u64.to_le_bytes()); // metadata kv count
    bytes.extend_from_slice(&4u64.to_le_bytes()); // key length
    bytes.extend_from_slice(b"test");
    bytes.extend_from_slice(&0u32.to_le_bytes()); // u8 metadata value, then EOF
    std::fs::write(&tmp, bytes).unwrap();

    let err = gguf::inspect(&tmp).expect_err("truncated metadata value must be rejected");
    assert_eq!(err.code, omnira_lib::errors::ErrorCode::ModelFormatInvalid);
    std::fs::remove_file(&tmp).ok();
}

async fn start_runtime() -> runtime::ManagedRuntime {
    runtime::start(
        None,
        None,
        None,
        model_path().display().to_string(),
        "spike-model".to_string(),
        Some(32768),
    )
    .await
    .expect("runtime must start (Vulkan or CPU fallback)")
}

async fn wait_for_idle_slot(
    client: &reqwest::Client,
    base: &str,
    key: &str,
    slot_id: i64,
    budget: Duration,
) -> Result<(), String> {
    let deadline = tokio::time::Instant::now() + budget;
    loop {
        let remaining = deadline.saturating_duration_since(tokio::time::Instant::now());
        if remaining.is_zero() {
            return Err(format!(
                "slot {slot_id} remained busy after disconnect for {budget:?}"
            ));
        }
        let slots = client
            .get(format!("{base}/slots"))
            .bearer_auth(key)
            .timeout(remaining)
            .send()
            .await
            .map_err(|e| format!("slot probe: {e}"))?
            .error_for_status()
            .map_err(|e| format!("slot status: {e}"))?
            .json::<serde_json::Value>()
            .await
            .map_err(|e| format!("slot body: {e}"))?;
        let processing = slots
            .as_array()
            .and_then(|slots| {
                slots
                    .iter()
                    .find(|slot| slot["id"].as_i64() == Some(slot_id))
            })
            .and_then(|slot| slot["is_processing"].as_bool())
            .ok_or_else(|| format!("slot {slot_id} missing from monitoring response"))?;
        if !processing {
            return Ok(());
        }
        tokio::time::sleep(Duration::from_millis(20).min(remaining)).await;
    }
}

#[tokio::test]
async fn cancellation_probe_cannot_pass_using_a_different_idle_slot() {
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let base = format!("http://{}", listener.local_addr().unwrap());
    let busy = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(true));
    let server_busy = busy.clone();
    let server = tokio::spawn(async move {
        loop {
            let (mut socket, _) = listener.accept().await.unwrap();
            let mut request = Vec::new();
            let mut buffer = [0; 1024];
            while !request.windows(4).any(|part| part == b"\r\n\r\n") {
                let n = socket.read(&mut buffer).await.unwrap();
                if n == 0 {
                    break;
                }
                request.extend_from_slice(&buffer[..n]);
            }
            let body = serde_json::json!([
                {"id": 0, "is_processing": server_busy.load(std::sync::atomic::Ordering::SeqCst)},
                {"id": 1, "is_processing": false},
            ])
            .to_string();
            let response = format!("HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}", body.len());
            let _ = socket.write_all(response.as_bytes()).await;
        }
    });
    let client = reqwest::Client::new();
    assert!(
        wait_for_idle_slot(&client, &base, "test-key", 0, Duration::from_millis(100))
            .await
            .is_err(),
        "another idle slot must not mask a generation that never cancels"
    );
    busy.store(false, std::sync::atomic::Ordering::SeqCst);
    wait_for_idle_slot(&client, &base, "test-key", 0, Duration::from_millis(100))
        .await
        .unwrap();
    assert!(
        wait_for_idle_slot(&client, &base, "test-key", 2, Duration::from_millis(100))
            .await
            .is_err(),
        "an absent slot must fail closed"
    );
    server.abort();
}

/// Spike (a): CORS/webview-origin. Sends a preflight OPTIONS request with the
/// Tauri webview origin and inspects the Access-Control-* response headers.
/// Also verifies an actual cross-origin-style POST works with the api-key.
#[tokio::test]
async fn spike_cors_and_streaming() {
    if !have_model() {
        eprintln!("SKIP: test model not present");
        return;
    }
    let rt = start_runtime().await;
    println!("runtime up: variant={:?} port={}", rt.variant, rt.port);
    let base = format!("http://127.0.0.1:{}", rt.port);
    let client = reqwest::Client::new();

    // Preflight, as WebView2 would send it from http://tauri.localhost.
    let preflight = client
        .request(
            reqwest::Method::OPTIONS,
            format!("{base}/v1/chat/completions"),
        )
        .header("Origin", "http://tauri.localhost")
        .header("Access-Control-Request-Method", "POST")
        .header(
            "Access-Control-Request-Headers",
            "authorization,content-type",
        )
        .send()
        .await
        .expect("preflight request");
    let allow_origin = preflight
        .headers()
        .get("access-control-allow-origin")
        .map(|v| v.to_str().unwrap_or("").to_string());
    let allow_headers = preflight
        .headers()
        .get("access-control-allow-headers")
        .map(|v| v.to_str().unwrap_or("").to_string());
    println!(
        "CORS SPIKE RESULT: status={} allow-origin={allow_origin:?} allow-headers={allow_headers:?}",
        preflight.status()
    );
    assert!(
        allow_origin.is_some(),
        "llama-server must answer preflight with Access-Control-Allow-Origin for the direct path"
    );

    // Missing api-key must be rejected (UnauthorizedLocalRequest boundary).
    let unauthorized = client
        .post(format!("{base}/v1/chat/completions"))
        .json(&serde_json::json!({
            "messages": [{"role": "user", "content": "hi"}],
        }))
        .send()
        .await
        .expect("request without key");
    assert_eq!(
        unauthorized.status(),
        reqwest::StatusCode::UNAUTHORIZED,
        "llama-server must reject requests without the session api-key"
    );

    // Streaming with the key works and yields chunks.
    let resp = client
        .post(format!("{base}/v1/chat/completions"))
        .header("Origin", "http://tauri.localhost")
        .bearer_auth(&rt.api_key)
        .json(&serde_json::json!({
            "messages": [{"role": "user", "content": "Say hello in one short sentence."}],
            "stream": true,
            "max_tokens": 32,
        }))
        .send()
        .await
        .expect("streaming request");
    assert!(resp.status().is_success(), "streaming request must succeed");
    let body = resp.text().await.expect("stream body");
    assert!(body.contains("data: "), "response must be SSE");
    assert!(body.contains("[DONE]"), "stream must terminate with [DONE]");
    println!("STREAMING OK ({} bytes of SSE)", body.len());

    runtime::stop(rt).await;
}

/// Spike (b): cancellation-on-disconnect. Starts a long generation, drops the
/// connection while slot 0 is busy, then proves that exact slot becomes idle.
/// A follow-up on slot 0 checks reuse separately from cancellation latency.
#[tokio::test]
async fn spike_cancellation_on_disconnect() {
    if !have_model() {
        eprintln!("SKIP: test model not present");
        return;
    }
    println!("CANCELLATION: starting runtime");
    let rt = start_runtime().await;
    println!("CANCELLATION: ready variant={:?}", rt.variant);
    let base = format!("http://127.0.0.1:{}", rt.port);
    let client = reqwest::Client::new();

    // Kick off a very long generation and abandon it after the first chunk.
    let mut resp = client
        .post(format!("{base}/v1/chat/completions"))
        .bearer_auth(&rt.api_key)
        .json(&serde_json::json!({
            "messages": [{"role": "user", "content": "Write an extremely long story about the ocean. Do not stop."}],
            "stream": true,
            "max_tokens": 4000,
            "ignore_eos": true,
            "id_slot": 0,
        }))
        .timeout(Duration::from_secs(30))
        .send()
        .await
        .expect("long request");
    println!("CANCELLATION: long response headers received");
    assert!(resp.status().is_success(), "long generation must succeed");
    let _first = resp
        .chunk()
        .await
        .expect("first chunk")
        .expect("some bytes");
    let slots = client
        .get(format!("{base}/slots"))
        .bearer_auth(&rt.api_key)
        .timeout(Duration::from_secs(30))
        .send()
        .await
        .expect("busy-slot probe")
        .error_for_status()
        .unwrap()
        .json::<serde_json::Value>()
        .await
        .unwrap();
    assert_eq!(
        slots
            .as_array()
            .unwrap()
            .iter()
            .find(|slot| slot["id"] == 0)
            .unwrap()["is_processing"],
        true,
        "negative control: slot 0 must still be generating before disconnect"
    );
    println!("CANCELLATION: first chunk received; disconnecting");
    drop(resp); // closes the connection mid-generation

    let cancelled_at = Instant::now();
    wait_for_idle_slot(&client, &base, &rt.api_key, 0, Duration::from_secs(20))
        .await
        .expect("the generating slot must become idle after disconnect");
    println!(
        "CANCELLATION: slot 0 became idle in {:?}",
        cancelled_at.elapsed()
    );

    // The slot must be free: a short request should complete promptly.
    let t0 = Instant::now();
    println!("CANCELLATION: sending follow-up");
    let short = client
        .post(format!("{base}/v1/chat/completions"))
        .bearer_auth(&rt.api_key)
        .json(&serde_json::json!({
            "messages": [{"role": "user", "content": "Say OK."}],
            "max_tokens": 8,
            "id_slot": 0,
        }))
        .timeout(Duration::from_secs(30))
        .send()
        .await
        .expect("short request after disconnect");
    assert!(short.status().is_success());
    let elapsed = t0.elapsed();
    println!("CANCELLATION SPIKE RESULT: follow-up completed in {elapsed:?}");

    runtime::stop(rt).await;
}

/// Port-race retry and fallback machinery: starting two runtimes back to back
/// must yield distinct healthy ports.
#[tokio::test]
async fn runtime_restart_cycles_cleanly() {
    if !have_model() {
        eprintln!("SKIP: test model not present");
        return;
    }
    let rt1 = start_runtime().await;
    let port1 = rt1.port;
    runtime::stop(rt1).await;

    let rt2 = start_runtime().await;
    println!("restart: port {} -> {}", port1, rt2.port);
    assert!(matches!(
        rt2.variant,
        RuntimeVariant::Cuda | RuntimeVariant::Vulkan | RuntimeVariant::Cpu
    ));
    runtime::stop(rt2).await;
}

/// Force-kill orphan check, run manually (see scripts/dev/orphan-check.ps1):
/// starts a runtime and then sleeps so the harness can kill this process and
/// verify llama-server.exe dies with it via the Job Object.
#[tokio::test]
#[ignore]
async fn orphan_check_holds_runtime_for_kill() {
    let rt = start_runtime().await;
    println!("ORPHAN-CHECK-READY pid-file port={}", rt.port);
    tokio::time::sleep(Duration::from_secs(120)).await;
    runtime::stop(rt).await;
}
