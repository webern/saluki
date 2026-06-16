use std::{fs, path::Path};

#[test]
fn internal_runtime_boundary_does_not_depend_on_source_config() {
    let src_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let banned = ["GenericConfiguration", "try_get_typed(", "compat_datadog_source"];
    let mut offenders = Vec::new();

    collect_source_config_offenders(&src_dir.join("internal"), &banned, &mut offenders);

    assert!(
        offenders.is_empty(),
        "ADP internal runtime boundary must not depend on source config:\n{}",
        offenders.join("\n")
    );
}

#[test]
fn component_builders_do_not_depend_on_source_config() {
    let src_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let banned = [
        "GenericConfiguration",
        "try_get_typed(",
        "compat_datadog_source",
        "ConfigurationLoader",
        "ConfigUpdate",
        "saluki_config::",
    ];
    let mut offenders = Vec::new();

    collect_source_config_offenders(&src_dir.join("components"), &banned, &mut offenders);

    assert!(
        offenders.is_empty(),
        "ADP-local component builders must not depend on source config:\n{}",
        offenders.join("\n")
    );
}

#[test]
fn bootstrap_and_non_run_cli_do_not_depend_on_source_config() {
    let src_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let banned = [
        "GenericConfiguration",
        "try_get_typed(",
        "compat_datadog_source",
        "ConfigurationLoader",
        "DatadogRemapper",
        "KEY_ALIASES",
    ];
    let mut offenders = Vec::new();

    for path in [
        src_dir.join("main.rs"),
        src_dir.join("cli/config.rs"),
        src_dir.join("cli/debug"),
        src_dir.join("cli/dogstatsd.rs"),
        src_dir.join("cli/utils.rs"),
    ] {
        collect_source_config_offenders(&path, &banned, &mut offenders);
    }

    assert!(
        offenders.is_empty(),
        "ADP bootstrap and non-run CLI must not depend on source config:\n{}",
        offenders.join("\n")
    );
}

fn collect_source_config_offenders(path: &Path, banned: &[&str], offenders: &mut Vec<String>) {
    if path.is_dir() {
        let entries = fs::read_dir(path).unwrap_or_else(|e| panic!("failed to read {}: {e}", path.display()));
        for entry in entries {
            let entry = entry.unwrap_or_else(|e| panic!("failed to read directory entry in {}: {e}", path.display()));
            collect_source_config_offenders(&entry.path(), banned, offenders);
        }
        return;
    }

    if path.extension().and_then(|ext| ext.to_str()) != Some("rs") {
        return;
    }

    let source = fs::read_to_string(path).unwrap_or_else(|e| panic!("failed to read {}: {e}", path.display()));
    for (line_no, line) in source.lines().enumerate() {
        for needle in banned {
            if line.contains(needle) {
                offenders.push(format!("{}:{} contains `{}`", path.display(), line_no + 1, needle));
            }
        }
    }
}
