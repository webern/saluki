//! Datadog-specific configuration helpers.

pub mod mrf;

pub use datadog_agent_config::{DatadogRemapper, KEY_ALIASES};

pub use self::mrf::MrfConfiguration;
