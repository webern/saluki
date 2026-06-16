//! Configuration loading, authority resolution, and translation for ADP.
//!
//! This crate is the adapter between source configuration models and the ADP-native model. It is
//! the only production crate that may touch `saluki_config::GenericConfiguration`, and the only
//! crate that depends on both the Datadog source model (`datadog-agent-config`) and the ADP-native
//! target model (`agent-data-plane-config`).
//!
//! Runtime components and topology assembly receive typed outputs from this crate
//! ([`SalukiConfiguration`](agent_data_plane_config::SalukiConfiguration)) rather than a raw map. No
//! submitted configuration API hides raw-map compatibility behind names like `from_native`.

#![deny(missing_docs)]
#![deny(warnings)]

pub mod bootstrap;
pub mod datadog_agent;
pub mod dynamic;
pub mod stream;
pub mod system;
pub mod translate;
mod validate;

pub use bootstrap::BootstrapInputs;
pub use datadog_agent::DatadogAgentConnection;
pub use dynamic::{ConfigUpdateRouter, DynamicConfigHandles};
pub use saluki_component_config::ScopedConfigHandle;
pub use stream::ConfigStreamHandle;
pub use system::{
    translate_from_generic, ConfigurationSystem, StartedAttachments, StartedConfigurationSystem, StartedParts,
};
pub use translate::{translate_datadog, PipelineGates, Translator};
