use std::{
    net::{TcpListener, UdpSocket},
    sync::{Arc, Mutex},
    time::Instant,
};

use axum::{
    body::Bytes,
    extract::State,
    http::StatusCode,
    routing::post,
    Router,
};
use saluki_app::bootstrap::AppBootstrapper;
use saluki_components::config::{DatadogRemapper, KEY_ALIASES};
use saluki_config::ConfigurationLoader;
use tempfile::TempDir;

use crate::cli::handle_run_command;

/// Grab an available port from the OS by binding to port 0 and reading back the assignment.
///
/// TODO: There is an inherent race condition for the DogStatsD port — we release it here and
/// ADP re-binds it later. Another process could claim it in between. A cross-thread port pool
/// object for the tests would probably solve this. Maybe there's another way.
fn available_port() -> u16 {
    let listener = TcpListener::bind("127.0.0.1:0").expect("failed to bind to ephemeral port");
    listener.local_addr().unwrap().port()
}

fn create_config_dir(backend_port: u16, dogstatsd_port: u16) -> TempDir {
    let dir = TempDir::new().expect("failed to create temp dir");
    let config = format!(
        r#"
hostname: "test-host"
api_key: "deadbeefdeadbeefdeadbeefdeadbeef"
dd_url: "http://127.0.0.1:{backend_port}"
dogstatsd_port: {dogstatsd_port}
log_level: "info"
data_plane:
  standalone_mode: true
  dogstatsd:
    enabled: true
"#
    );
    std::fs::write(dir.path().join("datadog.yaml"), config).expect("failed to write datadog.yaml");
    dir
}

async fn build_config(config_dir: &TempDir) -> (std::path::PathBuf, saluki_config::GenericConfiguration) {
    let config_path = config_dir.path().join("datadog.yaml");
    let config = ConfigurationLoader::default()
        .with_key_aliases(KEY_ALIASES)
        .from_yaml(&config_path)
        .expect("failed to load yaml config")
        .add_providers([DatadogRemapper::new()])
        .with_default_secrets_resolution()
        .await
        .expect("failed to load secrets resolution")
        .bootstrap_generic();
    (config_path, config)
}

#[derive(Clone, Default)]
struct CapturedRequests {
    bodies: Arc<Mutex<Vec<(String, Bytes)>>>,
}

async fn capture_handler(
    State(state): State<CapturedRequests>,
    axum::extract::OriginalUri(uri): axum::extract::OriginalUri,
    body: Bytes,
) -> StatusCode {
    let path = uri.path().to_string();
    println!("Mock backend received POST {} ({} bytes)", path, body.len());
    state.bodies.lock().unwrap().push((path, body));
    StatusCode::ACCEPTED
}

/// Starts the mock backend, returning the listener's actual port.
/// No race condition here — the listener stays bound.
async fn start_mock_backend(captured: CapturedRequests) -> (u16, tokio::task::JoinHandle<()>) {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("failed to bind mock backend");
    let port = listener.local_addr().unwrap().port();

    let app = Router::new()
        .route("/{*path}", post(capture_handler))
        .with_state(captured);

    let handle = tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });

    (port, handle)
}

#[tokio::test]
async fn send_metric_and_capture_output() {
    // Start the mock backend to capture forwarded payloads.
    let captured = CapturedRequests::default();
    let (backend_port, _backend_handle) = start_mock_backend(captured.clone()).await;

    // Grab an ephemeral port for DogStatsD.
    let dogstatsd_port = available_port();

    // Build config and bootstrap ADP.
    let config_dir = create_config_dir(backend_port, dogstatsd_port);
    let (config_path, config) = build_config(&config_dir).await;

    let _guard = AppBootstrapper::from_configuration(&config)
        .expect("failed to create bootstrapper")
        .with_metrics_prefix("adp")
        .bootstrap()
        .await
        .expect("failed to bootstrap");

    let (shutdown_tx, shutdown_rx) = tokio::sync::oneshot::channel::<()>();

    let handle = tokio::spawn(async move {
        handle_run_command(
            Instant::now(),
            config_path,
            config,
            async { let _ = shutdown_rx.await; },
        )
        .await
    });

    // Wait for ADP to be healthy.
    // TODO: use a health probe instead of a fixed time interval
    tokio::time::sleep(std::time::Duration::from_secs(1)).await;

    // Send a DogStatsD metric.
    let socket = UdpSocket::bind("127.0.0.1:0").expect("failed to bind UDP socket");
    socket
        .send_to(
            b"my.test.metric:1|c",
            format!("127.0.0.1:{dogstatsd_port}"),
        )
        .expect("failed to send DogStatsD packet");

    println!("Sent DogStatsD packet, waiting for aggregation flush...");

    // Poll for the mock backend to receive something.
    // The aggregation window is 10s, plus encoding and forwarding time.
    // TODO: how can we choose a better timeout or make this deterministically exist exactly
    // when things have either worked or they havent?
    let deadline = tokio::time::Instant::now() + std::time::Duration::from_secs(30);
    loop {
        if !captured.bodies.lock().unwrap().is_empty() {
            break;
        }
        if tokio::time::Instant::now() >= deadline {
            panic!("Timed out waiting for mock backend to receive a request");
        }
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
    }

    // 7. Check what the mock backend received.
    let requests = captured.bodies.lock().unwrap();
    println!("Mock backend received {} requests", requests.len());
    for (path, body) in requests.iter() {
        println!("  POST {} ({} bytes)", path, body.len());
    }
    assert!(!requests.is_empty(), "Expected at least one request to the mock backend");

    // 8. Shut down ADP.
    let _ = shutdown_tx.send(());
    let result = handle.await.expect("task panicked");
    println!("ADP result: {:?}", result);
}
