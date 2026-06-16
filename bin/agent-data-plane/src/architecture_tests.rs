use std::{fs, path::Path};

#[test]
fn internal_runtime_boundary_does_not_depend_on_source_config() {
    let internal_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("src/internal");
    let banned = ["GenericConfiguration", "try_get_typed(", "compat_datadog_source"];
    let mut offenders = Vec::new();

    collect_source_config_offenders(&internal_dir, &banned, &mut offenders);

    assert!(
        offenders.is_empty(),
        "ADP internal runtime boundary must not depend on source config:\n{}",
        offenders.join("\n")
    );
}

fn collect_source_config_offenders(path: &Path, banned: &[&str], offenders: &mut Vec<String>) {
    let entries = fs::read_dir(path).unwrap_or_else(|e| panic!("failed to read {}: {e}", path.display()));
    for entry in entries {
        let entry = entry.unwrap_or_else(|e| panic!("failed to read directory entry in {}: {e}", path.display()));
        let path = entry.path();
        if path.is_dir() {
            collect_source_config_offenders(&path, banned, offenders);
            continue;
        }

        if path.extension().and_then(|ext| ext.to_str()) != Some("rs") {
            continue;
        }

        let source = fs::read_to_string(&path).unwrap_or_else(|e| panic!("failed to read {}: {e}", path.display()));
        for (line_no, line) in source.lines().enumerate() {
            for needle in banned {
                if line.contains(needle) {
                    offenders.push(format!("{}:{} contains `{}`", path.display(), line_no + 1, needle));
                }
            }
        }
    }
}
