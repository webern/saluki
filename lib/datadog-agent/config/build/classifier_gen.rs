use std::fmt::Write as _;
use std::path::Path;

use datadog_agent_config_overlay_model::{PipelineAffinity, SchemaOverlay, Severity, SupportLevel};
use indexmap::IndexMap;

use super::schema_gen::FieldInfo;

// These keys are not in the Datadog Agent schema; they exist only in ADP.
// When config_registry is deleted, delete this table too.
struct SalukiEntry {
    yaml_path: &'static str,
    schema_default: Option<&'static str>,
    additional_yaml_paths: &'static [&'static str],
    pipeline_affinity: &'static str,
}

static SALUKI_ENTRIES: &[SalukiEntry] = &[
    // ── dogstatsd.rs ─────────────────────────────────────────────────────────
    SalukiEntry {
        yaml_path: "dogstatsd_allow_context_heap_allocs",
        schema_default: Some("true"),
        additional_yaml_paths: &[],
        pipeline_affinity: "PipelineAffinity::Pipelines(&[Pipeline::DogStatsD])",
    },
    SalukiEntry {
        yaml_path: "dogstatsd_autoscale_udp_listeners",
        schema_default: Some("false"),
        additional_yaml_paths: &[],
        pipeline_affinity: "PipelineAffinity::Pipelines(&[Pipeline::DogStatsD])",
    },
    SalukiEntry {
        yaml_path: "dogstatsd_buffer_count",
        schema_default: None,
        additional_yaml_paths: &[],
        pipeline_affinity: "PipelineAffinity::Pipelines(&[Pipeline::DogStatsD])",
    },
    SalukiEntry {
        yaml_path: "dogstatsd_cached_contexts_limit",
        schema_default: None,
        additional_yaml_paths: &[],
        pipeline_affinity: "PipelineAffinity::Pipelines(&[Pipeline::DogStatsD])",
    },
    SalukiEntry {
        yaml_path: "dogstatsd_cached_tagsets_limit",
        schema_default: None,
        additional_yaml_paths: &[],
        pipeline_affinity: "PipelineAffinity::Pipelines(&[Pipeline::DogStatsD])",
    },
    SalukiEntry {
        yaml_path: "dogstatsd_minimum_sample_rate",
        schema_default: None,
        additional_yaml_paths: &[],
        pipeline_affinity: "PipelineAffinity::Pipelines(&[Pipeline::DogStatsD])",
    },
    SalukiEntry {
        yaml_path: "dogstatsd_permissive_decoding",
        schema_default: Some("true"),
        additional_yaml_paths: &[],
        pipeline_affinity: "PipelineAffinity::Pipelines(&[Pipeline::DogStatsD])",
    },
    SalukiEntry {
        yaml_path: "dogstatsd_string_interner_size_bytes",
        schema_default: None,
        additional_yaml_paths: &[],
        pipeline_affinity: "PipelineAffinity::Pipelines(&[Pipeline::DogStatsD])",
    },
    SalukiEntry {
        yaml_path: "dogstatsd_tcp_port",
        schema_default: None,
        additional_yaml_paths: &[],
        pipeline_affinity: "PipelineAffinity::Pipelines(&[Pipeline::DogStatsD])",
    },
    // ── otlp.rs ──────────────────────────────────────────────────────────────
    SalukiEntry {
        yaml_path: "otlp_config.traces.enable_otlp_compute_top_level_by_span_kind",
        schema_default: Some("true"),
        additional_yaml_paths: &[],
        pipeline_affinity: "PipelineAffinity::Pipelines(&[Pipeline::Traces])",
    },
    SalukiEntry {
        yaml_path: "otlp_config.traces.ignore_missing_datadog_fields",
        schema_default: None,
        additional_yaml_paths: &[],
        pipeline_affinity: "PipelineAffinity::Pipelines(&[Pipeline::Traces])",
    },
    SalukiEntry {
        yaml_path: "otlp_config.traces.string_interner_size",
        schema_default: None,
        additional_yaml_paths: &[],
        pipeline_affinity: "PipelineAffinity::Pipelines(&[Pipeline::Traces])",
    },
    SalukiEntry {
        yaml_path: "otlp_config.receiver.protocols.http.transport",
        schema_default: None,
        additional_yaml_paths: &[],
        pipeline_affinity: "PipelineAffinity::Pipelines(&[Pipeline::Otlp])",
    },
    SalukiEntry {
        yaml_path: "otlp_allow_context_heap_allocs",
        schema_default: Some("true"),
        additional_yaml_paths: &[],
        pipeline_affinity: "PipelineAffinity::Pipelines(&[Pipeline::Otlp])",
    },
    SalukiEntry {
        yaml_path: "otlp_cached_contexts_limit",
        schema_default: None,
        additional_yaml_paths: &[],
        pipeline_affinity: "PipelineAffinity::Pipelines(&[Pipeline::Otlp])",
    },
    SalukiEntry {
        yaml_path: "otlp_cached_tagsets_limit",
        schema_default: None,
        additional_yaml_paths: &[],
        pipeline_affinity: "PipelineAffinity::Pipelines(&[Pipeline::Otlp])",
    },
    SalukiEntry {
        yaml_path: "otlp_string_interner_size",
        schema_default: None,
        additional_yaml_paths: &[],
        pipeline_affinity: "PipelineAffinity::Pipelines(&[Pipeline::Otlp])",
    },
    // ── aggregate.rs ─────────────────────────────────────────────────────────
    SalukiEntry {
        yaml_path: "aggregate_window_duration_seconds",
        schema_default: None,
        additional_yaml_paths: &[],
        pipeline_affinity: "PipelineAffinity::Pipelines(&[Pipeline::DogStatsD, Pipeline::Checks])",
    },
    SalukiEntry {
        yaml_path: "aggregate_flush_interval",
        schema_default: None,
        additional_yaml_paths: &[],
        pipeline_affinity: "PipelineAffinity::Pipelines(&[Pipeline::DogStatsD, Pipeline::Checks])",
    },
    SalukiEntry {
        yaml_path: "aggregate_context_limit",
        schema_default: None,
        additional_yaml_paths: &[],
        pipeline_affinity: "PipelineAffinity::Pipelines(&[Pipeline::DogStatsD, Pipeline::Checks])",
    },
    SalukiEntry {
        yaml_path: "counter_expiry_seconds",
        schema_default: None,
        additional_yaml_paths: &["dogstatsd_expiry_seconds"],
        pipeline_affinity: "PipelineAffinity::Pipelines(&[Pipeline::DogStatsD, Pipeline::Checks])",
    },
    SalukiEntry {
        yaml_path: "aggregate_passthrough_idle_flush_timeout",
        schema_default: None,
        additional_yaml_paths: &[],
        pipeline_affinity: "PipelineAffinity::Pipelines(&[Pipeline::DogStatsD, Pipeline::Checks])",
    },
    // ── trace_obfuscation.rs ─────────────────────────────────────────────────
    SalukiEntry {
        yaml_path: "apm_config.obfuscation.sql.dbms",
        schema_default: None,
        additional_yaml_paths: &[],
        pipeline_affinity: "PipelineAffinity::Pipelines(&[Pipeline::Traces])",
    },
    SalukiEntry {
        yaml_path: "apm_config.obfuscation.sql.dollar_quoted_func",
        schema_default: None,
        additional_yaml_paths: &[],
        pipeline_affinity: "PipelineAffinity::Pipelines(&[Pipeline::Traces])",
    },
    SalukiEntry {
        yaml_path: "apm_config.obfuscation.sql.keep_sql_alias",
        schema_default: None,
        additional_yaml_paths: &[],
        pipeline_affinity: "PipelineAffinity::Pipelines(&[Pipeline::Traces])",
    },
    SalukiEntry {
        yaml_path: "apm_config.obfuscation.sql.replace_digits",
        schema_default: None,
        additional_yaml_paths: &[],
        pipeline_affinity: "PipelineAffinity::Pipelines(&[Pipeline::Traces])",
    },
    SalukiEntry {
        yaml_path: "apm_config.obfuscation.sql.table_names",
        schema_default: None,
        additional_yaml_paths: &[],
        pipeline_affinity: "PipelineAffinity::Pipelines(&[Pipeline::Traces])",
    },
    // ── encoders.rs ──────────────────────────────────────────────────────────
    SalukiEntry {
        yaml_path: "flush_timeout_secs",
        schema_default: None,
        additional_yaml_paths: &[],
        pipeline_affinity: "PipelineAffinity::Pipelines(&[Pipeline::Checks, Pipeline::Traces, Pipeline::DogStatsD])",
    },
    SalukiEntry {
        yaml_path: "serializer_max_metrics_per_payload",
        schema_default: None,
        additional_yaml_paths: &[],
        pipeline_affinity: "PipelineAffinity::Pipelines(&[Pipeline::Checks, Pipeline::DogStatsD])",
    },
    // ── dogstatsd_mapper.rs ──────────────────────────────────────────────────
    SalukiEntry {
        yaml_path: "dogstatsd_mapper_string_interner_size",
        schema_default: None,
        additional_yaml_paths: &[],
        pipeline_affinity: "PipelineAffinity::Pipelines(&[Pipeline::DogStatsD])",
    },
];

