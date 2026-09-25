//! Exercises the two product shapes a [`ProductDecoder`] has to serve.
//!
//! A product's configurations are either several instances of one schema under arbitrary IDs, or a known set of IDs each
//! with a schema of its own. These tests implement one decoder of each kind and drive them the way the client will, so
//! that a change to the trait has to keep both shapes expressible.

use std::collections::BTreeMap;

use serde::de::DeserializeOwned;
use serde::Deserialize;
use snafu::Snafu;

use crate::{ApplyError, AsApplyError, ConfigId, ProductDecoder};

/// Stands in for the client's decoding loop: a fresh decoder, one `decode` per assigned configuration in ascending ID
/// order, then `build`. Returns the configurations the decoder rejected alongside the outcome of `build`.
///
/// Callers pass distinct IDs because the client guarantees that: several assigned files collapsing to one configuration
/// ID are rejected as a set before any of them reaches a decoder.
fn drive<P: ProductDecoder>(assigned: &[(&str, &[u8])]) -> (Vec<String>, Result<P::Snapshot, P::Error>) {
    let mut sorted = assigned.to_vec();
    sorted.sort_by_key(|(id, _)| *id);

    let mut decoder = P::default();
    let mut rejected = Vec::new();
    for (id, payload) in sorted {
        let id = ConfigId(id.to_string());
        if decoder.decode(&id, payload).is_err() {
            rejected.push(id.to_string());
        }
    }

    (rejected, decoder.build())
}

// A product whose configuration IDs are known in advance and each carry a different schema.

#[derive(Debug, Deserialize)]
struct AttributeMappings {
    rename: BTreeMap<String, String>,
}

#[derive(Debug, Deserialize)]
struct MetricMappings {
    drop: Vec<String>,
}

/// The assembled snapshot: attribute mappings are required, metric mappings are not.
struct SemanticCore {
    attributes: AttributeMappings,
    metrics: Option<MetricMappings>,
}

#[derive(Debug, Snafu)]
enum SemanticCoreError {
    #[snafu(display("Configuration {id} is not one this product knows."))]
    UnknownConfiguration { id: String },

    #[snafu(display("Configuration {id} is not valid JSON: {source}"))]
    MalformedConfiguration { id: String, source: serde_json::Error },

    #[snafu(display("No attribute mappings were assigned."))]
    MissingAttributes,
}

impl AsApplyError for SemanticCoreError {
    fn as_apply_error(&self) -> ApplyError {
        ApplyError::new(self.to_string())
    }
}

#[derive(Default)]
struct SemanticCoreDecoder {
    attributes: Option<AttributeMappings>,
    metrics: Option<MetricMappings>,
}

impl ProductDecoder for SemanticCoreDecoder {
    type Snapshot = SemanticCore;

    type Error = SemanticCoreError;

    fn decode(&mut self, id: &ConfigId, payload: &[u8]) -> Result<(), Self::Error> {
        match &**id {
            "attributes.v1" => self.attributes = Some(from_json(id, payload)?),
            "metrics.v1" => self.metrics = Some(from_json(id, payload)?),
            _ => return Err(SemanticCoreError::UnknownConfiguration { id: id.to_string() }),
        }

        Ok(())
    }

    fn build(self) -> Result<Self::Snapshot, Self::Error> {
        let attributes = self.attributes.ok_or(SemanticCoreError::MissingAttributes)?;

        Ok(SemanticCore {
            attributes,
            metrics: self.metrics,
        })
    }
}

fn from_json<T: DeserializeOwned>(id: &ConfigId, payload: &[u8]) -> Result<T, SemanticCoreError> {
    serde_json::from_slice(payload).map_err(|source| SemanticCoreError::MalformedConfiguration {
        id: id.to_string(),
        source,
    })
}

// A product assigned any number of configurations of one schema, keeping the last valid one.

#[derive(Debug, Deserialize)]
struct Registry {
    rename: BTreeMap<String, String>,
}

#[derive(Default)]
struct LastValidRegistry {
    chosen: Option<Registry>,
}

impl ProductDecoder for LastValidRegistry {
    type Snapshot = Registry;

