//! DogStatsD metric prefix and listener-side metric filter transform.

use agent_data_plane_config_system::ScopedConfigHandle;
use async_trait::async_trait;
use metrics::{Counter, Gauge};
use resource_accounting::{MemoryBounds, MemoryBoundsBuilder};
use saluki_component_config::PrefixFilterConfig;
use saluki_core::data_model::event::{metric::Metric, EventType};
use saluki_core::{
    components::{
        transforms::{Transform, TransformBuilder, TransformContext},
        ComponentContext,
    },
    observability::ComponentMetricsExt as _,
    topology::OutputDefinition,
};
use saluki_error::GenericError;
use saluki_metrics::MetricsBuilder;
use serde::Deserialize;
use tokio::select;
use tracing::{debug, error};

use crate::components::dogstatsd_filterlist::{Blocklist, EffectiveFilterlist};

const METRIC_FILTERLIST_SIZE_METRIC: &str = "metric_filterlist_size";
const METRIC_FILTERLIST_UPDATES_METRIC: &str = "metric_filterlist_updates_total";
const LISTENER_FILTERED_POINTS_METRIC: &str = "dogstatsd_listener_filtered_points_total";

/// DogStatsD prefix filter transform.
///
/// Appends a prefix to every metric if specified.
///
/// Checks if a metric name should be allowed.
#[derive(Deserialize)]
#[cfg_attr(test, derive(Debug, derive_where::DeriveWhere, serde::Serialize))]
#[cfg_attr(test, derive_where(PartialEq))]
pub struct DogStatsDPrefixFilterConfiguration {
    #[serde(default, rename = "statsd_metric_namespace")]
    metric_prefix: String,

    #[serde(
        default = "default_metric_prefix_blocklist",
        rename = "statsd_metric_namespace_blocklist",
        alias = "statsd_metric_namespace_blacklist"
    )]
    metric_prefix_blocklist: Vec<String>,

    #[serde(default)]
    metric_filterlist: Vec<String>,

    #[serde(default)]
    metric_filterlist_match_prefix: bool,

    #[serde(default, rename = "statsd_metric_blocklist")]
    metric_blocklist: Vec<String>,

    #[serde(default, rename = "statsd_metric_blocklist_match_prefix")]
    metric_blocklist_match_prefix: bool,

    #[serde(skip)]
    #[cfg_attr(test, derive_where(skip))]
    dynamic_handle: Option<ScopedConfigHandle<PrefixFilterConfig>>,
}

fn default_metric_prefix_blocklist() -> Vec<String> {
    vec![
        "datadog.agent".to_string(),
        "datadog.dogstatsd".to_string(),
        "datadog.process".to_string(),
        "datadog.trace_agent".to_string(),
        "datadog.tracer".to_string(),
        "activemq".to_string(),
        "activemq_58".to_string(),
        "airflow".to_string(),
        "cassandra".to_string(),
        "confluent".to_string(),
        "hazelcast".to_string(),
        "hive".to_string(),
        "ignite".to_string(),
        "jboss".to_string(),
        "jvm".to_string(),
        "kafka".to_string(),
        "presto".to_string(),
        "sidekiq".to_string(),
        "solr".to_string(),
        "tomcat".to_string(),
        "runtime".to_string(),
    ]
}

impl DogStatsDPrefixFilterConfiguration {
    /// Creates a new `DogStatsDPrefixFilterConfiguration` from native configuration.
    ///
    /// The initial filterlist/blocklist are taken from `native`. Dynamic updates are delivered
    /// through the typed [`ScopedConfigHandle`] attached via [`Self::with_dynamic_handle`]; absent a
    /// handle, the built transform applies no runtime updates.
    pub fn from_native(native: &PrefixFilterConfig) -> Result<Self, GenericError> {
        Ok(Self {
            metric_prefix: String::new(),
            metric_prefix_blocklist: default_metric_prefix_blocklist(),
            metric_filterlist: native.metric_filterlist.iter().map(|s| s.to_string()).collect(),
            metric_filterlist_match_prefix: native.metric_filterlist_match_prefix,
            metric_blocklist: native.metric_blocklist.iter().map(|s| s.to_string()).collect(),
            metric_blocklist_match_prefix: native.metric_blocklist_match_prefix,
            dynamic_handle: None,
        })
    }