pub fn generate(overlay: &SchemaOverlay, schema_map: &IndexMap<String, FieldInfo>, out_dir: &Path) {
    validate_saluki_entries(overlay);

    let mut out = String::new();
    writeln!(
        out,
        "// @generated by build.rs from core_schema.yaml + schema_overlay.yaml — DO NOT EDIT"
    )
    .unwrap();
    writeln!(out).unwrap();
    writeln!(out, "pub(crate) static CLASSIFIER_ENTRIES: &[ClassifierEntry] = &[").unwrap();

    for (yaml_path, entry) in &overlay.supported {
        let alias_lit = alias_literal(&entry.additional_yaml_paths);
        let support_level = overlay_support_level(&entry.support_level);
        let pipeline_affinity = overlay_pipeline_affinity_expr(&entry.pipelines);
        let default_lit = schema_default_literal(yaml_path, schema_map);

        writeln!(out, "    ClassifierEntry {{").unwrap();
        writeln!(out, "        yaml_path: \"{}\",", yaml_path).unwrap();
        writeln!(out, "        aliases: {},", alias_lit).unwrap();
        writeln!(out, "        support_level: {},", support_level).unwrap();
        writeln!(out, "        pipeline_affinity: {},", pipeline_affinity).unwrap();
        writeln!(out, "        default: {},", default_lit).unwrap();
        writeln!(out, "    }},").unwrap();
    }

    for (yaml_path, entry) in &overlay.unsupported {
        let severity = match entry.severity {
            Severity::Low => "Severity::Low",
            Severity::Medium => "Severity::Medium",
            Severity::High => "Severity::High",
        };
        let pipeline_affinity = overlay_pipeline_affinity_expr(&entry.pipelines);
        let default_lit = schema_default_literal(yaml_path, schema_map);

        writeln!(out, "    ClassifierEntry {{").unwrap();
        writeln!(out, "        yaml_path: \"{}\",", yaml_path).unwrap();
        writeln!(out, "        aliases: &[],").unwrap();
        writeln!(out, "        support_level: SupportLevel::Incompatible({}),", severity).unwrap();
        writeln!(out, "        pipeline_affinity: {},", pipeline_affinity).unwrap();
        writeln!(out, "        default: {},", default_lit).unwrap();
        writeln!(out, "    }},").unwrap();
    }

    for (yaml_path, entry) in &overlay.investigate {
        let severity = match entry.severity {
            Some(Severity::Low) => "Severity::Low",
            Some(Severity::Medium) => "Severity::Medium",
            Some(Severity::High) => "Severity::High",
            None => continue,
        };
        let default_lit = schema_default_literal(yaml_path, schema_map);

        writeln!(out, "    ClassifierEntry {{").unwrap();
        writeln!(out, "        yaml_path: \"{}\",", yaml_path).unwrap();
        writeln!(out, "        aliases: &[],").unwrap();
        writeln!(out, "        support_level: SupportLevel::Incompatible({}),", severity).unwrap();
        writeln!(out, "        pipeline_affinity: PipelineAffinity::CrossCutting,").unwrap();
        writeln!(out, "        default: {},", default_lit).unwrap();
        writeln!(out, "    }},").unwrap();
    }

    for se in SALUKI_ENTRIES {
        let alias_lit = if se.additional_yaml_paths.is_empty() {
            "&[]".to_string()
        } else {
            let items: Vec<String> = se.additional_yaml_paths.iter().map(|p| format!("\"{}\"", p)).collect();
            format!("&[{}]", items.join(", "))
        };
        let default_lit = match se.schema_default {
            Some(d) => format!("Some(\"{}\")", super::schema_gen::escape_str(d)),
            None => "None".to_string(),
        };

        writeln!(out, "    ClassifierEntry {{").unwrap();
        writeln!(out, "        yaml_path: \"{}\",", se.yaml_path).unwrap();
        writeln!(out, "        aliases: {},", alias_lit).unwrap();
        writeln!(out, "        support_level: SupportLevel::Full,").unwrap();
        writeln!(out, "        pipeline_affinity: {},", se.pipeline_affinity).unwrap();
        writeln!(out, "        default: {},", default_lit).unwrap();
        writeln!(out, "    }},").unwrap();
    }

    writeln!(out, "];").unwrap();

    let path = out_dir.join("classifier_data.rs");
    std::fs::write(&path, out).unwrap_or_else(|e| panic!("cannot write {}: {}", path.display(), e));
}

