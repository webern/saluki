use std::collections::HashMap;

use serde::de::value::{BorrowedBytesDeserializer, Error};
use serde::Deserialize;
use serde_json::json;

use crate::Json;

#[derive(Debug, Deserialize)]
struct ExampleConfiguration {
    #[serde(rename = "attributes.v1")]
    attributes: Json<AttributeMappings>,

    #[serde(rename = "metrics.v1")]
    metrics: Json<MetricMappings>,
}

#[derive(Debug, Deserialize)]
struct AttributeMappings {
    rename: HashMap<String, String>,
}

#[derive(Debug, Deserialize)]
struct MetricMappings {
    drop: Vec<String>,
}

/// Stands in for the map that `Payloads` presents: configuration ID to raw contents.
// TODO: drive these tests through the `Payloads` deserializer once it exists, which presents contents as bytes rather
// than as strings.
fn assigned(contents: &[(&str, serde_json::Value)]) -> serde_json::Value {
    contents
        .iter()
        .map(|(id, value)| ((*id).to_string(), json!(value.to_string())))
        .collect()
}

#[test]
fn deserializes_named_configurations() {
    let payloads = assigned(&[
        ("attributes.v1", json!({ "rename": { "http.host": "server.address" } })),
        ("metrics.v1", json!({ "drop": ["runtime.jvm.gc.count"] })),
    ]);

    let configuration: ExampleConfiguration = serde_json::from_value(payloads).expect("should deserialize");

    assert_eq!(
        Some(&"server.address".to_string()),
        configuration.attributes.0.rename.get("http.host")
    );
    assert_eq!(vec!["runtime.jvm.gc.count".to_string()], configuration.metrics.0.drop);
}

#[test]
fn deserializes_json_bytes() {
    let payload = BorrowedBytesDeserializer::<Error>::new(br#"{"rename":{"http.host":"server.address"}}"#);

    let attributes = Json::<AttributeMappings>::deserialize(payload).expect("should deserialize");

    assert_eq!(
        Some(&"server.address".to_string()),
        attributes.0.rename.get("http.host")
    );
}

#[test]
fn rejects_malformed_json_payload() {
    let payload = BorrowedBytesDeserializer::<Error>::new(b"{");

    let error = Json::<AttributeMappings>::deserialize(payload).expect_err("should reject malformed JSON");

    assert!(error.to_string().contains("EOF"));
}

#[test]
fn deserializes_unnamed_configurations() {
    let payloads = assigned(&[
        ("attributes.v1", json!({ "rename": {} })),
        ("attributes.v2", json!({ "rename": { "host": "host.name" } })),
    ]);

    let attributes: HashMap<String, Json<AttributeMappings>> =
        serde_json::from_value(payloads).expect("should deserialize");

    assert_eq!(2, attributes.len());
    assert_eq!(
        Some(&"host.name".to_string()),
        attributes["attributes.v2"].0.rename.get("host")
    );
}
