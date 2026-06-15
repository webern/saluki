//! Typed whole-config change watcher.
//!
//! [`TotalConfigWatcher`] subscribes to all config-change events, re-runs the full translation on
//! each one, and only yields when the resulting [`TotalSalukiConfiguration`] actually changes.

use datadog_agent_config::TotalSalukiConfiguration;
use saluki_config::{dynamic::ConfigChangeEvent, GenericConfiguration};
use tokio::sync::broadcast;
use tracing::warn;

use crate::cli::otlp_native::build_total_config;

/// Watches for any configuration change and re-runs the full translation.
///
/// Yields only when the translated [`TotalSalukiConfiguration`] actually differs from the previous
/// value. If dynamic configuration is disabled, [`changed`](Self::changed) never returns.
pub struct TotalConfigWatcher {
    config: GenericConfiguration,
    rx: Option<broadcast::Receiver<ConfigChangeEvent>>,
    last: TotalSalukiConfiguration,
}

impl TotalConfigWatcher {
    /// Creates a new `TotalConfigWatcher`.
    ///
    /// `initial` should be the `TotalSalukiConfiguration` that was built at startup so the first
    /// real change is correctly detected as a diff.
    pub fn new(config: GenericConfiguration, initial: TotalSalukiConfiguration) -> Self {
        let rx = config.subscribe_for_updates();
        Self { config, rx, last: initial }
    }

    /// Waits until the translated config changes and returns `(old, new)`.
    ///
    /// Loops internally until a change is detected; callers always receive a pair where
    /// `old != new`.
    pub async fn changed(&mut self) -> (TotalSalukiConfiguration, TotalSalukiConfiguration) {
        let Some(rx) = self.rx.as_mut() else {
            std::future::pending::<()>().await;
            unreachable!();
        };

        loop {
            match rx.recv().await {
                Ok(_event) => {
                    match build_total_config(&self.config) {
                        Ok(new_config) if new_config != self.last => {
                            let old = std::mem::replace(&mut self.last, new_config.clone());
                            return (old, new_config);
                        }
                        Ok(_) => continue,
                        Err(e) => {
                            warn!(error = %e, "Failed to translate config after change event; skipping.");
                            continue;
                        }
                    }
                }
                Err(broadcast::error::RecvError::Lagged(n)) => {
                    warn!(dropped = n, "TotalConfigWatcher dropped config-change events; continuing.");
                    continue;
                }
                Err(broadcast::error::RecvError::Closed) => {
                    std::future::pending::<()>().await;
                    unreachable!();
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use saluki_config::{dynamic::ConfigUpdate, ConfigurationLoader};
    use serde_json::json;

    use crate::cli::otlp_native::build_total_config;

    use super::TotalConfigWatcher;

    // Send an update for a key that DatadogConfiguration does not know about. The translated
    // TotalSalukiConfiguration should be identical before and after, so `changed()` must NOT fire.
    #[tokio::test]
    async fn no_diff_suppression() {
        let (cfg, sender) = ConfigurationLoader::for_tests(
            Some(json!({ "log_level": "info" })),
            None,
            true,
        )
        .await;
        let sender = sender.expect("sender should exist");

        sender
            .send(ConfigUpdate::Snapshot(json!({ "log_level": "info" })))
            .await
            .unwrap();
        cfg.ready().await;

        let initial = build_total_config(&cfg).expect("initial build_total_config failed");
        let mut watcher = TotalConfigWatcher::new(cfg.clone(), initial);

        // Send a partial update for an unknown key; translated config is unchanged.
        sender
            .send(ConfigUpdate::Partial {
                key: "some_unknown_key".to_string(),
                value: json!("whatever"),
            })
            .await
            .unwrap();

        let result = tokio::time::timeout(
            std::time::Duration::from_millis(200),
            watcher.changed(),
        )
        .await;

        assert!(result.is_err(), "changed() should not have fired for a no-diff update");
    }

    // Send a real log_level change; the translated TotalSalukiConfiguration differs, so
    // `changed()` must return the correct old and new values.
    #[tokio::test]
    async fn fires_on_real_change() {
        let (cfg, sender) = ConfigurationLoader::for_tests(
            Some(json!({ "log_level": "info" })),
            None,
            true,
        )
        .await;
        let sender = sender.expect("sender should exist");

        sender
            .send(ConfigUpdate::Snapshot(json!({ "log_level": "info" })))
            .await
            .unwrap();
        cfg.ready().await;

        let initial = build_total_config(&cfg).expect("initial build_total_config failed");
        let mut watcher = TotalConfigWatcher::new(cfg.clone(), initial.clone());

        sender
            .send(ConfigUpdate::Partial {
                key: "log_level".to_string(),
                value: json!("debug"),
            })
            .await
            .unwrap();

        let (old, new) = tokio::time::timeout(
            std::time::Duration::from_secs(2),
            watcher.changed(),
        )
        .await
        .expect("timed out waiting for log_level change");

        assert_eq!(old.logs.log_level, "info");
        assert_eq!(new.logs.log_level, "debug");
    }
}
