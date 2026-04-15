use std::time::Instant;

use saluki_app::bootstrap::AppBootstrapper;
use saluki_components::config::{DatadogRemapper, KEY_ALIASES};
use saluki_config::ConfigurationLoader;
use tempfile::TempDir;

use crate::cli::handle_run_command;

fn create_config_dir() -> TempDir {
    let dir = TempDir::new().expect("failed to create temp dir");
    let config = r#"
hostname: "test-host"
api_key: "deadbeefdeadbeefdeadbeefdeadbeef"
data_plane:
  standalone_mode: true
  dogstatsd:
    enabled: true
"#;
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

#[tokio::test]
async fn boots_and_shuts_down() {
    let config_dir = create_config_dir();
    let (config_path, config) = build_config(&config_dir).await;

    let _bootstrap_guard = AppBootstrapper::from_configuration(&config)
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

    // Give ADP a moment to start up, then shut it down.
    tokio::time::sleep(std::time::Duration::from_secs(2)).await;
    let _ = shutdown_tx.send(());

    let result = handle.await.expect("task panicked");
    // For now, just see what happens — we expect this to fail due to missing config,
    // and the error will tell us what we need to add.
    println!("Result: {:?}", result);
}
