//! Checks domain. Carries the checks IPC endpoint; the checks metrics-encoding settings live in
//! `shared.metrics_encoding`.
// TODO: add the rest of the checks pipeline configuration as the checks pipeline is migrated.

use serde::Serialize;

use crate::control::ListenAddress;

/// Resolved checks configuration.
#[derive(Clone, Debug, Default, Serialize)]
pub struct Domain {
    /// Address the checks pipeline exposes for IPC with the core Agent. (not in Datadog Agent
    /// config schema)
    pub ipc_endpoint: ListenAddress,
}