    type Error = ApplyError;

    fn decode(&mut self, _id: &ConfigId, payload: &[u8]) -> Result<(), Self::Error> {
        self.chosen = Some(
            serde_json::from_slice(payload)
                .map_err(|_| ApplyError::new("Registry configuration is not valid JSON."))?,
        );

        Ok(())
    }

    fn build(self) -> Result<Self::Snapshot, Self::Error> {
        self.chosen
            .ok_or_else(|| ApplyError::new("No registry configuration was assigned."))
    }
}

#[test]
fn apply_error_preserves_message() {
    for message in ["Required configuration is missing.", "", "  details\nwith whitespace  "] {
        let error = ApplyError::new(message);
        let owned_error = ApplyError::new(message.to_string());

        assert_eq!(error.to_string(), message);
        assert_eq!(owned_error.to_string(), message);
        assert_eq!(error.clone().to_string(), message);
        assert_eq!(error.as_apply_error().to_string(), message);
        assert!(std::error::Error::source(&error).is_none());
    }
}

#[test]
fn converts_structured_error_to_apply_error() {
    let error = SemanticCoreError::MissingAttributes;

    assert_eq!(
        error.as_apply_error().to_string(),
        "No attribute mappings were assigned."
    );
}

#[test]
fn decodes_configurations_of_differing_shapes() {
    let (rejected, snapshot) = drive::<SemanticCoreDecoder>(&[
        ("attributes.v1", br#"{"rename":{"http.host":"server.address"}}"#),
        ("metrics.v1", br#"{"drop":["runtime.jvm.gc.count"]}"#),
    ]);
    let snapshot = snapshot.expect("should build");

    assert!(rejected.is_empty());
    assert_eq!(
        Some(&"server.address".to_string()),
        snapshot.attributes.rename.get("http.host")
    );
    assert_eq!(
        vec!["runtime.jvm.gc.count".to_string()],
        snapshot.metrics.expect("should decode metrics").drop
    );
}

#[test]
fn rejects_one_configuration_and_keeps_the_rest() {
    let (rejected, snapshot) = drive::<SemanticCoreDecoder>(&[
        ("attributes.v1", br#"{"rename":{"http.host":"server.address"}}"#),
        ("metrics.v1", b"{"),
    ]);
    let snapshot = snapshot.expect("should build without the malformed configuration");

    assert_eq!(vec!["metrics.v1".to_string()], rejected);
    assert!(snapshot.attributes.rename.contains_key("http.host"));
    assert!(snapshot.metrics.is_none());
}

#[test]
fn rejects_a_snapshot_missing_a_required_configuration() {
    let (rejected, snapshot) = drive::<SemanticCoreDecoder>(&[("metrics.v1", br#"{"drop":[]}"#)]);

    assert!(rejected.is_empty());
    assert!(matches!(snapshot, Err(SemanticCoreError::MissingAttributes)));
}

#[test]
fn rejects_an_empty_assignment_when_configuration_is_required() {
    let (rejected, snapshot) = drive::<SemanticCoreDecoder>(&[]);

    assert!(rejected.is_empty());
    assert!(matches!(snapshot, Err(SemanticCoreError::MissingAttributes)));
}

#[test]
fn reduces_configurations_of_one_shape_in_ascending_order() {
    let (rejected, snapshot) = drive::<LastValidRegistry>(&[
        ("registry.v3", br#"{"rename":{"host":"host.name"}}"#),
        ("registry.v1", br#"{"rename":{}}"#),
        ("registry.v2", b"{"),
    ]);
    let snapshot = snapshot.expect("should build from the valid configurations");

    assert_eq!(vec!["registry.v2".to_string()], rejected);
    assert_eq!(Some(&"host.name".to_string()), snapshot.rename.get("host"));
}

#[test]
fn settings_default_to_the_upstream_poll_schedule() {
    let config = crate::RcClientConfiguration::default();

    assert_eq!(std::time::Duration::from_secs(5), config.poll_interval);
    assert_eq!(std::time::Duration::from_secs(90), config.max_backoff);
}