    /// Attaches the typed, scoped dynamic-configuration handle for this slice.
    ///
    /// The built transform watches this handle and applies refreshed prefix-filter configuration at
    /// runtime, replacing the legacy raw-key configuration watchers.
    pub fn with_dynamic_handle(mut self, handle: ScopedConfigHandle<PrefixFilterConfig>) -> Self {
        self.dynamic_handle = Some(handle);
        self
    }
}

#[async_trait]
impl TransformBuilder for DogStatsDPrefixFilterConfiguration {
    fn input_event_type(&self) -> EventType {
        EventType::Metric
    }

    fn outputs(&self) -> &[OutputDefinition<EventType>] {
        static OUTPUTS: &[OutputDefinition<EventType>] = &[OutputDefinition::default_output(EventType::Metric)];
        OUTPUTS
    }

    async fn build(&self, context: ComponentContext) -> Result<Box<dyn Transform + Send>, GenericError> {
        // Ensure our metric prefix has a trailing period so that we don't have to check for, and possibly add it, when we're
        // actually processing metrics.
        let mut metric_prefix = self.metric_prefix.clone();
        if !metric_prefix.is_empty() && !metric_prefix.ends_with(".") {
            metric_prefix.push('.');
        }
        let metrics_builder = MetricsBuilder::from_component_context(&context);
        let effective_filterlist = EffectiveFilterlist::new(
            self.metric_filterlist.clone(),
            self.metric_filterlist_match_prefix,
            self.metric_blocklist.clone(),
            self.metric_blocklist_match_prefix,
        );
        let telemetry = FilterlistTelemetry::new(&metrics_builder);
        let mut filter = DogStatsDPrefixFilter {
            metric_prefix,
            metric_prefix_blocklist: self.metric_prefix_blocklist.clone(),
            matcher: Blocklist::default(),
            effective_filterlist,
            telemetry,
            dynamic_handle: self.dynamic_handle.clone(),
        };
        filter.sync_effective_blocklist(false);

        Ok(Box::new(filter))
    }
}

impl MemoryBounds for DogStatsDPrefixFilterConfiguration {
    fn specify_bounds(&self, builder: &mut MemoryBoundsBuilder) {
        // Capture the size of the heap allocation when the component is built.
        builder
            .minimum()
            .with_single_value::<DogStatsDPrefixFilter>("component struct");
    }
}

#[derive(Clone)]
struct FilterlistTelemetry {
    filterlist_size: Gauge,
    filterlist_updates: Counter,
    listener_filtered_points: Counter,
}

impl FilterlistTelemetry {
    fn new(builder: &MetricsBuilder) -> Self {
        Self {
            filterlist_size: builder.register_gauge(METRIC_FILTERLIST_SIZE_METRIC),
            filterlist_updates: builder.register_counter(METRIC_FILTERLIST_UPDATES_METRIC),
            listener_filtered_points: builder.register_counter(LISTENER_FILTERED_POINTS_METRIC),
        }
    }

    #[cfg(test)]
    fn noop() -> Self {
        Self {
            filterlist_size: Gauge::noop(),
            filterlist_updates: Counter::noop(),
            listener_filtered_points: Counter::noop(),
        }
    }

    fn increment_filterlist_updates(&self) {
        self.filterlist_updates.increment(1);
    }

    fn increment_listener_filtered_points(&self) {
        self.listener_filtered_points.increment(1);
    }

    fn set_filterlist_size(&self, size: usize) {
        self.filterlist_size.set(size as f64);
    }
}

struct DogStatsDPrefixFilter {
    metric_prefix: String,
    metric_prefix_blocklist: Vec<String>,
    matcher: Blocklist,
    effective_filterlist: EffectiveFilterlist,
    telemetry: FilterlistTelemetry,
    dynamic_handle: Option<ScopedConfigHandle<PrefixFilterConfig>>,
}

impl DogStatsDPrefixFilter {
    fn sync_effective_blocklist(&mut self, count_update: bool) {
        self.matcher = self.effective_filterlist.to_matcher();
        self.telemetry
            .set_filterlist_size(self.effective_filterlist.effective_len());
        if count_update {
            self.telemetry.increment_filterlist_updates();
        }
    }

