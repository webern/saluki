use std::fmt::Write as _;
use std::path::Path;

use datadog_agent_config_overlay_model::{
    Pipeline, PipelineAffinity, SchemaOverlay, Severity, SupportLevel, ValueType,
};
use indexmap::IndexMap;

use super::schema_gen::{self, FieldInfo};

// These keys are not in the Datadog Agent schema; they exist only in ADP.
// When config_registry is deleted, delete this table too.
struct SalukiEntry {
    yaml_path: &'static str,
    value_type: &'static str,
    schema_default: Option<&'static str>,
    env_vars: &'static [&'static str],
    env_var_override: Option<&'static [&'static str]>,
    additional_yaml_paths: &'static [&'static str],
    used_by: &'static [&'static str],
    test_json: Option<&'static str>,
    pipeline_affinity: &'static str,
    filename: &'static str,
}

static SALUKI_ENTRIES: &[SalukiEntry] = &[
    // ── dogstatsd.rs ─────────────────────────────────────────────────────────
    SalukiEntry {
        yaml_path: "dogstatsd_allow_context_heap_allocs",
        value_type: "ValueType::Bool",
        schema_default: Some("true"),
        env_vars: &[],
        env_var_override: None,
        additional_yaml_paths: &[],
        used_by: &["DOGSTATSD_CONFIGURATION"],
        test_json: None,
        pipeline_affinity: "PipelineAffinity::Pipelines(&[Pipeline::DogStatsD])",
        filename: "dogstatsd.rs",
    },
    SalukiEntry {
        yaml_path: "dogstatsd_autoscale_udp_listeners",
        value_type: "ValueType::Bool",
        schema_default: Some("false"),
        env_vars: &[],
        env_var_override: None,
        additional_yaml_paths: &[],
        used_by: &["DOGSTATSD_CONFIGURATION"],
        test_json: None,
        pipeline_affinity: "PipelineAffinity::Pipelines(&[Pipeline::DogStatsD])",
        filename: "dogstatsd.rs",
    },
    SalukiEntry {
        yaml_path: "dogstatsd_buffer_count",
        value_type: "ValueType::Integer",
        schema_default: None,
        env_vars: &[],
        env_var_override: None,
        additional_yaml_paths: &[],
        used_by: &["DOGSTATSD_CONFIGURATION"],
        test_json: None,
        pipeline_affinity: "PipelineAffinity::Pipelines(&[Pipeline::DogStatsD])",
        filename: "dogstatsd.rs",
    },
    SalukiEntry {
        yaml_path: "dogstatsd_cached_contexts_limit",
        value_type: "ValueType::Integer",
        schema_default: None,
        env_vars: &[],
        env_var_override: None,
        additional_yaml_paths: &[],
        used_by: &["DOGSTATSD_CONFIGURATION"],
        test_json: None,
        pipeline_affinity: "PipelineAffinity::Pipelines(&[Pipeline::DogStatsD])",
        filename: "dogstatsd.rs",
    },
    SalukiEntry {
        yaml_path: "dogstatsd_cached_tagsets_limit",
        value_type: "ValueType::Integer",
        schema_default: None,
        env_vars: &[],
        env_var_override: None,
        additional_yaml_paths: &[],
        used_by: &["DOGSTATSD_CONFIGURATION"],
        test_json: None,
        pipeline_affinity: "PipelineAffinity::Pipelines(&[Pipeline::DogStatsD])",
        filename: "dogstatsd.rs",
    },
    SalukiEntry {
        yaml_path: "dogstatsd_minimum_sample_rate",
        value_type: "ValueType::Float",
        schema_default: None,
        env_vars: &[],
        env_var_override: None,
        additional_yaml_paths: &[],
        used_by: &["DOGSTATSD_CONFIGURATION"],
        test_json: None,
        pipeline_affinity: "PipelineAffinity::Pipelines(&[Pipeline::DogStatsD])",
        filename: "dogstatsd.rs",
    },
    SalukiEntry {
        yaml_path: "dogstatsd_permissive_decoding",
        value_type: "ValueType::Bool",
        schema_default: Some("true"),
        env_vars: &[],
        env_var_override: None,
        additional_yaml_paths: &[],
        used_by: &["DOGSTATSD_CONFIGURATION"],
        test_json: None,
        pipeline_affinity: "PipelineAffinity::Pipelines(&[Pipeline::DogStatsD])",
        filename: "dogstatsd.rs",
    },
    SalukiEntry {
        yaml_path: "dogstatsd_string_interner_size_bytes",
        value_type: "ValueType::Integer",
        schema_default: None,
        env_vars: &[],
        env_var_override: None,
        additional_yaml_paths: &[],
        used_by: &["DOGSTATSD_CONFIGURATION"],
        test_json: None,
        pipeline_affinity: "PipelineAffinity::Pipelines(&[Pipeline::DogStatsD])",
        filename: "dogstatsd.rs",
    },
    SalukiEntry {
        yaml_path: "dogstatsd_tcp_port",
        value_type: "ValueType::Integer",
        schema_default: None,
        env_vars: &[],
        env_var_override: None,
        additional_yaml_paths: &[],
        used_by: &["DOGSTATSD_CONFIGURATION"],
        test_json: None,
        pipeline_affinity: "PipelineAffinity::Pipelines(&[Pipeline::DogStatsD])",
        filename: "dogstatsd.rs",
    },
    // ── otlp.rs ──────────────────────────────────────────────────────────────
    SalukiEntry {
        yaml_path: "otlp_config.traces.enable_otlp_compute_top_level_by_span_kind",
        value_type: "ValueType::Bool",
        schema_default: Some("true"),
        env_vars: &[],
        env_var_override: None,
        additional_yaml_paths: &[],
        used_by: &["OTLP_DECODER_CONFIGURATION", "OTLP_CONFIGURATION"],
        test_json: None,
        pipeline_affinity: "PipelineAffinity::Pipelines(&[Pipeline::Traces])",
        filename: "otlp.rs",
    },
    SalukiEntry {
        yaml_path: "otlp_config.traces.ignore_missing_datadog_fields",
        value_type: "ValueType::Bool",
        schema_default: None,
        env_vars: &[],
        env_var_override: None,
        additional_yaml_paths: &[],
        used_by: &["OTLP_DECODER_CONFIGURATION", "OTLP_CONFIGURATION"],
        test_json: None,
        pipeline_affinity: "PipelineAffinity::Pipelines(&[Pipeline::Traces])",
        filename: "otlp.rs",
    },
    SalukiEntry {
        yaml_path: "otlp_config.traces.string_interner_size",
        value_type: "ValueType::Integer",
        schema_default: None,
        env_vars: &[],
        env_var_override: None,
        additional_yaml_paths: &[],
        used_by: &["OTLP_DECODER_CONFIGURATION", "OTLP_CONFIGURATION"],
        test_json: None,
        pipeline_affinity: "PipelineAffinity::Pipelines(&[Pipeline::Traces])",
        filename: "otlp.rs",
    },
    SalukiEntry {
        yaml_path: "otlp_config.receiver.protocols.http.transport",
        value_type: "ValueType::String",
        schema_default: None,
        env_vars: &[],
        env_var_override: None,
        additional_yaml_paths: &[],
        used_by: &["OTLP_RELAY_CONFIGURATION", "OTLP_CONFIGURATION"],
        test_json: None,
        pipeline_affinity: "PipelineAffinity::Pipelines(&[Pipeline::Otlp])",
        filename: "otlp.rs",
    },
    SalukiEntry {
        yaml_path: "otlp_allow_context_heap_allocs",
        value_type: "ValueType::Bool",
        schema_default: Some("true"),
        env_vars: &[],
        env_var_override: None,
        additional_yaml_paths: &[],
        used_by: &["OTLP_CONFIGURATION"],
        test_json: None,
        pipeline_affinity: "PipelineAffinity::Pipelines(&[Pipeline::Otlp])",
        filename: "otlp.rs",
    },
    SalukiEntry {
        yaml_path: "otlp_cached_contexts_limit",
        value_type: "ValueType::Integer",
        schema_default: None,
        env_vars: &[],
        env_var_override: None,
        additional_yaml_paths: &[],
        used_by: &["OTLP_CONFIGURATION"],
        test_json: None,
        pipeline_affinity: "PipelineAffinity::Pipelines(&[Pipeline::Otlp])",
        filename: "otlp.rs",
    },
    SalukiEntry {
        yaml_path: "otlp_cached_tagsets_limit",
        value_type: "ValueType::Integer",
        schema_default: None,
        env_vars: &[],
        env_var_override: None,
        additional_yaml_paths: &[],
        used_by: &["OTLP_CONFIGURATION"],
        test_json: None,
        pipeline_affinity: "PipelineAffinity::Pipelines(&[Pipeline::Otlp])",
        filename: "otlp.rs",
    },
    SalukiEntry {
        yaml_path: "otlp_string_interner_size",
        value_type: "ValueType::Integer",
        schema_default: None,
        env_vars: &[],
        env_var_override: None,
        additional_yaml_paths: &[],
        used_by: &["OTLP_CONFIGURATION"],
        test_json: None,
        pipeline_affinity: "PipelineAffinity::Pipelines(&[Pipeline::Otlp])",
        filename: "otlp.rs",
    },
    // ── aggregate.rs ─────────────────────────────────────────────────────────
    SalukiEntry {
        yaml_path: "aggregate_window_duration_seconds",
        value_type: "ValueType::Integer",
        schema_default: None,
        env_vars: &[],
        env_var_override: None,
        additional_yaml_paths: &[],
        used_by: &["AGGREGATE_CONFIGURATION"],
        test_json: None,
        pipeline_affinity: "PipelineAffinity::Pipelines(&[Pipeline::DogStatsD, Pipeline::Checks])",
        filename: "aggregate.rs",
    },
    SalukiEntry {
        yaml_path: "aggregate_flush_interval",
        value_type: "ValueType::String",
        schema_default: None,
        env_vars: &[],
        env_var_override: None,
        additional_yaml_paths: &[],
        used_by: &["AGGREGATE_CONFIGURATION"],
        test_json: Some(r#"{"secs": 42, "nanos": 0}"#),
        pipeline_affinity: "PipelineAffinity::Pipelines(&[Pipeline::DogStatsD, Pipeline::Checks])",
        filename: "aggregate.rs",
    },
    SalukiEntry {
        yaml_path: "aggregate_context_limit",
        value_type: "ValueType::Integer",
        schema_default: None,
        env_vars: &[],
        env_var_override: None,
        additional_yaml_paths: &[],
        used_by: &["AGGREGATE_CONFIGURATION"],
        test_json: None,
        pipeline_affinity: "PipelineAffinity::Pipelines(&[Pipeline::DogStatsD, Pipeline::Checks])",
        filename: "aggregate.rs",
    },
    SalukiEntry {
        yaml_path: "counter_expiry_seconds",
        value_type: "ValueType::Integer",
        schema_default: None,
        env_vars: &[],
        env_var_override: None,
        additional_yaml_paths: &["dogstatsd_expiry_seconds"],
        used_by: &["AGGREGATE_CONFIGURATION"],
        test_json: None,
        pipeline_affinity: "PipelineAffinity::Pipelines(&[Pipeline::DogStatsD, Pipeline::Checks])",
        filename: "aggregate.rs",
    },
    SalukiEntry {
        yaml_path: "aggregate_passthrough_idle_flush_timeout",
        value_type: "ValueType::String",
        schema_default: None,
        env_vars: &[],
        env_var_override: None,
        additional_yaml_paths: &[],
        used_by: &["AGGREGATE_CONFIGURATION"],
        test_json: Some(r#"{"secs": 42, "nanos": 0}"#),
        pipeline_affinity: "PipelineAffinity::Pipelines(&[Pipeline::DogStatsD, Pipeline::Checks])",
        filename: "aggregate.rs",
    },
    // ── trace_obfuscation.rs ─────────────────────────────────────────────────
    SalukiEntry {
        yaml_path: "apm_config.obfuscation.sql.dbms",
        value_type: "ValueType::String",
        schema_default: None,
        env_vars: &[],
        env_var_override: None,
        additional_yaml_paths: &[],
        used_by: &["TRACE_OBFUSCATION_CONFIGURATION"],
        test_json: None,
        pipeline_affinity: "PipelineAffinity::Pipelines(&[Pipeline::Traces])",
        filename: "trace_obfuscation.rs",
    },
    SalukiEntry {
        yaml_path: "apm_config.obfuscation.sql.dollar_quoted_func",
        value_type: "ValueType::Bool",
        schema_default: None,
        env_vars: &[],
        env_var_override: None,
        additional_yaml_paths: &[],
        used_by: &["TRACE_OBFUSCATION_CONFIGURATION"],
        test_json: None,
        pipeline_affinity: "PipelineAffinity::Pipelines(&[Pipeline::Traces])",
        filename: "trace_obfuscation.rs",
    },
    SalukiEntry {
        yaml_path: "apm_config.obfuscation.sql.keep_sql_alias",
        value_type: "ValueType::Bool",
        schema_default: None,
        env_vars: &[],
        env_var_override: None,
        additional_yaml_paths: &[],
        used_by: &["TRACE_OBFUSCATION_CONFIGURATION"],
        test_json: None,
        pipeline_affinity: "PipelineAffinity::Pipelines(&[Pipeline::Traces])",
        filename: "trace_obfuscation.rs",
    },
    SalukiEntry {
        yaml_path: "apm_config.obfuscation.sql.replace_digits",
        value_type: "ValueType::Bool",
        schema_default: None,
        env_vars: &[],
        env_var_override: None,
        additional_yaml_paths: &[],
        used_by: &["TRACE_OBFUSCATION_CONFIGURATION"],
        test_json: None,
        pipeline_affinity: "PipelineAffinity::Pipelines(&[Pipeline::Traces])",
        filename: "trace_obfuscation.rs",
    },
    SalukiEntry {
        yaml_path: "apm_config.obfuscation.sql.table_names",
        value_type: "ValueType::Bool",
        schema_default: None,
        env_vars: &[],
        env_var_override: None,
        additional_yaml_paths: &[],
        used_by: &["TRACE_OBFUSCATION_CONFIGURATION"],
        test_json: None,
        pipeline_affinity: "PipelineAffinity::Pipelines(&[Pipeline::Traces])",
        filename: "trace_obfuscation.rs",
    },
    // ── encoders.rs ──────────────────────────────────────────────────────────
    SalukiEntry {
        yaml_path: "flush_timeout_secs",
        value_type: "ValueType::Integer",
        schema_default: None,
        env_vars: &[],
        env_var_override: None,
        additional_yaml_paths: &[],
        used_by: &[
            "DATADOG_APM_STATS_ENCODER_CONFIGURATION",
            "DATADOG_METRICS_CONFIGURATION",
            "DATADOG_TRACE_CONFIGURATION",
        ],
        test_json: None,
        pipeline_affinity: "PipelineAffinity::Pipelines(&[Pipeline::Checks, Pipeline::Traces, Pipeline::DogStatsD])",
        filename: "encoders.rs",
    },
    SalukiEntry {
        yaml_path: "serializer_max_metrics_per_payload",
        value_type: "ValueType::Integer",
        schema_default: None,
        env_vars: &[],
        env_var_override: None,
        additional_yaml_paths: &[],
        used_by: &["DATADOG_METRICS_CONFIGURATION"],
        test_json: None,
        pipeline_affinity: "PipelineAffinity::Pipelines(&[Pipeline::Checks, Pipeline::DogStatsD])",
        filename: "encoders.rs",
    },
    // ── dogstatsd_mapper.rs ──────────────────────────────────────────────────
    SalukiEntry {
        yaml_path: "dogstatsd_mapper_string_interner_size",
        value_type: "ValueType::Integer",
        schema_default: None,
        env_vars: &[],
        env_var_override: None,
        additional_yaml_paths: &[],
        used_by: &["DOGSTATSD_MAPPER_CONFIGURATION"],
        test_json: None,
        pipeline_affinity: "PipelineAffinity::Pipelines(&[Pipeline::DogStatsD])",
        filename: "dogstatsd_mapper.rs",
    },
];

#[allow(dead_code)]
pub fn generate(overlay: &SchemaOverlay, schema_map: &IndexMap<String, FieldInfo>, out_dir: &Path) {
    let registry_dir = out_dir.join("config_registry");
    std::fs::create_dir_all(&registry_dir).unwrap();

    validate_saluki_entries(overlay);
    generate_schema_rs(overlay, schema_map, &registry_dir, "");
    generate_subsystem_files(overlay, schema_map, &registry_dir, "");
    generate_unsupported_rs(overlay, &registry_dir, "");
    generate_mod_rs(overlay, &registry_dir);
}

/// Write generated registry files directly into the source tree for PR diff visibility.
///
/// Each subsystem file gets `#[allow(unused_imports)]\nuse super::*;\n\n` prepended so
/// it compiles as a file-based Rust module. The `mod.rs` uses plain `mod` declarations
/// instead of `include!(OUT_DIR)` so Cargo resolves them from the source tree.
pub fn generate_in_tree(overlay: &SchemaOverlay, schema_map: &IndexMap<String, FieldInfo>, src_dir: &Path) {
    std::fs::create_dir_all(src_dir).unwrap();

    validate_saluki_entries(overlay);

    let preamble = "#[allow(unused_imports)]\nuse super::*;\n\n";
    generate_schema_rs(overlay, schema_map, src_dir, preamble);
    generate_subsystem_files(overlay, schema_map, src_dir, preamble);
    generate_unsupported_rs(overlay, src_dir, preamble);
    generate_mod_rs_in_tree(overlay, src_dir);
}

fn validate_saluki_entries(overlay: &SchemaOverlay) {
    for entry in SALUKI_ENTRIES {
        if overlay.supported.contains_key(entry.yaml_path)
            || overlay.unsupported.contains_key(entry.yaml_path)
            || overlay.ignored.contains_key(entry.yaml_path)
        {
            panic!(
                "Saluki entry '{}' collides with a vendored schema key in the overlay — \
                 it should use the schema entry instead of a hard-coded SchemaEntry",
                entry.yaml_path
            );
        }
    }
}

fn generate_schema_rs(_overlay: &SchemaOverlay, schema_map: &IndexMap<String, FieldInfo>, dir: &Path, preamble: &str) {
    let mut out = String::new();
    writeln!(
        out,
        "// @generated by build.rs from core_schema.yaml + schema_overlay.yaml — DO NOT EDIT"
    )
    .unwrap();
    out.push_str(preamble);
    writeln!(out).unwrap();

    let mut keys: Vec<&str> = schema_map.keys().map(|s| s.as_str()).collect();
    keys.sort_unstable();

    for yaml_path in &keys {
        let info = &schema_map[*yaml_path];
        let const_name = yaml_path_to_const(yaml_path);
        let vt = schema_gen::field_type_as_rust(&info.value_type);

        if schema_gen::is_unknown(&info.value_type) {
            writeln!(
                out,
                "// TODO: unknown type for '{}' — set value_type_override in the annotation",
                yaml_path
            )
            .unwrap();
        }

        let env_vars_lit = if info.env_vars.is_empty() {
            "&[]".to_string()
        } else {
            let items: Vec<String> = info
                .env_vars
                .iter()
                .map(|e| format!("\"{}\"", schema_gen::escape_str(e)))
                .collect();
            format!("&[{}]", items.join(", "))
        };

        let default_lit = match &info.default {
            Some(d) => format!("Some(\"{}\")", schema_gen::escape_str(d)),
            None => "None".to_string(),
        };

        writeln!(out, "pub const {}: SchemaEntry = SchemaEntry {{", const_name).unwrap();
        writeln!(out, "    schema: Schema::Datadog,").unwrap();
        writeln!(out, "    yaml_path: \"{}\",", yaml_path).unwrap();
        writeln!(out, "    env_vars: {},", env_vars_lit).unwrap();
        writeln!(out, "    value_type: {},", vt).unwrap();
        writeln!(out, "    default: {},", default_lit).unwrap();
        writeln!(out, "}};").unwrap();
        writeln!(out).unwrap();
    }

    let path = dir.join("schema.rs");
    std::fs::write(&path, out).unwrap_or_else(|e| panic!("cannot write {}: {}", path.display(), e));
}

fn generate_subsystem_files(overlay: &SchemaOverlay, schema_map: &IndexMap<String, FieldInfo>, dir: &Path, preamble: &str) {
    let mut datadog_by_file: IndexMap<String, Vec<(&str, &datadog_agent_config_overlay_model::Supported)>> =
        IndexMap::new();
    for (yaml_path, entry) in &overlay.supported {
        let filename = entry
            .additional_attributes
            .get("config_registry_filename")
            .unwrap_or_else(|| panic!("supported key '{}' missing config_registry_filename", yaml_path));
        datadog_by_file
            .entry(filename.clone())
            .or_default()
            .push((yaml_path.as_str(), entry));
    }

    let mut saluki_by_file: IndexMap<&str, Vec<&SalukiEntry>> = IndexMap::new();
    for entry in SALUKI_ENTRIES {
        saluki_by_file.entry(entry.filename).or_default().push(entry);
    }

    let mut all_files: Vec<String> = datadog_by_file.keys().cloned().collect();
    for &f in saluki_by_file.keys() {
        if !all_files.iter().any(|x| x == f) {
            all_files.push(f.to_string());
        }
    }
    all_files.sort_unstable();

    for filename in &all_files {
        let datadog_entries = datadog_by_file
            .get(filename.as_str())
            .map(|v| v.as_slice())
            .unwrap_or(&[]);
        let saluki_entries = saluki_by_file
            .get(filename.as_str())
            .map(|v| v.as_slice())
            .unwrap_or(&[]);
        generate_one_file(filename, datadog_entries, saluki_entries, schema_map, dir, preamble);
    }
}

fn generate_one_file(
    filename: &str, datadog_entries: &[(&str, &datadog_agent_config_overlay_model::Supported)],
    saluki_entries: &[&SalukiEntry], schema_map: &IndexMap<String, FieldInfo>, dir: &Path, preamble: &str,
) {
    let mut out = String::new();
    writeln!(out, "// @generated by build.rs from schema_overlay.yaml — DO NOT EDIT").unwrap();
    out.push_str(preamble);
    writeln!(out).unwrap();

    for se in saluki_entries {
        let const_name = format!("{}_SCHEMA", yaml_path_to_const(se.yaml_path));
        let default_lit = match se.schema_default {
            Some(d) => format!("Some(\"{}\")", schema_gen::escape_str(d)),
            None => "None".to_string(),
        };
        let env_vars_lit = if se.env_vars.is_empty() {
            "&[]".to_string()
        } else {
            let items: Vec<String> = se.env_vars.iter().map(|e| format!("\"{}\"", e)).collect();
            format!("&[{}]", items.join(", "))
        };
        writeln!(out, "static {}: SchemaEntry = SchemaEntry {{", const_name).unwrap();
        writeln!(out, "    schema: Schema::Saluki,").unwrap();
        writeln!(out, "    yaml_path: \"{}\",", se.yaml_path).unwrap();
        writeln!(out, "    env_vars: {},", env_vars_lit).unwrap();
        writeln!(out, "    value_type: {},", se.value_type).unwrap();
        writeln!(out, "    default: {},", default_lit).unwrap();
        writeln!(out, "}};").unwrap();
        writeln!(out).unwrap();
    }

    writeln!(out, "crate::declare_annotations! {{").unwrap();

    for (yaml_path, entry) in datadog_entries {
        emit_datadog_annotation(&mut out, yaml_path, entry, schema_map);
    }
    for se in saluki_entries {
        emit_saluki_annotation(&mut out, se);
    }

    writeln!(out, "}}").unwrap();

    let path = dir.join(filename);
    std::fs::write(&path, out).unwrap_or_else(|e| panic!("cannot write {}: {}", path.display(), e));
}

fn emit_datadog_annotation(
    out: &mut String, yaml_path: &str, entry: &datadog_agent_config_overlay_model::Supported,
    _schema_map: &IndexMap<String, FieldInfo>,
) {
    let const_name = yaml_path_to_const(yaml_path);
    let support_level = overlay_support_level(&entry.support_level);
    let pipeline_affinity = overlay_pipeline_affinity_expr(&entry.pipelines);

    let alias_paths = &entry.additional_yaml_paths;
    let alias_lit = if alias_paths.is_empty() {
        "&[]".to_string()
    } else {
        let items: Vec<String> = alias_paths.iter().map(|p| format!("\"{}\"", p)).collect();
        format!("&[{}]", items.join(", "))
    };

    let env_override = match &entry.env_var_override {
        None => "None".to_string(),
        Some(vars) => {
            let items: Vec<String> = vars
                .iter()
                .map(|v| format!("\"{}\"", schema_gen::escape_str(v)))
                .collect();
            format!("Some(&[{}])", items.join(", "))
        }
    };

    let used_by_lit = {
        let items: Vec<String> = entry
            .used_by
            .iter()
            .map(|u| format!("structs::{}", u.as_smoke_test_const()))
            .collect();
        format!("&[{}]", items.join(", "))
    };

    let vt_override = match &entry.value_type_override {
        None => "None".to_string(),
        Some(vt) => format!("Some({})", overlay_value_type(vt)),
    };

    let test_json_lit = match &entry.test_json {
        None => "None".to_string(),
        Some(s) => format!("Some(\"{}\")", schema_gen::escape_str(s)),
    };

    let description = &entry.description;

    writeln!(out, "    /// `{}`-{}", yaml_path, description).unwrap();
    writeln!(out, "    {} = SalukiAnnotation {{", const_name).unwrap();
    writeln!(out, "        schema: &super::schema::{},", const_name).unwrap();
    writeln!(out, "        support_level: {},", support_level).unwrap();
    writeln!(out, "        additional_yaml_paths: {},", alias_lit).unwrap();
    writeln!(out, "        env_var_override: {},", env_override).unwrap();
    writeln!(out, "        used_by: {},", used_by_lit).unwrap();
    writeln!(out, "        value_type_override: {},", vt_override).unwrap();
    writeln!(out, "        test_json: {},", test_json_lit).unwrap();
    writeln!(out, "        pipeline_affinity: {},", pipeline_affinity).unwrap();
    writeln!(out, "    }};").unwrap();
}

fn emit_saluki_annotation(out: &mut String, se: &SalukiEntry) {
    let const_name = yaml_path_to_const(se.yaml_path);
    let schema_const = format!("{}_SCHEMA", const_name);

    let alias_lit = if se.additional_yaml_paths.is_empty() {
        "&[]".to_string()
    } else {
        let items: Vec<String> = se.additional_yaml_paths.iter().map(|p| format!("\"{}\"", p)).collect();
        format!("&[{}]", items.join(", "))
    };

    let env_override = match se.env_var_override {
        None => "None".to_string(),
        Some(vars) => {
            let items: Vec<String> = vars.iter().map(|v| format!("\"{}\"", v)).collect();
            format!("Some(&[{}])", items.join(", "))
        }
    };

    let used_by_lit = {
        let items: Vec<String> = se.used_by.iter().map(|u| format!("structs::{}", u)).collect();
        format!("&[{}]", items.join(", "))
    };

    let test_json_lit = match se.test_json {
        None => "None".to_string(),
        Some(s) => format!("Some(\"{}\")", schema_gen::escape_str(s)),
    };

    writeln!(out, "    /// `{}`", se.yaml_path).unwrap();
    writeln!(out, "    {} = SalukiAnnotation {{", const_name).unwrap();
    writeln!(out, "        schema: &{},", schema_const).unwrap();
    writeln!(out, "        support_level: SupportLevel::Full,").unwrap();
    writeln!(out, "        additional_yaml_paths: {},", alias_lit).unwrap();
    writeln!(out, "        env_var_override: {},", env_override).unwrap();
    writeln!(out, "        used_by: {},", used_by_lit).unwrap();
    writeln!(out, "        value_type_override: None,").unwrap();
    writeln!(out, "        test_json: {},", test_json_lit).unwrap();
    writeln!(out, "        pipeline_affinity: {},", se.pipeline_affinity).unwrap();
    writeln!(out, "    }};").unwrap();
}

fn generate_unsupported_rs(overlay: &SchemaOverlay, dir: &Path, preamble: &str) {
    let mut out = String::new();
    writeln!(out, "// @generated by build.rs from schema_overlay.yaml — DO NOT EDIT").unwrap();
    out.push_str(preamble);
    writeln!(out, "crate::declare_annotations! {{").unwrap();

    for (yaml_path, entry) in &overlay.unsupported {
        let const_name = yaml_path_to_const(yaml_path);
        let severity = match entry.severity {
            Severity::Low => "Severity::Low",
            Severity::Medium => "Severity::Medium",
            Severity::High => "Severity::High",
        };
        let pipeline_affinity = overlay_pipeline_affinity_expr(&entry.pipelines);

        writeln!(out, "    /// `{}`-{}", yaml_path, entry.description).unwrap();
        writeln!(out, "    {} = SalukiAnnotation {{", const_name).unwrap();
        writeln!(out, "        schema: &super::schema::{},", const_name).unwrap();
        writeln!(out, "        support_level: SupportLevel::Incompatible({}),", severity).unwrap();
        writeln!(out, "        additional_yaml_paths: &[],").unwrap();
        writeln!(out, "        env_var_override: None,").unwrap();
        writeln!(out, "        used_by: &[],").unwrap();
        writeln!(out, "        value_type_override: None,").unwrap();
        writeln!(out, "        test_json: None,").unwrap();
        writeln!(out, "        pipeline_affinity: {},", pipeline_affinity).unwrap();
        writeln!(out, "    }};").unwrap();
    }

    for (yaml_path, entry) in &overlay.investigate {
        let severity = match entry.severity {
            Some(Severity::Low) => "Severity::Low",
            Some(Severity::Medium) => "Severity::Medium",
            Some(Severity::High) => "Severity::High",
            None => continue,
        };
        let const_name = yaml_path_to_const(yaml_path);

        writeln!(out, "    /// `{}`-{}", yaml_path, entry.description).unwrap();
        writeln!(out, "    {} = SalukiAnnotation {{", const_name).unwrap();
        writeln!(out, "        schema: &super::schema::{},", const_name).unwrap();
        writeln!(out, "        support_level: SupportLevel::Incompatible({}),", severity).unwrap();
        writeln!(out, "        additional_yaml_paths: &[],").unwrap();
        writeln!(out, "        env_var_override: None,").unwrap();
        writeln!(out, "        used_by: &[],").unwrap();
        writeln!(out, "        value_type_override: None,").unwrap();
        writeln!(out, "        test_json: None,").unwrap();
        writeln!(out, "        pipeline_affinity: PipelineAffinity::CrossCutting,").unwrap();
        writeln!(out, "    }};").unwrap();
    }

    writeln!(out, "}}").unwrap();

    let path = dir.join("unsupported.rs");
    std::fs::write(&path, out).unwrap_or_else(|e| panic!("cannot write {}: {}", path.display(), e));
}

#[allow(dead_code)]
fn generate_mod_rs(overlay: &SchemaOverlay, dir: &Path) {
    let supported_files: Vec<String> = {
        let mut files: std::collections::HashSet<String> = overlay
            .supported
            .values()
            .filter_map(|e| e.additional_attributes.get("config_registry_filename").cloned())
            .collect();
        for se in SALUKI_ENTRIES {
            files.insert(se.filename.to_string());
        }
        let mut files: Vec<String> = files.into_iter().collect();
        files.sort_unstable();
        files
    };

    let mut out = String::new();
    writeln!(out, "// @generated by build.rs from schema_overlay.yaml — DO NOT EDIT").unwrap();
    writeln!(out).unwrap();
    writeln!(
        out,
        "use crate::config_registry::{{Pipeline, PipelineAffinity, SalukiAnnotation, Schema, SchemaEntry, Severity, SupportLevel, ValueType, structs}};"
    )
    .unwrap();
    writeln!(out).unwrap();

    writeln!(out, "#[allow(dead_code)]").unwrap();
    writeln!(out, "mod schema {{").unwrap();
    writeln!(out, "    #[allow(unused_imports)]").unwrap();
    writeln!(out, "    use super::*;").unwrap();
    writeln!(
        out,
        "    include!(concat!(env!(\"OUT_DIR\"), \"/config_registry/schema.rs\"));"
    )
    .unwrap();
    writeln!(out, "}}").unwrap();
    writeln!(out).unwrap();

    for filename in &supported_files {
        let stem = filename.trim_end_matches(".rs");
        writeln!(out, "mod {} {{", stem).unwrap();
        writeln!(out, "    #[allow(unused_imports)]").unwrap();
        writeln!(out, "    use super::*;").unwrap();
        writeln!(
            out,
            "    include!(concat!(env!(\"OUT_DIR\"), \"/config_registry/{}\"));",
            filename
        )
        .unwrap();
        writeln!(out, "}}").unwrap();
        writeln!(out).unwrap();
    }

    writeln!(out, "pub(super) mod unsupported {{").unwrap();
    writeln!(out, "    #[allow(unused_imports)]").unwrap();
    writeln!(out, "    use super::*;").unwrap();
    writeln!(
        out,
        "    include!(concat!(env!(\"OUT_DIR\"), \"/config_registry/unsupported.rs\"));"
    )
    .unwrap();
    writeln!(out, "}}").unwrap();
    writeln!(out).unwrap();

    writeln!(out, "use std::sync::LazyLock;").unwrap();
    writeln!(out).unwrap();

    writeln!(
        out,
        "pub static SUPPORTED_ANNOTATIONS: LazyLock<Vec<&'static SalukiAnnotation>> = LazyLock::new(|| {{"
    )
    .unwrap();
    writeln!(out, "    let mut v: Vec<&'static SalukiAnnotation> = Vec::new();").unwrap();
    for filename in &supported_files {
        let stem = filename.trim_end_matches(".rs");
        writeln!(out, "    v.extend_from_slice({}::ALL);", stem).unwrap();
    }
    writeln!(out, "    v").unwrap();
    writeln!(out, "}});").unwrap();
    writeln!(out).unwrap();

    writeln!(
        out,
        "pub static UNSUPPORTED_ANNOTATIONS: LazyLock<Vec<&'static SalukiAnnotation>> = LazyLock::new(|| {{"
    )
    .unwrap();
    writeln!(out, "    let mut v: Vec<&'static SalukiAnnotation> = Vec::new();").unwrap();
    writeln!(out, "    v.extend_from_slice(unsupported::ALL);").unwrap();
    writeln!(out, "    v").unwrap();
    writeln!(out, "}});").unwrap();
    writeln!(out).unwrap();

    writeln!(
        out,
        "pub static ALL_ANNOTATIONS: LazyLock<Vec<&'static SalukiAnnotation>> = LazyLock::new(|| {{"
    )
    .unwrap();
    writeln!(out, "    let mut v = SUPPORTED_ANNOTATIONS.clone();").unwrap();
    writeln!(out, "    v.extend_from_slice(&UNSUPPORTED_ANNOTATIONS);").unwrap();
    writeln!(out, "    v").unwrap();
    writeln!(out, "}});").unwrap();

    let path = dir.join("mod.rs");
    std::fs::write(&path, out).unwrap_or_else(|e| panic!("cannot write {}: {}", path.display(), e));
}

/// Generate the in-tree `mod.rs` using plain `mod` declarations (file-based modules) instead of
/// `include!(OUT_DIR)` blocks. The sibling files are compiled directly from the source tree.
fn generate_mod_rs_in_tree(overlay: &SchemaOverlay, dir: &Path) {
    let supported_files: Vec<String> = {
        let mut files: std::collections::HashSet<String> = overlay
            .supported
            .values()
            .filter_map(|e| e.additional_attributes.get("config_registry_filename").cloned())
            .collect();
        for se in SALUKI_ENTRIES {
            files.insert(se.filename.to_string());
        }
        let mut files: Vec<String> = files.into_iter().collect();
        files.sort_unstable();
        files
    };

    let mut out = String::new();
    writeln!(out, "// @generated by build.rs from schema_overlay.yaml — DO NOT EDIT").unwrap();
    writeln!(out).unwrap();
    writeln!(
        out,
        "use crate::config_registry::{{Pipeline, PipelineAffinity, SalukiAnnotation, Schema, SchemaEntry, Severity, SupportLevel, ValueType, structs}};"
    )
    .unwrap();
    writeln!(out).unwrap();

    writeln!(out, "#[allow(dead_code)]").unwrap();
    writeln!(out, "mod schema;").unwrap();
    writeln!(out).unwrap();

    for filename in &supported_files {
        let stem = filename.trim_end_matches(".rs");
        writeln!(out, "mod {};", stem).unwrap();
    }
    writeln!(out).unwrap();

    writeln!(out, "pub(super) mod unsupported;").unwrap();
    writeln!(out).unwrap();

    writeln!(out, "use std::sync::LazyLock;").unwrap();
    writeln!(out).unwrap();

    writeln!(
        out,
        "pub static SUPPORTED_ANNOTATIONS: LazyLock<Vec<&'static SalukiAnnotation>> = LazyLock::new(|| {{"
    )
    .unwrap();
    writeln!(out, "    let mut v: Vec<&'static SalukiAnnotation> = Vec::new();").unwrap();
    for filename in &supported_files {
        let stem = filename.trim_end_matches(".rs");
        writeln!(out, "    v.extend_from_slice({}::ALL);", stem).unwrap();
    }
    writeln!(out, "    v").unwrap();
    writeln!(out, "}});").unwrap();
    writeln!(out).unwrap();

    writeln!(
        out,
        "pub static UNSUPPORTED_ANNOTATIONS: LazyLock<Vec<&'static SalukiAnnotation>> = LazyLock::new(|| {{"
    )
    .unwrap();
    writeln!(out, "    let mut v: Vec<&'static SalukiAnnotation> = Vec::new();").unwrap();
    writeln!(out, "    v.extend_from_slice(unsupported::ALL);").unwrap();
    writeln!(out, "    v").unwrap();
    writeln!(out, "}});").unwrap();
    writeln!(out).unwrap();

    writeln!(
        out,
        "pub static ALL_ANNOTATIONS: LazyLock<Vec<&'static SalukiAnnotation>> = LazyLock::new(|| {{"
    )
    .unwrap();
    writeln!(out, "    let mut v = SUPPORTED_ANNOTATIONS.clone();").unwrap();
    writeln!(out, "    v.extend_from_slice(&UNSUPPORTED_ANNOTATIONS);").unwrap();
    writeln!(out, "    v").unwrap();
    writeln!(out, "}});").unwrap();

    let path = dir.join("mod.rs");
    std::fs::write(&path, out).unwrap_or_else(|e| panic!("cannot write {}: {}", path.display(), e));
}

fn yaml_path_to_const(yaml_path: &str) -> String {
    yaml_path
        .chars()
        .map(|c| if c == '.' || c == '-' { '_' } else { c })
        .collect::<String>()
        .to_uppercase()
}

fn overlay_support_level(sl: &SupportLevel) -> &'static str {
    match sl {
        SupportLevel::Full => "SupportLevel::Full",
        SupportLevel::Partial => "SupportLevel::Partial",
    }
}

fn overlay_pipeline_affinity_expr(pa: &PipelineAffinity) -> String {
    match pa {
        PipelineAffinity::CrossCutting => "PipelineAffinity::CrossCutting".to_string(),
        PipelineAffinity::Pipelines(ps) => {
            let parts: Vec<&str> = ps
                .iter()
                .map(|p| match p {
                    Pipeline::DogStatsD => "Pipeline::DogStatsD",
                    Pipeline::Checks => "Pipeline::Checks",
                    Pipeline::Otlp => "Pipeline::Otlp",
                    Pipeline::Traces => "Pipeline::Traces",
                })
                .collect();
            format!("PipelineAffinity::Pipelines(&[{}])", parts.join(", "))
        }
    }
}

fn overlay_value_type(vt: &ValueType) -> &'static str {
    match vt {
        ValueType::Boolean => "ValueType::Bool",
        ValueType::Integer => "ValueType::Integer",
        ValueType::Float => "ValueType::Float",
        ValueType::String => "ValueType::String",
        ValueType::StringList => "ValueType::StringList",
    }
}