fn validate_saluki_entries(overlay: &SchemaOverlay) {
    for entry in SALUKI_ENTRIES {
        if overlay.supported.contains_key(entry.yaml_path)
            || overlay.unsupported.contains_key(entry.yaml_path)
            || overlay.investigate.contains_key(entry.yaml_path)
            || overlay.ignored.contains_key(entry.yaml_path)
        {
            panic!(
                "Saluki entry '{}' collides with a vendored schema key in the overlay — \
                 it should use the schema entry instead of a hard-coded entry",
                entry.yaml_path
            );
        }
    }
}

fn alias_literal(paths: &[String]) -> String {
    if paths.is_empty() {
        "&[]".to_string()
    } else {
        let items: Vec<String> = paths.iter().map(|p| format!("\"{}\"", p)).collect();
        format!("&[{}]", items.join(", "))
    }
}

fn schema_default_literal(yaml_path: &str, schema_map: &IndexMap<String, FieldInfo>) -> String {
    if let Some(info) = schema_map.get(yaml_path) {
        match &info.default {
            Some(d) => format!("Some(\"{}\")", super::schema_gen::escape_str(d)),
            None => "None".to_string(),
        }
    } else {
        "None".to_string()
    }
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
                    datadog_agent_config_overlay_model::Pipeline::DogStatsD => "Pipeline::DogStatsD",
                    datadog_agent_config_overlay_model::Pipeline::Checks => "Pipeline::Checks",
                    datadog_agent_config_overlay_model::Pipeline::Otlp => "Pipeline::Otlp",
                    datadog_agent_config_overlay_model::Pipeline::Traces => "Pipeline::Traces",
                })
                .collect();
            format!("PipelineAffinity::Pipelines(&[{}])", parts.join(", "))
        }
    }
}