    fn update_metric_filterlist(&mut self, metric_filterlist: Vec<String>) {
        let count_update = self.effective_filterlist.metric_filterlist_is_active() || !metric_filterlist.is_empty();
        self.effective_filterlist.set_metric_filterlist(metric_filterlist);
        self.sync_effective_blocklist(count_update);
    }

    fn update_metric_blocklist(&mut self, metric_blocklist: Vec<String>) {
        let count_update = !self.effective_filterlist.metric_filterlist_is_active();
        self.effective_filterlist.set_metric_blocklist(metric_blocklist);
        self.sync_effective_blocklist(count_update);
    }

    fn update_metric_filterlist_match_prefix(&mut self, match_prefix: bool) {
        let count_update = self.effective_filterlist.metric_filterlist_is_active();
        self.effective_filterlist
            .set_metric_filterlist_match_prefix(match_prefix);
        self.sync_effective_blocklist(count_update);
    }

    fn update_metric_blocklist_match_prefix(&mut self, match_prefix: bool) {
        let count_update = !self.effective_filterlist.metric_filterlist_is_active();
        self.effective_filterlist
            .set_metric_blocklist_match_prefix(match_prefix);
        self.sync_effective_blocklist(count_update);
    }

    /// Applies a refreshed, fully-typed prefix-filter configuration delivered over the scoped handle.
    fn apply_dynamic_update(&mut self, new_config: PrefixFilterConfig) {
        debug!("Applying dynamic prefix-filter configuration update.");
        self.update_metric_filterlist(new_config.metric_filterlist.iter().map(|s| s.to_string()).collect());
        self.update_metric_filterlist_match_prefix(new_config.metric_filterlist_match_prefix);
        self.update_metric_blocklist(new_config.metric_blocklist.iter().map(|s| s.to_string()).collect());
        self.update_metric_blocklist_match_prefix(new_config.metric_blocklist_match_prefix);
    }

    fn process_metric(&self, metric: &mut Metric) -> bool {
        let metric_name = metric.context().name().as_ref();

        if self.metric_prefix.is_empty() {
            if self.matcher.contains(metric_name) {
                self.telemetry.increment_listener_filtered_points();
                debug!("Metric {} excluded due to blocklist.", metric_name);
                return false;
            }
        } else {
            // We don't want to prefix the metric if it has a prefix that is on our _prefix_ blocklist,
            // which ensures we don't prefix metrics that are already prefixed.
            let new_metric_name = if self.has_excluded_prefix(metric_name) {
                metric.context().name().clone()
            } else {
                let mut prefixed_metric_name = self.metric_prefix.clone();
                prefixed_metric_name.push_str(metric_name);
                prefixed_metric_name.into()
            };

            if self.matcher.contains(&new_metric_name) {
                self.telemetry.increment_listener_filtered_points();
                debug!("Metric {} excluded due to blocklist.", new_metric_name);
                return false;
            }

            // Update metric with new name.
            let new_context = metric.context().with_name(new_metric_name);
            let existing_context = metric.context_mut();
            *existing_context = new_context;
        }

        true
    }

    fn has_excluded_prefix(&self, metric_name: &str) -> bool {
        !self.metric_prefix.is_empty()
            && self
                .metric_prefix_blocklist
                .iter()
                .any(|prefix| metric_name.starts_with(prefix))
    }
}

