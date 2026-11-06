//! Telemetry received by a `datadog-intake` server, shared by correctness and integration tests.
//!
//! This module fetches intake data. Deciding what the data must look like is left to each test kind.

mod collected;
#[cfg(test)]
pub(crate) use self::collected::DumpKind;
pub(crate) use self::collected::{CollectedData, DUMP_KINDS};

#[cfg(any(test, feature = "test-util"))]
pub mod test_server;
