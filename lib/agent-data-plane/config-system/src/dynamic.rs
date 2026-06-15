//! Typed, scoped dynamic configuration updates.
//!
//! This resolves the design's open routing-mechanism question with the clarity-over-cleverness
//! default: **re-translate the snapshot, diff, and route**. Config updates are rare and not a hot
//! path, so re-running the whole translation costs nothing meaningful and is obviously correct.
//!
//! Three fixed invariants hold:
//!
//! 1. The configuration system owns the stream and is the *sole* receiver of inbound updates. The
//!    router here consumes the single [`ConfigUpdate`] receiver from the stream handle.
//! 2. Updates are typed. A component watches a native config slice (for example
//!    [`DatadogForwarderConfig`]), never a string key, and never holds a `GenericConfiguration`.
//! 3. Each handle is scoped to its own slice. The router pushes a new value onto a slice's channel
//!    only when *that slice* changed, so a change to another component's config physically cannot
//!    reach it — blast-radius containment, reasoned about locally.
//!
//! The forwarder API-key refresh — the design's worked example — is just one instance of the
//! scoped-handle pattern: [`DynamicConfigHandles::forwarder`] carries the whole
//! [`DatadogForwarderConfig`], so a refreshed API key arrives as a typed value, not a re-read of a
//! retained map.

use agent_data_plane_config::{SalukiConfiguration, SalukiPrivateConfiguration};
use datadog_agent_config::DatadogConfiguration;
use saluki_component_config::{DatadogForwarderConfig, DogStatsDConfig, PrefixFilterConfig, TagFilterlistConfig};
use saluki_config::{dynamic::ConfigUpdate, upsert};
use tokio::sync::{mpsc, watch};
use tracing::{debug, warn};

use crate::bootstrap::read_pipeline_gates_value;
use crate::translate::translate_datadog;

/// A scoped, typed handle a single component holds to observe updates to *its* config slice.
///
/// A component can read the latest value and await the next change. Because the channel only ever
/// carries this slice's type, the component cannot observe another component's configuration.
#[derive(Clone)]
pub struct ScopedConfigHandle<T> {
    rx: watch::Receiver<T>,
}

impl<T: Clone> ScopedConfigHandle<T> {
    /// Returns the current value for this slice.
    pub fn current(&self) -> T {
        self.rx.borrow().clone()
    }

    /// Waits for the next change to this slice and returns the new value.
    ///
    /// Returns `None` if the configuration system has shut down.
    pub async fn changed(&mut self) -> Option<T> {
        match self.rx.changed().await {
            Ok(()) => Some(self.rx.borrow().clone()),
            Err(_) => None,
        }
    }
}

/// The set of typed, scoped handles handed to components.
///
/// Each field is an independent channel; this is what makes blast-radius containment physical rather
/// than conventional.
#[derive(Clone)]
pub struct DynamicConfigHandles {
    /// Runtime log level.
    pub log_level: ScopedConfigHandle<Option<String>>,

    /// Datadog forwarder configuration (carries refreshed API keys/endpoints).
    pub forwarder: ScopedConfigHandle<DatadogForwarderConfig>,

    /// DogStatsD prefix/blocklist filter configuration.
    pub prefix_filter: ScopedConfigHandle<PrefixFilterConfig>,

    /// Metric tag filterlist configuration.
    pub tag_filterlist: ScopedConfigHandle<TagFilterlistConfig>,

    /// DogStatsD source configuration.
    pub dogstatsd_source: ScopedConfigHandle<DogStatsDConfig>,
}

struct Senders {
    log_level: watch::Sender<Option<String>>,
    forwarder: watch::Sender<DatadogForwarderConfig>,
    prefix_filter: watch::Sender<PrefixFilterConfig>,
    tag_filterlist: watch::Sender<TagFilterlistConfig>,
    dogstatsd_source: watch::Sender<DogStatsDConfig>,
}

/// Routes inbound config updates to typed, scoped handles by re-translating and diffing.
pub struct ConfigUpdateRouter {
    senders: Senders,
    private: SalukiPrivateConfiguration,
    last_snapshot: serde_json::Value,
}

impl ConfigUpdateRouter {
    /// Builds a router seeded with the initial configuration, returning the scoped handles a
    /// topology hands to its components.
    ///
    /// `initial_snapshot` is the authoritative snapshot value the initial [`SalukiConfiguration`] was
    /// translated from; the router applies partial updates onto it before re-translating.
    pub fn new(
        initial: &SalukiConfiguration, initial_snapshot: serde_json::Value, private: SalukiPrivateConfiguration,
    ) -> (Self, DynamicConfigHandles) {
        let (log_level_tx, log_level_rx) = watch::channel(initial.logging.log_level.clone());
        let (forwarder_tx, forwarder_rx) = watch::channel(initial.forwarder.datadog.clone());
        let (prefix_tx, prefix_rx) = watch::channel(initial.dogstatsd.prefix_filter.clone());
        let (tag_tx, tag_rx) = watch::channel(initial.dogstatsd.tag_filterlist.clone());
        let (dsd_tx, dsd_rx) = watch::channel(initial.dogstatsd.source.clone());

        let handles = DynamicConfigHandles {
            log_level: ScopedConfigHandle { rx: log_level_rx },
            forwarder: ScopedConfigHandle { rx: forwarder_rx },
            prefix_filter: ScopedConfigHandle { rx: prefix_rx },
            tag_filterlist: ScopedConfigHandle { rx: tag_rx },
            dogstatsd_source: ScopedConfigHandle { rx: dsd_rx },
        };

        let router = Self {
            senders: Senders {
                log_level: log_level_tx,
                forwarder: forwarder_tx,
                prefix_filter: prefix_tx,
                tag_filterlist: tag_tx,
                dogstatsd_source: dsd_tx,
            },
            private,
            last_snapshot: initial_snapshot,
        };

        (router, handles)
    }

