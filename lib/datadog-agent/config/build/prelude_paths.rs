//! Single source of truth for the fully-qualified prelude paths that `typify` emits and both code
//! generators shorten to their in-scope names (`::std::option::Option` -> `Option`, etc.).
//!
//! `datadog_config_gen`'s `PathShortener` rewrites these paths in the generated data model, and
//! `witness_gen`'s `WitnessPathShortener` rewrites them in the leaf types it copies into the witness
//! trait. Sharing the rule here keeps the two from silently diverging: a path the data model
//! shortens but the witness does not would make a witness method type drift from the
//! `DatadogConfiguration` field it mirrors.

/// How a leading-`::` prelude path should be rewritten.
pub enum Shorten {
    /// Collapse the path to its final segment (`::std::option::Option` -> `Option`).
    Yes,
    /// Collapse the path to its final segment and flag that `HashMap` is now referenced unqualified,
    /// so the importing module brings `std::collections::HashMap` into scope.
    YesHashMap,
    /// Leave the path fully qualified.
    No,
}

/// Classify a leading-`::` path by its dotted segments (for example `["std", "option", "Option"]`).
///
/// Callers pass the path's segment idents only when it has a leading `::`; relative paths are never
/// shortened. The `TryFrom`/`TryInto`/`Default` entries appear only in the derive and `impl` paths
/// of the generated data model, never in a leaf field type, so adding them here does not change the
/// witness output -- it only keeps one authoritative list.
pub fn classify(segments: &[&str]) -> Shorten {
    match segments {
        ["std", "option", "Option"]
        | ["std", "string", "String"]
        | ["std", "vec", "Vec"]
        | ["std", "boxed", "Box"]
        | ["std", "default", "Default"]
        | ["std", "convert", "TryFrom"]
        | ["std", "convert", "TryInto"] => Shorten::Yes,
        ["std", "collections", "HashMap"] => Shorten::YesHashMap,
        _ => Shorten::No,
    }
}
