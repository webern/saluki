//! Configuration loading, authority resolution, and translation for ADP.
//!
//! This crate is the adapter between source configuration models and the ADP-native model. It is
//! the only production crate that may touch `saluki_config::GenericConfiguration`, and the only
//! crate that depends on both the Datadog source model (`datadog-agent-config`) and the ADP-native
//! target model (`agent-data-plane-config`).

#![deny(missing_docs)]
#![deny(warnings)]

// Filled in by later build-order steps (skeleton + bootstrap, connection + stream, translation).
