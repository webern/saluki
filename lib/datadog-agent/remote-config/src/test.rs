//! complicated tests requiring a mock type etc go here

use std::collections::HashMap;

use serde::Deserialize;
use serde_json::json;

use crate::Json;

/// A subscriber's type for `APM_SEMANTIC_CORE_DD`, naming the configuration IDs it expects.
// TODO: replace these placeholder configuration IDs with the ones the product actually assigns.
#[derive(Debug, Deserialize)]
struct SemanticCore {
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

    let semantic_core: SemanticCore = serde_json::from_value(payloads).expect("should deserialize");

    assert_eq!(
        Some(&"server.address".to_string()),
        semantic_core.attributes.0.rename.get("http.host")
    );
    assert_eq!(vec!["runtime.jvm.gc.count".to_string()], semantic_core.metrics.0.drop);
}

#[test]
fn reports_a_missing_configuration() {
    let payloads = assigned(&[("attributes.v1", json!({ "rename": {} }))]);

    let error = serde_json::from_value::<SemanticCore>(payloads).expect_err("should not deserialize");

    // This message is what the subscriber reports upstream against the configuration.
    assert!(error.to_string().contains("metrics.v1"));
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
