//! Configuration system facade: translation of source configuration into the ADP-native model.
//!
//! This crate turns the two configuration sources into one `SalukiConfiguration`:
//!
//! - the typed Datadog source (`DatadogConfiguration`), whose supported keys the generated `drive`
//!   feeds to `SalukiConfigBuilder` (a `DatadogConfigWitness`) one key at a time, and
//! - the Saluki-schema-only source ([`SalukiOnly`]), whose values [`SalukiOnly::seed`] copies into
//!   the fields the Datadog schema does not cover.
//!
//! [`translate`] runs both writers and returns the assembled model. This is the only ADP production
//! crate that bridges the source configuration to the model; it constructs no components and does
//! not depend on `saluki-components`.

mod builder;
mod source;

use agent_data_plane_config::SalukiConfiguration;
use builder::SalukiConfigBuilder;
use datadog_agent_config::{drive, DatadogConfiguration, TranslateError};
pub use source::SalukiOnly;

/// Translates the Datadog and Saluki-only sources into one [`SalukiConfiguration`].
///
/// The Datadog `drive` feeds every supported key in `datadog` to a `SalukiConfigBuilder`;
/// `SalukiConfigBuilder::finish` then assembles the multi-key endpoint field. Finally
/// [`SalukiOnly::seed`] copies the Saluki-only values into their (disjoint) destinations.
///
/// # Errors
///
/// Returns the first [`TranslateError`] recorded while consuming a Datadog value (for example, an
/// enum or byte-size string that cannot be parsed). Seeding does not fail.
pub fn translate(
    datadog: &DatadogConfiguration, saluki_only: &SalukiOnly,
) -> Result<SalukiConfiguration, TranslateError> {
    let mut builder = SalukiConfigBuilder::default();
    drive(datadog, &mut builder)?;
    let mut config = builder.finish();
    saluki_only.seed(&mut config);
    Ok(config)
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use agent_data_plane_config::domains::dogstatsd::OriginTagCardinality;
    use datadog_agent_config::DatadogConfiguration;
    use serde_json::json;

    use super::{translate, SalukiOnly};

    #[test]
    fn translate_small_map_through_witness_and_seed() {
        // A small raw Datadog source map exercising a scalar conversion, an enum parse, a
        // seconds->Duration conversion, and the endpoint-assembly inputs.
        let datadog: DatadogConfiguration = serde_json::from_value(json!({
            "api_key": "abc",
            "dd_url": "https://custom.example.com",
            "dogstatsd_port": 9125,
            "dogstatsd_tag_cardinality": "high",
            "expected_tags_duration": 15.0,
        }))
        .expect("datadog source deserializes");

        // A small Saluki-only source setting one seeded field.
        let saluki_only: SalukiOnly = serde_json::from_value(json!({
            "dogstatsd": { "tcp_port": 8126 },
        }))
        .expect("saluki-only source deserializes");

        let config = translate(&datadog, &saluki_only).expect("translation succeeds");

        // Driven scalar conversion: i64 -> u16.
        assert_eq!(config.domains.dogstatsd.listeners.port, 9125);
        // Driven enum parse.
        assert_eq!(
            config.domains.dogstatsd.origin.tag_cardinality,
            OriginTagCardinality::High
        );
        // Driven seconds(f64) -> Duration.
        assert_eq!(config.shared.tags.expected_tags_duration, Duration::from_secs_f64(15.0));
        // Assembled multi-key field: the primary endpoint carries the api key and the resolved URL
        // (`dd_url` takes precedence over `site`).
        let primary = &config.shared.endpoints.endpoints[0];
        assert_eq!(primary.api_keys, vec!["abc".to_string()]);
        assert_eq!(primary.url, "https://custom.example.com");
        // Seeded Saluki-only field.
        assert_eq!(config.domains.dogstatsd.listeners.tcp_port, 8126);
    }
}
