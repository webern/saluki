use std::path::{Path, PathBuf};

use datadog_agent_config_overlay_model::{Files, SchemaOverlay};

#[path = "build/schema_gen.rs"]
#[allow(dead_code)]
mod schema_gen;

#[path = "build/classifier_gen.rs"]
mod classifier_gen;

#[path = "build/doc_gen.rs"]
mod doc_gen;

fn main() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let schema_dir = manifest_dir.join("schema");
    let template_path = schema_dir.join("configuration.md.tmpl");

    let files = Files {
        schema: schema_dir.join("core_schema.yaml"),
        overlay: schema_dir.join("schema_overlay.yaml"),
    };

    let out_dir = PathBuf::from(std::env::var("OUT_DIR").unwrap());
    let docs_target = manifest_dir.join("../../../docs/agent-data-plane/configuration/dogstatsd.md");

    println!("cargo:rerun-if-changed={}", files.schema.display());
    println!("cargo:rerun-if-changed={}", files.overlay.display());
    println!("cargo:rerun-if-changed={}", template_path.display());
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=build/schema_gen.rs");
    println!("cargo:rerun-if-changed=build/classifier_gen.rs");
    println!("cargo:rerun-if-changed=build/doc_gen.rs");

    let schema_path = files.schema.clone();
    let overlay = SchemaOverlay::load(files).unwrap_or_else(|e| panic!("{e}"));
    let schema_map = schema_gen::load_schema(&schema_path);

    classifier_gen::generate(&overlay, &schema_map, &out_dir);
    doc_gen::generate(&overlay, &template_path, &out_dir);

    write_generated_doc(&out_dir, &docs_target);
}

fn write_generated_doc(out_dir: &Path, dst: &Path) {
    let src = out_dir.join("docs/configuration.md");
    let content = std::fs::read(&src)
        .unwrap_or_else(|e| panic!("cannot read {}: {}", src.display(), e));
    std::fs::write(dst, content)
        .unwrap_or_else(|e| panic!("cannot write {}: {}", dst.display(), e));
}
