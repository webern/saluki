//! MRF metrics gateway transform.

use std::collections::HashSet;

use async_trait::async_trait;
use resource_accounting::{MemoryBounds, MemoryBoundsBuilder};
use saluki_config::GenericConfiguration;
use saluki_core::{
    components::{
        transforms::{Transform, TransformBuilder, TransformContext},
        ComponentContext,
    },
    data_model::event::{Event, EventType},
    topology::{EventsBuffer, OutputDefinition},
};
use saluki_error::GenericError;
use tokio::select;
use tracing::{debug, error};

use crate::config::MrfConfiguration;

/// MRF metrics gateway transform configuration.
///
/// This transform sits between the enrichment stage and the MRF-specific encoder/forwarder. It owns
/// all routing and filtering decisions for the MRF metrics pipeline:
///
/// - When MRF is disabled, all events are dropped.
/// - When MRF is enabled with no allowlist, all events are forwarded.
/// - When MRF is enabled with an allowlist, only matching events are forwarded.
///
/// The transform reads static MRF configuration from a snapshot taken at build time, and watches
/// `multi_region_failover.failover_metrics` and `multi_region_failover.metric_allowlist` for
/// dynamic updates.
pub struct MrfMetricsGatewayConfiguration {
    mrf_config: MrfConfiguration,
    configuration: Option<GenericConfiguration>,
}

impl MrfMetricsGatewayConfiguration {
    /// Creates a new `MrfMetricsGatewayConfiguration` from the given [`MrfConfiguration`].
    pub fn new(mrf_config: MrfConfiguration, configuration: GenericConfiguration) -> Self {
        Self {
            mrf_config,
            configuration: Some(configuration),
        }
    }

    /// Creates a new `MrfMetricsGatewayConfiguration` from native configuration.
    ///
    /// The static gateway mode comes from `mrf_config`; runtime updates are delivered through the
    /// configuration system's typed, scoped handles rather than a retained raw map, so no
    /// `GenericConfiguration` is held.
    pub fn from_native(mrf_config: MrfConfiguration) -> Self {
        Self {
            mrf_config,
            configuration: None,
        }
    }
}

/// Routing and filtering state for the MRF metrics gateway.
#[derive(Debug)]
enum GatewayMode {
    /// MRF is disabled or improperly configured; drop all events.
    Inactive,
    /// MRF is active and no allowlist is configured; forward all events.
    ForwardAll,
    /// MRF is active and an allowlist is configured; forward only matching events.
    FilteredForward { allowlist: HashSet<String> },
}

/// MRF metrics gateway transform.
pub struct MrfMetricsGateway {
    mrf_config: MrfConfiguration,
    mode: GatewayMode,
    configuration: Option<GenericConfiguration>,
}

impl MrfMetricsGateway {
    fn new(mrf_config: MrfConfiguration, configuration: Option<GenericConfiguration>) -> Self {
        let mode = Self::mode_for_config(&mrf_config);

        Self {
            mrf_config,
            mode,
            configuration,
        }
    }

    fn mode_for_config(mrf_config: &MrfConfiguration) -> GatewayMode {
        if !mrf_config.is_metrics_forwarding_requested() {
            GatewayMode::Inactive
        } else if mrf_config.metric_allowlist().is_empty() {
            GatewayMode::ForwardAll
        } else {
            GatewayMode::FilteredForward {
                allowlist: mrf_config.metric_allowlist().iter().cloned().collect(),
            }
        }
    }

    fn update_failover_metrics(&mut self, failover_metrics: bool) {
        self.mrf_config.set_failover_metrics(failover_metrics);
        self.mode = Self::mode_for_config(&self.mrf_config);
    }

    fn update_metric_allowlist(&mut self, metric_allowlist: Vec<String>) {
        self.mrf_config.set_metric_allowlist(metric_allowlist);
        self.mode = Self::mode_for_config(&self.mrf_config);
    }

    fn should_forward(&self, event: &Event) -> bool {
        match &self.mode {
            GatewayMode::Inactive => false,
            GatewayMode::ForwardAll => true,
            GatewayMode::FilteredForward { allowlist } => {
                let Event::Metric(metric) = event else {
                    return false;
                };
                allowlist.contains(metric.context().name().as_ref())
            }
        }
    }

    async fn process_event_batch(
        &self, mut events: EventsBuffer, context: &mut TransformContext,
    ) -> Result<(), GenericError> {
        let input_count = events.len();
        events.remove_if(|event| !self.should_forward(event));
        let forwarded_count = events.len();
        let dropped_count = input_count.saturating_sub(forwarded_count);

        let sent_count = context.dispatcher().buffered()?.send_all(events).await?;
        debug!(
            forwarded_events = sent_count,
            dropped_events = dropped_count,
            "MRF metrics gateway processed event batch."
        );

        Ok(())
    }
}

