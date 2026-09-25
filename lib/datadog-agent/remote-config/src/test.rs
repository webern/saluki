//! Exercises the two product shapes a [`ProductDecoder`] has to serve.
//!
//! A product's configurations are either several instances of one schema under arbitrary IDs, or a known set of IDs each
//! with a schema of its own. These tests exercise both through the client's evaluation and subscription paths, and
//! exercise the client's subscription registry without starting the polling loop.

use std::collections::BTreeMap;
use std::sync::Arc;

use datadog_protos::remote_config::{ClientGetConfigsRequest, ClientGetConfigsResponse};
use serde::de::DeserializeOwned;
use serde::Deserialize;
use snafu::Snafu;

use crate::decoder::{evaluate, Outcome};
use crate::source::{FetchError, RcAgent};
use crate::{
    ApplyError, ConfigId, Error, ProductDecoder, ProductId, RcClientConfiguration, RemoteConfigurationClient,
    RemoteConfigurationWorker, TestPublisher,
};

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

impl ApplyError for SemanticCoreError {
    fn apply_error(&self) -> String {
        self.to_string()
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

    type Error = String;

    fn decode(&mut self, _id: &ConfigId, payload: &[u8]) -> Result<(), Self::Error> {
        self.chosen =
            Some(serde_json::from_slice(payload).map_err(|_| "Registry configuration is not valid JSON.".to_owned())?);

        Ok(())
    }

    fn build(self) -> Result<Self::Snapshot, Self::Error> {
        self.chosen
            .ok_or_else(|| "No registry configuration was assigned.".to_owned())
    }
}

#[test]
fn string_apply_error_preserves_message() {
    for message in ["Required configuration is missing.", "", "  details\nwith whitespace  "] {
        assert_eq!(message.to_owned().apply_error(), message);
    }
}

#[test]
fn converts_structured_error_to_apply_error() {
    let error = SemanticCoreError::MissingAttributes;

    assert_eq!(error.apply_error(), "No attribute mappings were assigned.");
}

#[tokio::test]
async fn decodes_configurations_of_differing_shapes() {
    let (publisher, mut subscription) = TestPublisher::<SemanticCore, SemanticCoreError>::new();
    assert!(subscription.current().is_none());

    publisher.assign::<SemanticCoreDecoder>([
        ("metrics.v1", br#"{"drop":["runtime.jvm.gc.count"]}"#.as_slice()),
        (
            "attributes.v1",
            br#"{"rename":{"http.host":"server.address"}}"#.as_slice(),
        ),
    ]);
    let current = subscription.current().expect("should publish");
    let snapshot = subscription.changed().await.expect("should build");
    assert!(std::sync::Arc::ptr_eq(&current, &snapshot));

    assert_eq!(
        Some(&"server.address".to_string()),
        snapshot.attributes.rename.get("http.host")
    );
    assert_eq!(
        vec!["runtime.jvm.gc.count".to_string()],
        snapshot.metrics.as_ref().expect("should decode metrics").drop
    );
    assert!(std::sync::Arc::ptr_eq(&snapshot, &subscription.current().unwrap()));
}

#[tokio::test]
async fn rejects_one_configuration_and_keeps_the_rest() {
    let (publisher, mut subscription) = TestPublisher::<SemanticCore, SemanticCoreError>::new();
    publisher.assign::<SemanticCoreDecoder>([
        (
            "attributes.v1",
            br#"{"rename":{"http.host":"server.address"}}"#.as_slice(),
        ),
        ("metrics.v1", b"{".as_slice()),
    ]);
    let snapshot = subscription
        .changed()
        .await
        .expect("should build without the malformed configuration");

    assert!(snapshot.attributes.rename.contains_key("http.host"));
    assert!(snapshot.metrics.is_none());

    let evaluated = evaluate::<SemanticCoreDecoder>(vec![
        (ConfigId::new("metrics.v1"), b"{"),
        (ConfigId::new("attributes.v1"), br#"{"rename":{}}"#),
    ]);
    assert!(matches!(evaluated.outcome, Outcome::Accepted(_)));
    assert_eq!(evaluated.verdicts[0].0.to_string(), "attributes.v1");
    assert!(evaluated.verdicts[0].1.is_none());
    assert_eq!(evaluated.verdicts[1].0.to_string(), "metrics.v1");
    assert!(evaluated.verdicts[1]
        .1
        .as_ref()
        .unwrap()
        .to_string()
        .contains("metrics.v1"));
}

#[tokio::test]
async fn rejects_a_snapshot_missing_a_required_configuration() {
    let (publisher, mut subscription) = TestPublisher::<SemanticCore, SemanticCoreError>::new();
    publisher.assign::<SemanticCoreDecoder>([("metrics.v1", br#"{"drop":[]}"#)]);

    assert!(
        matches!(subscription.changed().await, Err(error) if matches!(&*error, SemanticCoreError::MissingAttributes))
    );
    assert!(subscription.current().is_none());

    let evaluated = evaluate::<SemanticCoreDecoder>(vec![
        (ConfigId::new("metrics.v1"), br#"{"drop":[]}"#),
        (ConfigId::new("unknown.v1"), b"{}"),
    ]);
    assert!(matches!(
        evaluated.outcome,
        Outcome::Rejected(SemanticCoreError::MissingAttributes)
    ));
    assert_eq!(
        evaluated.verdicts[0].1.as_ref().unwrap().to_string(),
        "No attribute mappings were assigned."
    );
    assert_eq!(
        evaluated.verdicts[1].1.as_ref().unwrap().to_string(),
        "Configuration unknown.v1 is not one this product knows."
    );
}

#[tokio::test]
async fn rejects_an_empty_assignment_when_configuration_is_required() {
    let (publisher, mut subscription) = TestPublisher::<SemanticCore, SemanticCoreError>::new();
    publisher.assign::<SemanticCoreDecoder>([("attributes.v1", br#"{"rename":{}}"#)]);
    let accepted = subscription.changed().await.unwrap();

    publisher.assign::<SemanticCoreDecoder>(Vec::<(&str, &[u8])>::new());
    assert!(
        matches!(subscription.changed().await, Err(error) if matches!(&*error, SemanticCoreError::MissingAttributes))
    );
    assert!(std::sync::Arc::ptr_eq(&accepted, &subscription.current().unwrap()));

    let evaluated = evaluate::<SemanticCoreDecoder>(vec![]);
    assert!(matches!(
        evaluated.outcome,
        Outcome::Rejected(SemanticCoreError::MissingAttributes)
    ));
    assert!(evaluated.verdicts.is_empty());
}

#[tokio::test]
async fn reduces_configurations_of_one_shape_in_ascending_order() {
    let (publisher, mut subscription) = TestPublisher::<Registry>::new();
    publisher.assign::<LastValidRegistry>([
        ("registry.v3", br#"{"rename":{"host":"host.name"}}"#.as_slice()),
        ("registry.v1", br#"{"rename":{}}"#.as_slice()),
        ("registry.v2", b"{".as_slice()),
    ]);
    let snapshot = subscription
        .changed()
        .await
        .expect("should build from the valid configurations");

    assert_eq!(Some(&"host.name".to_string()), snapshot.rename.get("host"));
}

#[derive(Default)]
struct PanickingDecoder {
    panic_in_build: bool,
}

impl ProductDecoder for PanickingDecoder {
    type Snapshot = ();
    type Error = String;

    fn decode(&mut self, _id: &ConfigId, payload: &[u8]) -> Result<(), Self::Error> {
        match payload {
            b"decode" => panic!("decoder panicked"),
            b"build" => self.panic_in_build = true,
            _ => {}
        }
        Ok(())
    }

    fn build(self) -> Result<Self::Snapshot, Self::Error> {
        assert!(!self.panic_in_build, "build panicked");
        Ok(())
    }
}

#[tokio::test]
async fn panics_reject_every_configuration_without_publishing() {
    let (publisher, mut subscription) = TestPublisher::<()>::new();
    publisher.assign::<PanickingDecoder>(Vec::<(&str, &[u8])>::new());
    let accepted = subscription.changed().await.unwrap();

    for panic_at in ["decode", "build"] {
        let evaluated = evaluate::<PanickingDecoder>(vec![
            (ConfigId::new("first"), b"ok"),
            (ConfigId::new("second"), panic_at.as_bytes()),
        ]);
        assert!(matches!(evaluated.outcome, Outcome::Panicked));
        assert_eq!(evaluated.verdicts.len(), 2);
        assert!(evaluated
            .verdicts
            .iter()
            .all(|(_, reason)| reason.as_deref() == Some("Product decoder panicked.")));

        publisher.assign::<PanickingDecoder>([("first", "ok"), ("second", panic_at)]);
        assert!(
            tokio::time::timeout(std::time::Duration::from_millis(10), subscription.changed())
                .await
                .is_err()
        );
        assert!(std::sync::Arc::ptr_eq(&accepted, &subscription.current().unwrap()));
    }
}

#[tokio::test]
async fn clones_observe_latest_state_and_closed_subscriptions_wait() {
    let (publisher, mut subscription) = TestPublisher::<u32>::new();
    let mut clone = subscription.clone();
    publisher.accept(1);
    publisher.accept(2);
    assert_eq!(*subscription.changed().await.unwrap(), 2);
    assert_eq!(*clone.changed().await.unwrap(), 2);

    publisher.reject("invalid".to_owned());
    assert_eq!(subscription.changed().await.unwrap_err().to_string(), "invalid");
    assert_eq!(clone.changed().await.unwrap_err().to_string(), "invalid");
    assert_eq!(*subscription.current().unwrap(), 2);
    assert_eq!(*clone.current().unwrap(), 2);

    drop(publisher);
    assert!(
        tokio::time::timeout(std::time::Duration::from_millis(10), subscription.changed())
            .await
            .is_err()
    );
}

#[tokio::test]
async fn a_pending_publication_is_observed_after_the_publisher_stops() {
    let (publisher, mut subscription) = TestPublisher::<u32>::new();
    publisher.accept(3);
    drop(publisher);

    assert_eq!(*subscription.changed().await.unwrap(), 3);
    assert!(
        tokio::time::timeout(std::time::Duration::from_millis(10), subscription.changed())
            .await
            .is_err()
    );
}

#[test]
#[should_panic(expected = "duplicate configuration ID")]
fn test_publisher_rejects_duplicate_ids() {
    let (publisher, _) = TestPublisher::<Registry>::new();
    publisher.assign::<LastValidRegistry>([("same", b"{}"), ("same", b"{}")]);
}

#[test]
fn fetch_error_retains_unimplemented_status() {
    let error = FetchError::from(tonic::Status::unimplemented("Remote Configuration is disabled"));
    let FetchError::Unimplemented(cause) = error else {
        panic!("expected an unimplemented RPC");
    };

    let status = cause
        .downcast_ref::<tonic::Status>()
        .expect("original status is retained");
    assert_eq!(status.code(), tonic::Code::Unimplemented);
    assert_eq!(status.message(), "Remote Configuration is disabled");
}

#[test]
fn fetch_error_retains_other_rpc_statuses() {
    for code in [
        tonic::Code::Unavailable,
        tonic::Code::Unauthenticated,
        tonic::Code::InvalidArgument,
    ] {
        let error = FetchError::from(tonic::Status::new(code, "poll failed"));
        let FetchError::Rpc(cause) = error else {
            panic!("expected an RPC failure");
        };

        let status = cause
            .downcast_ref::<tonic::Status>()
            .expect("original status is retained");
        assert_eq!(status.code(), code);
        assert_eq!(status.message(), "poll failed");
    }
}

#[test]
fn settings_default_to_the_upstream_poll_schedule() {
    let config = crate::RcClientConfiguration::default();

    assert_eq!(std::time::Duration::from_secs(5), config.poll_interval);
    assert_eq!(std::time::Duration::from_secs(90), config.max_backoff);
}

#[test]
fn settings_reject_invalid_poll_intervals() {
    use std::time::Duration;

    for poll_interval in [Duration::ZERO, Duration::from_millis(999)] {
        let error = crate::RcClientConfiguration::new(poll_interval, Duration::from_secs(90)).unwrap_err();
        assert!(matches!(error, crate::Error::InvalidPollInterval { .. }));
        assert!(error.to_string().contains("poll_interval must be at least one second"));
    }
}

#[test]
fn settings_reject_max_backoff_below_poll_interval() {
    use std::time::Duration;

    let error = crate::RcClientConfiguration::new(Duration::from_secs(5), Duration::from_secs(4)).unwrap_err();
    assert!(matches!(error, crate::Error::InvalidMaxBackoff { .. }));
    assert!(error.to_string().contains("max_backoff must be at least poll_interval"));

    let settings = crate::RcClientConfiguration::new(Duration::from_secs(1), Duration::from_secs(1)).unwrap();
    assert_eq!(settings.poll_interval, settings.max_backoff);
}

/// Stands in for the Agent in tests that never start the polling loop.
struct NoAgent;

#[async_trait::async_trait]
impl RcAgent for NoAgent {
    async fn get_configs(&mut self, _request: ClientGetConfigsRequest) -> Result<ClientGetConfigsResponse, FetchError> {
        unreachable!("these tests never start the polling loop")
    }
}

fn client() -> (RemoteConfigurationClient, RemoteConfigurationWorker) {
    RemoteConfigurationClient::with_agent(Box::new(NoAgent), RcClientConfiguration::default())
}

fn registry_payload() -> Vec<(ConfigId, &'static [u8])> {
    vec![(ConfigId::new("registry.v1"), br#"{"rename":{"host":"host.name"}}"#)]
}

fn assert_already_subscribed<T>(result: crate::Result<T>, expected: &str) {
    match result {
        Err(Error::AlreadySubscribed { product }) => assert_eq!(product, expected),
        Err(error) => panic!("expected AlreadySubscribed, got {error}"),
        Ok(_) => panic!("expected AlreadySubscribed, got a subscription"),
    }
}

#[test]
fn a_product_variant_and_its_string_are_one_product() {
    let (client, _worker) = client();

    let _semantic_core = client
        .subscribe::<SemanticCoreDecoder>(ProductId::ApmSemanticCoreDd)
        .unwrap();
    assert_already_subscribed(
        client.subscribe::<SemanticCoreDecoder>("APM_SEMANTIC_CORE_DD"),
        "APM_SEMANTIC_CORE_DD",
    );

    let _sampling = client.subscribe::<LastValidRegistry>("APM_SAMPLING").unwrap();
    assert_already_subscribed(
        client.subscribe::<LastValidRegistry>(ProductId::ApmSampling),
        "APM_SAMPLING",
    );
}

#[test]
fn a_second_live_subscription_is_rejected_whatever_its_decoder() {
    let (client, _worker) = client();
    let subscription = client.subscribe::<LastValidRegistry>("FOO_MINE_DD").unwrap();

    assert_already_subscribed(client.subscribe::<LastValidRegistry>("FOO_MINE_DD"), "FOO_MINE_DD");
    assert_already_subscribed(client.subscribe::<PanickingDecoder>("FOO_MINE_DD"), "FOO_MINE_DD");
    assert_already_subscribed(
        client.clone().subscribe::<LastValidRegistry>("FOO_MINE_DD"),
        "FOO_MINE_DD",
    );

    // A surviving clone keeps the product subscribed.
    let clone = subscription.clone();
    drop(subscription);
    assert_already_subscribed(client.subscribe::<LastValidRegistry>("FOO_MINE_DD"), "FOO_MINE_DD");
    drop(clone);
    client.subscribe::<LastValidRegistry>("FOO_MINE_DD").unwrap();
}

#[tokio::test]
async fn resubscribing_starts_without_an_accepted_snapshot() {
    let (client, worker) = client();
    let mut first = client.subscribe::<LastValidRegistry>("FOO_MINE_DD").unwrap();
    worker.shared.assign("FOO_MINE_DD", registry_payload()).unwrap();
    assert!(first.changed().await.is_ok());
    drop(first);

    // The replacement may use a different decoder, and inherits nothing from the subscription it replaces.
    let mut second = client.subscribe::<PanickingDecoder>("FOO_MINE_DD").unwrap();
    assert!(second.current().is_none());
    assert!(
        tokio::time::timeout(std::time::Duration::from_millis(10), second.changed())
            .await
            .is_err()
    );

    let verdicts = worker
        .shared
        .assign("FOO_MINE_DD", vec![(ConfigId::new("first"), b"ok")])
        .unwrap();
    assert_eq!(verdicts.len(), 1);
    assert!(verdicts[0].1.is_none());
    assert!(second.changed().await.is_ok());
}

#[tokio::test]
async fn the_worker_delivers_into_subscriptions_made_by_any_client_handle() {
    let (client, worker) = client();
    assert!(Arc::ptr_eq(&client.shared, &worker.shared));

    let mut registry = client.clone().subscribe::<LastValidRegistry>("FOO_MINE_DD").unwrap();
    let verdicts = worker
        .shared
        .assign(
            "FOO_MINE_DD",
            vec![
                (ConfigId::new("registry.v2"), b"{"),
                (ConfigId::new("registry.v1"), br#"{"rename":{"host":"host.name"}}"#),
            ],
        )
        .unwrap();

    let snapshot = registry.changed().await.unwrap();
    assert_eq!(Some(&"host.name".to_string()), snapshot.rename.get("host"));
    assert_eq!(verdicts[0], (ConfigId::new("registry.v1"), None));
    assert_eq!(
        verdicts[1],
        (
            ConfigId::new("registry.v2"),
            Some("Registry configuration is not valid JSON.".to_owned())
        )
    );

    assert!(worker.shared.assign("APM_SAMPLING", registry_payload()).is_none());
}

#[test]
fn each_client_has_its_own_random_id() {
    let (first, _first_worker) = client();
    let (second, _second_worker) = client();

    let id = uuid::Uuid::parse_str(&first.shared.client_id).unwrap();
    assert_eq!(id.get_version(), Some(uuid::Version::Random));
    assert_ne!(first.shared.client_id, second.shared.client_id);
    assert_eq!(first.shared.client_id, first.clone().shared.client_id);
}

#[tokio::test]
async fn subscribing_wakes_the_worker_once() {
    let (client, worker) = client();
    let _registry = client.subscribe::<LastValidRegistry>("FOO_MINE_DD").unwrap();
    let _sampling = client.subscribe::<LastValidRegistry>("APM_SAMPLING").unwrap();

    worker.shared.wake.notified().await;
    assert!(
        tokio::time::timeout(std::time::Duration::from_millis(10), worker.shared.wake.notified())
            .await
            .is_err()
    );

    // A rejected subscribe does not wake the worker.
    assert_already_subscribed(client.subscribe::<LastValidRegistry>("FOO_MINE_DD"), "FOO_MINE_DD");
    assert!(
        tokio::time::timeout(std::time::Duration::from_millis(10), worker.shared.wake.notified())
            .await
            .is_err()
    );
}
