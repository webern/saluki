use std::path::PathBuf;

use datadog_agent_config_overlay_model::{Files, SchemaOverlay};

#[path = "../config/build/schema_gen.rs"]
mod schema_gen;

#[path = "../config/build/registry_gen.rs"]
mod registry_gen;

fn main() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let config_manifest_dir = manifest_dir.parent().unwrap().join("config");
    let schema_dir = config_manifest_dir.join("schema");

    let files = Files {
        schema: schema_dir.join("core_schema.yaml"),
        overlay: schema_dir.join("schema_overlay.yaml"),
    };

    let src_dir = manifest_dir
        .parent().unwrap()  // config-testsupport -> datadog-agent
        .parent().unwrap()  // datadog-agent -> lib
        .join("saluki-components/src/config_registry/datadog");

    println!("cargo:rerun-if-changed={}", files.schema.display());
    println!("cargo:rerun-if-changed={}", files.overlay.display());
    println!("cargo:rerun-if-changed=build.rs");
    println!(
        "cargo:rerun-if-changed={}",
        config_manifest_dir.join("build/schema_gen.rs").display()
    );
    println!(
        "cargo:rerun-if-changed={}",
        config_manifest_dir.join("build/registry_gen.rs").display()
    );

    let out_dir = PathBuf::from(std::env::var("OUT_DIR").unwrap());

    let schema_path = files.schema.clone();
    let overlay = SchemaOverlay::load(files).unwrap_or_else(|e| panic!("{e}"));
    let schema_map = schema_gen::load_schema(&schema_path);

    // Write generated files into saluki-components source tree for PR diff visibility.
    registry_gen::generate_in_tree(&overlay, &schema_map, &src_dir);

    // Also generate to OUT_DIR for config-testsupport's own include!() compilation.
    registry_gen::generate(&overlay, &schema_map, &out_dir);
}