#[async_trait]
impl Transform for DogStatsDPrefixFilter {
    async fn run(mut self: Box<Self>, mut context: TransformContext) -> Result<(), GenericError> {
        let mut health = context.take_health_handle();
        health.mark_ready();

        // Dynamic updates arrive as a fully-typed `PrefixFilterConfig` over the scoped handle (the
        // configuration system re-translates and routes only changed slices here). Absent a handle,
        // the transform applies its static initial configuration with no runtime updates.
        let mut handle = self.dynamic_handle.take();

        debug!("DogStatsD Prefix Filter transform started.");

        loop {
            select! {
                _ = health.live() => continue,
                maybe_events = context.events().next() => match maybe_events {
                    Some(mut events) => {
                        events.remove_if(|event| match event.try_as_metric_mut() {
                            // `process_metric` returns `true` if the metric should be kept, so we have to invert that
                            // here to match the predicate structure, which will _remove_ the event if `true` is returned.
                            Some(metric) => !self.process_metric(metric),
                            None => true,
                        });

                        if let Err(e) = context.dispatcher().dispatch(events).await {
                            error!(error = %e, "Failed to dispatch events.");
                        }
                    },
                    None => break,
                },
                maybe_update = async { handle.as_mut().unwrap().changed().await }, if handle.is_some() => {
                    match maybe_update {
                        Some(new_config) => self.apply_dynamic_update(new_config),
                        None => handle = None,
                    }
                },
            }
        }

        debug!("DogStatsD Prefix Filter transform stopped.");

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use metrics::set_default_local_recorder;
    use saluki_metrics::{test::TestRecorder, MetricsBuilder};
    use stringtheory::MetaString;

    use super::*;

    #[test]
    fn test_metric_prefix_add() {
        let filter = DogStatsDPrefixFilter {
            metric_prefix: "foo.".to_string(),
            metric_prefix_blocklist: vec![],
            matcher: Blocklist::default(),
            effective_filterlist: EffectiveFilterlist::default(),
            telemetry: FilterlistTelemetry::noop(),
            dynamic_handle: None,
        };

        let mut metric = Metric::gauge("bar", 1.0);
        assert!(filter.process_metric(&mut metric));
        assert_eq!(metric.context().name(), "foo.bar");
    }

    #[test]
    fn test_metric_prefix_blocklist() {
        let filter = DogStatsDPrefixFilter {
            metric_prefix: "foo".to_string(),
            metric_prefix_blocklist: vec!["foo".to_string(), "bar".to_string()],
            matcher: Blocklist::default(),
            effective_filterlist: EffectiveFilterlist::default(),
            telemetry: FilterlistTelemetry::noop(),
            dynamic_handle: None,
        };

        let mut metric = Metric::gauge("barbar", 1.0);
        assert!(filter.process_metric(&mut metric));
        assert_eq!(metric.context().name(), "barbar");
    }

    #[test]
    fn test_metric_blocklist() {
        let filter = DogStatsDPrefixFilter {
            metric_prefix: "".to_string(),
            metric_prefix_blocklist: vec![],
            matcher: Blocklist::new(["foobar", "test"], false),
            effective_filterlist: EffectiveFilterlist::default(),
            telemetry: FilterlistTelemetry::noop(),
            dynamic_handle: None,
        };

        let mut metric = Metric::gauge("foobar", 1.0);
        assert!(!filter.process_metric(&mut metric));

        let mut metric = Metric::gauge("foo", 1.0);
        assert!(filter.process_metric(&mut metric));
        assert_eq!(metric.context().name(), "foo");
    }

    #[test]
    fn test_metric_blocklist_with_metric_prefix() {
        let filter = DogStatsDPrefixFilter {
            metric_prefix: "foo.".to_string(),
            metric_prefix_blocklist: vec![],
            matcher: Blocklist::new(["foo.bar", "test"], false),
            effective_filterlist: EffectiveFilterlist::default(),
            telemetry: FilterlistTelemetry::noop(),
            dynamic_handle: None,
        };

        let mut metric = Metric::gauge("bar", 1.0);
        assert!(!filter.process_metric(&mut metric));

        let filter = DogStatsDPrefixFilter {
            metric_prefix: "foo.".to_string(),
            metric_prefix_blocklist: vec!["foo".to_string()],
            matcher: Blocklist::default(),
            effective_filterlist: EffectiveFilterlist::default(),
            telemetry: FilterlistTelemetry::noop(),
            dynamic_handle: None,
        };

        let mut metric = Metric::gauge("foo", 1.0);
        assert!(filter.process_metric(&mut metric));
        assert_eq!(metric.context().name(), "foo");
    }

    #[test]
    fn test_metric_match_prefix_without_added_prefix() {
        let filter = DogStatsDPrefixFilter {
            metric_prefix: "".to_string(),
            metric_prefix_blocklist: vec![],
            matcher: Blocklist::new(["b", "test"], true),
            effective_filterlist: EffectiveFilterlist::default(),
            telemetry: FilterlistTelemetry::noop(),
            dynamic_handle: None,
        };

        // match prefix is true, "bar" has prefix "b"
        let mut metric = Metric::gauge("bar", 1.0);
        assert!(!filter.process_metric(&mut metric));

        // match prefix is true, "test" has prefix "test"
        let mut metric = Metric::gauge("test", 1.0);
        assert!(!filter.process_metric(&mut metric));
    }

    #[test]
    fn test_metric_match_prefix_with_added_prefix() {
        let filter = DogStatsDPrefixFilter {
            metric_prefix: "foo".to_string(),
            metric_prefix_blocklist: vec![],
            matcher: Blocklist::new(["fo", "test"], true),
            effective_filterlist: EffectiveFilterlist::default(),
            telemetry: FilterlistTelemetry::noop(),
            dynamic_handle: None,
        };

        // new_metric is "foo.bar", match prefix is true, "foo.bar" has prefix "fo"
        let mut metric = Metric::gauge("bar", 1.0);
        assert!(!filter.process_metric(&mut metric));
    }

    #[test]
    fn test_metric_blocklist_dynamic_update() {
        let mut filter = DogStatsDPrefixFilter {
            metric_prefix: "".to_string(),
            metric_prefix_blocklist: vec![],
            matcher: Blocklist::new(["foobar", "test"], false),
            effective_filterlist: EffectiveFilterlist::new(
                Vec::new(),
                false,
                vec!["foobar".to_string(), "test".to_string()],
                false,
            ),
            telemetry: FilterlistTelemetry::noop(),
            dynamic_handle: None,
        };

        let mut metric = Metric::gauge("foobar", 1.0);
        assert!(!filter.process_metric(&mut metric));

        let mut metric = Metric::gauge("foo", 1.0);
        assert!(filter.process_metric(&mut metric));
        assert_eq!(metric.context().name(), "foo");

        // A typed dynamic update swaps the blocklist to ["foo"].
        filter.apply_dynamic_update(PrefixFilterConfig {
            metric_filterlist: Vec::new(),
            metric_filterlist_match_prefix: false,
            metric_blocklist: vec![MetaString::from("foo")],
            metric_blocklist_match_prefix: false,
        });

        // "foobar" is taken off the blocklist
        let mut metric = Metric::gauge("foobar", 1.0);
        assert!(filter.process_metric(&mut metric));
        assert_eq!(metric.context().name(), "foobar");

        // "foo" is added to the blocklist
        let mut metric = Metric::gauge("foo", 1.0);
        assert!(!filter.process_metric(&mut metric));

        // A further typed update adds "baz" to the metric filterlist (keeping the blocklist).
        filter.apply_dynamic_update(PrefixFilterConfig {
            metric_filterlist: vec![MetaString::from("baz")],
            metric_filterlist_match_prefix: false,
            metric_blocklist: vec![MetaString::from("foo")],
            metric_blocklist_match_prefix: false,
        });

        // "baz" is added to the filterlist
        let mut metric = Metric::gauge("baz", 1.0);
        assert!(!filter.process_metric(&mut metric));
    }

    #[test]
    fn test_metric_filterlist_match_prefix_dynamic_update_is_applied() {
        let mut filter = DogStatsDPrefixFilter {
            metric_prefix: "".to_string(),
            metric_prefix_blocklist: vec![],
            matcher: Blocklist::new(["foo"], false),
            effective_filterlist: EffectiveFilterlist::new(vec!["foo".to_string()], false, Vec::new(), false),
            telemetry: FilterlistTelemetry::noop(),
            dynamic_handle: None,
        };

        let mut metric = Metric::gauge("foo.bar", 1.0);
        assert!(filter.process_metric(&mut metric));

        // A typed dynamic update enables match-prefix on the filterlist.
        filter.apply_dynamic_update(PrefixFilterConfig {
            metric_filterlist: vec![MetaString::from("foo")],
            metric_filterlist_match_prefix: true,
            metric_blocklist: Vec::new(),
            metric_blocklist_match_prefix: false,
        });

        let mut metric = Metric::gauge("foo.bar", 1.0);
        assert!(!filter.process_metric(&mut metric));
        assert_eq!(filter.matcher, Blocklist::new(["foo"], true));
    }

    #[test]
    fn telemetry_only_counts_active_filterlist_updates() {
        let recorder = TestRecorder::default();
        let _local = set_default_local_recorder(&recorder);

        let telemetry = FilterlistTelemetry::new(&MetricsBuilder::default());
        let mut filter = DogStatsDPrefixFilter {
            metric_prefix: "".to_string(),
            metric_prefix_blocklist: vec![],
            matcher: Blocklist::default(),
            effective_filterlist: EffectiveFilterlist::new(
                vec!["preferred".to_string()],
                false,
                vec!["legacy".to_string()],
                false,
            ),
            telemetry,
            dynamic_handle: None,
        };

        filter.sync_effective_blocklist(false);
        filter.update_metric_blocklist(vec!["ignored".to_string(), "still_ignored".to_string()]);

        assert_eq!(recorder.counter(METRIC_FILTERLIST_UPDATES_METRIC), Some(0));
        assert_eq!(recorder.gauge(METRIC_FILTERLIST_SIZE_METRIC), Some(1.0));

        let mut metric = Metric::gauge("preferred", 1.0);
        assert!(!filter.process_metric(&mut metric));

        let mut metric = Metric::gauge("ignored", 1.0);
        assert!(filter.process_metric(&mut metric));

        filter.update_metric_filterlist(Vec::new());

        assert_eq!(recorder.counter(METRIC_FILTERLIST_UPDATES_METRIC), Some(1));
        assert_eq!(recorder.gauge(METRIC_FILTERLIST_SIZE_METRIC), Some(2.0));

        let mut metric = Metric::gauge("ignored", 1.0);
        assert!(!filter.process_metric(&mut metric));
    }

    #[test]
    fn telemetry_counts_active_reconfiguration_even_if_matcher_is_unchanged() {
        let recorder = TestRecorder::default();
        let _local = set_default_local_recorder(&recorder);

        let telemetry = FilterlistTelemetry::new(&MetricsBuilder::default());
        let mut filter = DogStatsDPrefixFilter {
            metric_prefix: "".to_string(),
            metric_prefix_blocklist: vec![],
            matcher: Blocklist::default(),
            effective_filterlist: EffectiveFilterlist::new(vec!["foo".to_string()], true, Vec::new(), false),
            telemetry,
            dynamic_handle: None,
        };

        filter.sync_effective_blocklist(false);
        filter.update_metric_filterlist(vec!["foo".to_string(), "foobar".to_string()]);

        assert_eq!(recorder.counter(METRIC_FILTERLIST_UPDATES_METRIC), Some(1));
        assert_eq!(recorder.gauge(METRIC_FILTERLIST_SIZE_METRIC), Some(2.0));

        let mut metric = Metric::gauge("foobar.baz", 1.0);
        assert!(!filter.process_metric(&mut metric));
    }

    #[test]
    fn telemetry_counts_listener_filtered_points() {
        let recorder = TestRecorder::default();
        let _local = set_default_local_recorder(&recorder);

        let telemetry = FilterlistTelemetry::new(&MetricsBuilder::default());
        let filter = DogStatsDPrefixFilter {
            metric_prefix: "".to_string(),
            metric_prefix_blocklist: vec![],
            matcher: Blocklist::new(["foo", "bar"], true),
            effective_filterlist: EffectiveFilterlist::default(),
            telemetry,
            dynamic_handle: None,
        };

        let mut exact_metric = Metric::gauge("foo", 1.0);
        assert!(!filter.process_metric(&mut exact_metric));

        let mut prefix_metric = Metric::gauge("bar.baz", 1.0);
        assert!(!filter.process_metric(&mut prefix_metric));

        assert_eq!(recorder.counter(LISTENER_FILTERED_POINTS_METRIC), Some(2));
    }
}

#[cfg(test)]
mod config_smoke {
    use datadog_agent_config_testing::config_registry::structs;
    use datadog_agent_config_testing::run_config_smoke_tests;
    use saluki_components::config::{DatadogRemapper, KEY_ALIASES};
    use serde_json::json;

    use super::DogStatsDPrefixFilterConfiguration;

    #[tokio::test]
    async fn smoke_test() {
        run_config_smoke_tests(
            structs::DOGSTATSD_PREFIX_FILTER_CONFIGURATION,
            &[],
            json!({}),
            |cfg| {
                cfg.as_typed::<DogStatsDPrefixFilterConfiguration>()
                    .expect("DogStatsDPrefixFilterConfiguration should deserialize")
            },
            KEY_ALIASES,
            DatadogRemapper::new,
        )
        .await
    }
}
