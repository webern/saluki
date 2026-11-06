use std::future::Future;

use base64::{engine::general_purpose::STANDARD, Engine as _};
use panoramic_core::step::HandleError;
use serde::de::DeserializeOwned;
use serde_json::Value;
use stele::{ClientStatisticsAggregator, Event, Log, LogsRequest, Metric, ServiceCheck, Span};

/// Collected data from a test target.
///
/// Holds all telemetry data sent by the test target to the `datadog-intake` server spawned for the test run.
#[derive(Default)]
pub(crate) struct CollectedData {
    events: Vec<Event>,
    log_requests: Vec<LogsRequest>,
    metrics: Vec<Metric>,
    service_checks: Vec<ServiceCheck>,
    spans: Vec<Span>,
    trace_stats: ClientStatisticsAggregator,
    dogstatsd_forwarded_packets: Vec<String>,
}

/// A telemetry kind a participant's `dump` operation can return.
#[derive(Clone, Copy)]
pub(crate) enum DumpKind {
    Events,
    Logs,
    Metrics,
    ServiceChecks,
    Spans,
    TraceStats,
    DogstatsdForwarded,
}

impl DumpKind {
    /// Returns the kind's name in the `dump` operation's request and in error messages.
    pub(crate) const fn name(self) -> &'static str {
        match self {
            Self::Events => "events",
            Self::Logs => "logs",
            Self::Metrics => "metrics",
            Self::ServiceChecks => "service_checks",
            Self::Spans => "spans",
            Self::TraceStats => "trace_stats",
            Self::DogstatsdForwarded => "dogstatsd_forwarded",
        }
    }
}

/// Every telemetry kind collected from each participant, in the order the `dump` operation is called.
///
/// Collection and the comparison step's time budget both derive from this list.
pub(crate) const DUMP_KINDS: &[DumpKind] = &[
    DumpKind::Events,
    DumpKind::Logs,
    DumpKind::Metrics,
    DumpKind::ServiceChecks,
    DumpKind::Spans,
    DumpKind::TraceStats,
    DumpKind::DogstatsdForwarded,
];

impl CollectedData {
    /// Collects each telemetry kind in [`DUMP_KINDS`] once through a participant's `dump` operation.
    ///
    /// # Errors
    ///
    /// Preserves operation errors; an invalid telemetry shape is a product error.
    pub(crate) async fn collect_dumps<F, Fut>(dump: F) -> Result<Self, HandleError>
    where
        F: Fn(&'static str) -> Fut,
        Fut: Future<Output = Result<Value, HandleError>>,
    {
        let mut data = Self::default();
        for kind in DUMP_KINDS {
            data.record(*kind, dump(kind.name()).await?)?;
        }
        Ok(data)
    }

    /// Decodes a dumped value into the field holding its kind.
    fn record(&mut self, kind: DumpKind, value: Value) -> Result<(), HandleError> {
        let name = kind.name();
        match kind {
            DumpKind::Events => self.events = decode_dump(name, value)?,
            DumpKind::Logs => self.log_requests = decode_dump(name, value)?,
            DumpKind::Metrics => self.metrics = decode_dump(name, value)?,
            DumpKind::ServiceChecks => self.service_checks = decode_dump(name, value)?,
            DumpKind::Spans => self.spans = decode_dump(name, value)?,
            DumpKind::TraceStats => self.trace_stats = decode_dump(name, value)?,
            DumpKind::DogstatsdForwarded => {
                let packets: Vec<Vec<u8>> = decode_dump(name, value)?;
                self.dogstatsd_forwarded_packets = packets.into_iter().map(|packet| STANDARD.encode(packet)).collect();
            }
        }
        Ok(())
    }

    /// Returns a reference to the collected events.
    pub(crate) fn events(&self) -> &[Event] {
        &self.events
    }

    /// Returns a reference to the collected logs requests.
    pub(crate) fn log_requests(&self) -> &[LogsRequest] {
        &self.log_requests
    }

    /// Returns the collected logs from every request, in the order they were received.
    pub(crate) fn logs(&self) -> impl Iterator<Item = &Log> {
        self.log_requests.iter().flat_map(LogsRequest::logs)
    }

    /// Creates collected data holding only the given logs requests.
    #[cfg(test)]
    pub(crate) fn with_logs(log_requests: Vec<LogsRequest>) -> Self {
        Self {
            log_requests,
            ..Self::default()
        }
    }

    /// Returns a reference to the collected metrics.
    pub(crate) fn metrics(&self) -> &[Metric] {
        &self.metrics
    }

    /// Returns a reference to the collected service checks.
    pub(crate) fn service_checks(&self) -> &[ServiceCheck] {
        &self.service_checks
    }

    /// Returns a reference to the collected spans.
    pub(crate) fn spans(&self) -> &[Span] {
        &self.spans
    }

    /// Returns a reference to the collected trace statistics.
    pub(crate) fn trace_stats(&self) -> &ClientStatisticsAggregator {
        &self.trace_stats
    }

    /// Returns a reference to the forwarded DogStatsD packets captured by datadog-intake.
    pub(crate) fn dogstatsd_forwarded_packets(&self) -> &[String] {
        &self.dogstatsd_forwarded_packets
    }
}

fn decode_dump<T: DeserializeOwned>(kind: &str, value: Value) -> Result<T, HandleError> {
    serde_json::from_value(value).map_err(|error| HandleError::Product {
        message: format!("Failed to decode dumped {kind} from intake: {error}"),
    })
}
