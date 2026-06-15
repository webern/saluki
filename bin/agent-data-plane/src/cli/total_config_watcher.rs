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