    /// Runs the routing loop until the stream closes.
    pub async fn run(mut self, mut updates: mpsc::Receiver<ConfigUpdate>) {
        while let Some(update) = updates.recv().await {
            match update {
                ConfigUpdate::Snapshot(snapshot) => self.last_snapshot = snapshot,
                ConfigUpdate::Partial { key, value } => upsert(&mut self.last_snapshot, &key, value),
            }
            self.apply();
        }
        debug!("Config update stream closed; dynamic routing stopped.");
    }

    /// Re-translate the current snapshot and route changed slices to their handles.
    fn apply(&mut self) {
        let dd_config: DatadogConfiguration = match serde_json::from_value(self.last_snapshot.clone()) {
            Ok(config) => config,
            Err(e) => {
                warn!(error = %e, "Failed to parse updated Datadog snapshot; keeping previous configuration.");
                return;
            }
        };
        let gates = read_pipeline_gates_value(&self.last_snapshot);
        let next = translate_datadog(&dd_config, &self.private, gates);

        // Each `send_if_modified` only fires receivers when the slice actually changed, so a handle
        // observes a change only when *its* slice changed.
        self.senders
            .log_level
            .send_if_modified(|current| replace_if_changed(current, next.logging.log_level));
        self.senders
            .forwarder
            .send_if_modified(|current| replace_if_changed(current, next.forwarder.datadog));
        self.senders
            .prefix_filter
            .send_if_modified(|current| replace_if_changed(current, next.dogstatsd.prefix_filter));
        self.senders
            .tag_filterlist
            .send_if_modified(|current| replace_if_changed(current, next.dogstatsd.tag_filterlist));
        self.senders
            .dogstatsd_source
            .send_if_modified(|current| replace_if_changed(current, next.dogstatsd.source));
    }
}

/// Replace `current` with `next` when they differ, reporting whether a change occurred.
fn replace_if_changed<T: PartialEq>(current: &mut T, next: T) -> bool {
    if *current == next {
        false
    } else {
        *current = next;
        true
    }
}

#[cfg(test)]
mod tests {
    use agent_data_plane_config::RuntimeConfigLanguage;

    use super::*;
    use crate::translate::{translate_datadog as translate, PipelineGates};

    fn snapshot_with(api_key: &str, log_level: &str, dsd_port: i64) -> serde_json::Value {
        serde_json::json!({
            "api_key": api_key,
            "log_level": log_level,
            "dogstatsd_port": dsd_port,
            "data_plane": { "enabled": true, "dogstatsd": { "enabled": true } },
        })
    }

    #[tokio::test]
    async fn routes_only_changed_slices_to_scoped_handles() {
        let private = SalukiPrivateConfiguration::for_language(RuntimeConfigLanguage::DatadogAgent);

        let initial_snapshot = snapshot_with("key-1", "info", 8125);
        let dd: DatadogConfiguration = serde_json::from_value(initial_snapshot.clone()).unwrap();
        let initial = translate(&dd, &private, PipelineGates::default());

        let (router, mut handles) = ConfigUpdateRouter::new(&initial, initial_snapshot, private);

        let (tx, rx) = mpsc::channel(8);
        let task = tokio::spawn(router.run(rx));

        // A new snapshot that changes only the forwarder API key and the log level (not DogStatsD).
        tx.send(ConfigUpdate::Snapshot(snapshot_with("key-2", "debug", 8125)))
            .await
            .unwrap();

        // The forwarder handle observes the refreshed API key (the design's worked example).
        let forwarder = handles.forwarder.changed().await.expect("forwarder change");
        assert_eq!(forwarder.endpoints[0].api_keys[0].as_ref(), "key-2");

        // The log-level handle observes the new level.
        let log_level = handles.log_level.changed().await.expect("log level change");
        assert_eq!(log_level.as_deref(), Some("debug"));

        // The DogStatsD source slice did not change, so its current value is unchanged and it has no
        // pending update (scoping: an unrelated change cannot reach it).
        assert_eq!(handles.dogstatsd_source.current().port, 8125);
        assert!(!handles.dogstatsd_source.rx.has_changed().unwrap());

        drop(tx);
        let _ = task.await;
    }
}