#[async_trait]
impl TransformBuilder for MrfMetricsGatewayConfiguration {
    async fn build(&self, _context: ComponentContext) -> Result<Box<dyn Transform + Send>, GenericError> {
        Ok(Box::new(MrfMetricsGateway::new(
            self.mrf_config.clone(),
            self.configuration.clone(),
        )))
    }

    fn input_event_type(&self) -> EventType {
        EventType::Metric
    }

    fn outputs(&self) -> &[OutputDefinition<EventType>] {
        static OUTPUTS: &[OutputDefinition<EventType>] = &[OutputDefinition::default_output(EventType::Metric)];
        OUTPUTS
    }
}

impl MemoryBounds for MrfMetricsGatewayConfiguration {
    fn specify_bounds(&self, builder: &mut MemoryBoundsBuilder) {
        let allowlist = self.mrf_config.metric_allowlist();
        builder
            .minimum()
            .with_single_value::<MrfMetricsGateway>("component struct")
            .with_fixed_amount("hashset overhead", std::mem::size_of::<HashSet<String>>())
            .with_fixed_amount(
                "allowlist strings",
                allowlist
                    .iter()
                    .map(|name| name.len() + std::mem::size_of::<String>())
                    .sum::<usize>(),
            )
            .with_fixed_amount(
                "hashset buckets",
                allowlist.len() * std::mem::size_of::<Option<String>>() * 2,
            );
    }
}

#[async_trait]
impl Transform for MrfMetricsGateway {
    async fn run(mut self: Box<Self>, mut context: TransformContext) -> Result<(), GenericError> {
        let mut health = context.take_health_handle();
        // String-key watchers exist only on the legacy raw-map path. On the native path the
        // configuration system delivers updates through typed, scoped handles instead.
        let mut failover_metrics_watcher = self
            .configuration
            .as_ref()
            .map(|c| c.watch_for_updates("multi_region_failover.failover_metrics"));
        let mut metric_allowlist_watcher = self
            .configuration
            .as_ref()
            .map(|c| c.watch_for_updates("multi_region_failover.metric_allowlist"));

        health.mark_ready();
        debug!(mode = ?self.mode, "MRF metrics gateway transform started.");

        loop {
            select! {
                _ = health.live() => continue,
                maybe_events = context.events().next() => match maybe_events {
                    Some(events) => {
                        if let Err(e) = self.process_event_batch(events, &mut context).await {
                            error!(error = %e, "MRF metrics gateway failed to process event batch.");
                        }
                    }
                    None => {
                        debug!("Event stream terminated, shutting down MRF metrics gateway transform.");
                        break;
                    }
                },
                (_, maybe_failover_metrics) = async { failover_metrics_watcher.as_mut().unwrap().changed::<bool>().await }, if failover_metrics_watcher.is_some() => {
                    if let Some(failover_metrics) = maybe_failover_metrics {
                        self.update_failover_metrics(failover_metrics);
                    }
                },
                (_, maybe_metric_allowlist) = async { metric_allowlist_watcher.as_mut().unwrap().changed::<Vec<String>>().await }, if metric_allowlist_watcher.is_some() => {
                    if let Some(metric_allowlist) = maybe_metric_allowlist {
                        self.update_metric_allowlist(metric_allowlist);
                    }
                },
            }
        }

        debug!("MRF metrics gateway transform stopped.");
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use saluki_core::data_model::event::{metric::Metric, Event};

    use super::*;

    fn gateway(failover_metrics: bool, metric_allowlist: Vec<String>) -> MrfMetricsGateway {
        let mrf_config = MrfConfiguration::new(
            true,
            failover_metrics,
            metric_allowlist,
            Some("mrf-api-key".to_string()),
            None,
            Some("https://mrf.example.com".to_string()),
        );
        MrfMetricsGateway::new(mrf_config, None)
    }

    #[test]
    fn failover_metrics_update_toggles_forwarding() {
        let mut gw = gateway(false, Vec::new());
        assert!(!gw.should_forward(&Event::Metric(Metric::counter("any.metric", 1.0))));

        gw.update_failover_metrics(true);
        assert!(gw.should_forward(&Event::Metric(Metric::counter("any.metric", 1.0))));

        gw.update_failover_metrics(false);
        assert!(!gw.should_forward(&Event::Metric(Metric::counter("any.metric", 1.0))));
    }

    #[test]
    fn metric_allowlist_update_changes_filtering() {
        let mut gw = gateway(true, Vec::new());

        // Empty allowlist forwards everything.
        assert!(gw.should_forward(&Event::Metric(Metric::counter("allowed.metric", 1.0))));
        assert!(gw.should_forward(&Event::Metric(Metric::counter("also.allowed", 1.0))));

        gw.update_metric_allowlist(vec!["also.allowed".to_string()]);

        assert!(!gw.should_forward(&Event::Metric(Metric::counter("allowed.metric", 1.0))));
        assert!(gw.should_forward(&Event::Metric(Metric::counter("also.allowed", 1.0))));
        assert!(!gw.should_forward(&Event::Metric(Metric::counter("blocked.metric", 1.0))));
    }
}
