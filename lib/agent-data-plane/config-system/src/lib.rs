//! Configuration loading, authority resolution, and translation for ADP.
//!
//! This crate is the adapter between source configuration models and the ADP-native model. It is the
//! only production crate that should depend on both `datadog-agent-config` and
//! `agent-data-plane-config`.
//!
//! Responsibilities belong here when they involve source mechanics or lifecycle decisions: loading
//! bootstrap inputs, preserving current source precedence, separating primary and Saluki-private
//! sources, implementing source-language translators, translating into `SalukiConfiguration`, and
//! eventually replacing raw string-key dynamic updates with typed updates.
//!
//! Runtime components and topology assembly should receive typed outputs from this crate rather than
//! `saluki_config::GenericConfiguration`. No submitted configuration API should hide raw-map
//! compatibility behind names like `from_native`; any work-in-progress bridge belongs here and must
//! be removed before review.

pub mod bootstrap;
pub mod datadog_agent;
pub mod logging;
pub mod stream;
pub mod system;

pub use bootstrap::BootstrapInputs;
pub use datadog_agent::DatadogAgentConnection;
pub use logging::{DynamicLogLevelWorker, LoggingConfigurationTranslator};
pub use stream::ConfigStreamHandle;
pub use system::{ConfigurationSystem, StartedAttachments, StartedConfigurationSystem};
