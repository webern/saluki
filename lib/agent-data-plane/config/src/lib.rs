//! ADP-native configuration model.
//!
//! This crate owns the target side of configuration translation: the typed configuration ADP wants
//! to run with after source-specific loading is complete. It should contain lifecycle and runtime
//! concepts such as `BootstrapConfiguration`, `SalukiPrivateConfiguration`, `SalukiConfiguration`,
//! and data-plane/component configuration bundles.
//!
//! This crate must not depend on `datadog-agent-config` or `saluki-config::GenericConfiguration`.
//! Datadog key names, schema defaults, environment aliases, and raw-map loading belong outside this
//! model. Keeping this crate source-agnostic lets Datadog, Saluki-private, OTel, or future inputs
//! translate into the same ADP-owned runtime shape.
