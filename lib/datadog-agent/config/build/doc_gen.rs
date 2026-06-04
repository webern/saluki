use std::collections::{BTreeMap, HashMap};
use std::fmt::Write as _;
use std::path::Path;

use datadog_agent_config_overlay_model::{
    Investigate, PipelineAffinity, SchemaOverlay, SupportLevel, Supported, Unsupported,
};
use tinytemplate::TinyTemplate;

const ISSUE_BASE_URL: &str = "https://github.com/DataDog/saluki/issues/";

const PIPELINE_SECTIONS: &[&str] = &["cross_cutting", "dogstatsd", "checks", "otlp", "traces"];

fn classify_pipeline(pa: &PipelineAffinity) -> &'static str {
    match pa {
        PipelineAffinity::CrossCutting => "cross_cutting",
        PipelineAffinity::Pipelines(ps) => {
            if ps.len() > 1 {
                "cross_cutting"
            } else {
                match ps[0] {
                    datadog_agent_config_overlay_model::Pipeline::DogStatsD => "dogstatsd",
                    datadog_agent_config_overlay_model::Pipeline::Checks => "checks",
                    datadog_agent_config_overlay_model::Pipeline::Otlp => "otlp",
                    datadog_agent_config_overlay_model::Pipeline::Traces => "traces",
                }
            }
        }
    }
}

// ── Table rendering ─────────────────────────────────────────────────────────

struct TwoColRow {
    key: String,
    description: String,
}

struct ThreeColRow {
    key: String,
    description: String,
    extra: String,
}

fn render_two_col_table(headers: [&str; 2], rows: &[TwoColRow]) -> String {
    if rows.is_empty() {
        return String::new();
    }
    let w0 = rows.iter().map(|r| r.key.len()).max().unwrap().max(headers[0].len());
    let w1 = rows
        .iter()
        .map(|r| r.description.len())
        .max()
        .unwrap()
        .max(headers[1].len());
    let mut out = String::new();
    writeln!(out, "| {:<w0$} | {:<w1$} |", headers[0], headers[1]).unwrap();
    writeln!(out, "| {:-<w0$} | {:-<w1$} |", "", "").unwrap();
    for row in rows {
        writeln!(out, "| {:<w0$} | {:<w1$} |", row.key, row.description).unwrap();
    }
    out
}

fn render_three_col_table(headers: [&str; 3], rows: &[ThreeColRow]) -> String {
    if rows.is_empty() {
        return String::new();
    }
    let w0 = rows.iter().map(|r| r.key.len()).max().unwrap().max(headers[0].len());
    let w1 = rows
        .iter()
        .map(|r| r.description.len())
        .max()
        .unwrap()
        .max(headers[1].len());
    let w2 = rows.iter().map(|r| r.extra.len()).max().unwrap().max(headers[2].len());
    let mut out = String::new();
    writeln!(
        out,
        "| {:<w0$} | {:<w1$} | {:<w2$} |",
        headers[0], headers[1], headers[2]
    )
    .unwrap();
    writeln!(out, "| {:-<w0$} | {:-<w1$} | {:-<w2$} |", "", "", "").unwrap();
    for row in rows {
        writeln!(
            out,
            "| {:<w0$} | {:<w1$} | {:<w2$} |",
            row.key, row.description, row.extra
        )
        .unwrap();
    }
    out
}

// ── Issue reference handling ────────────────────────────────────────────────

fn parse_issue_number(issue: &str) -> Option<u64> {
    issue.strip_prefix('#').and_then(|s| s.parse().ok())
}

/// Record an issue reference and return the markdown reference-style link text.
/// Uses the raw issue string (e.g. "#0001") as-is for both the link text and reference name.
fn collect_issue(issue: &Option<String>, issues: &mut BTreeMap<u64, String>) -> String {
    match issue {
        Some(i) => {
            if let Some(n) = parse_issue_number(i) {
                issues.entry(n).or_insert_with(|| i.clone());
            }
            format!("[{}]", i)
        }
        None => String::new(),
    }
}

// ── Documentation block rendering ───────────────────────────────────────────

fn render_docs_block(entries: &[(&str, &str)]) -> String {
    let mut out = String::new();
    for (key, doc) in entries {
        writeln!(out, "### `{}`", key).unwrap();
        writeln!(out).unwrap();
        writeln!(out, "{}", doc.trim()).unwrap();
        writeln!(out).unwrap();
    }
    out
}

// ── Slice builders ──────────────────────────────────────────────────────────

fn build_supported_slice(
    entries: &[(&str, &Supported)], level: SupportLevel, issues: &mut BTreeMap<u64, String>,
) -> (String, String) {
    let filtered: Vec<_> = entries
        .iter()
        .filter(|(_, s)| s.support_level == level)
        .copied()
        .collect();

    let rows: Vec<TwoColRow> = filtered
        .iter()
        .map(|(key, s)| TwoColRow {
            key: format!("`{}`", key),
            description: s.description.clone(),
        })
        .collect();
    let table = render_two_col_table(["Config Key", "Description"], &rows);

    for (_, s) in &filtered {
        if let Some(i) = &s.issue {
            if let Some(n) = parse_issue_number(i) {
                issues.entry(n).or_insert_with(|| i.clone());
            }
        }
    }

    let doc_entries: Vec<(&str, &str)> = filtered
        .iter()
        .filter_map(|(key, s)| s.documentation.as_deref().map(|d| (*key, d)))
        .collect();
    let docs = render_docs_block(&doc_entries);

    (table, docs)
}

fn build_unsupported_slice(
    entries: &[(&str, &Unsupported)], planned: bool, issues: &mut BTreeMap<u64, String>,
) -> (String, String) {
    let filtered: Vec<_> = entries.iter().filter(|(_, u)| u.planned == planned).copied().collect();

    let table = if planned {
        let rows: Vec<ThreeColRow> = filtered
            .iter()
            .map(|(key, u)| ThreeColRow {
                key: format!("`{}`", key),
                description: u.description.clone(),
                extra: collect_issue(&u.issue, issues),
            })
            .collect();
        render_three_col_table(["Config Key", "Description", "Issue"], &rows)
    } else {
        let rows: Vec<TwoColRow> = filtered
            .iter()
            .map(|(key, u)| TwoColRow {
                key: format!("`{}`", key),
                description: u.description.clone(),
            })
            .collect();
        render_two_col_table(["Config Key", "Description"], &rows)
    };

    if !planned {
        for (_, u) in &filtered {
            if let Some(i) = &u.issue {
                if let Some(n) = parse_issue_number(i) {
                    issues.entry(n).or_insert_with(|| i.clone());
                }
            }
        }
    }

    let doc_entries: Vec<(&str, &str)> = filtered
        .iter()
        .filter_map(|(key, u)| u.documentation.as_deref().map(|d| (*key, d)))
        .collect();
    let docs = render_docs_block(&doc_entries);

    (table, docs)
}

fn build_investigate_slice(
    entries: &[(&str, &Investigate)], issues: &mut BTreeMap<u64, String>,
) -> (String, String) {
    let rows: Vec<ThreeColRow> = entries
        .iter()
        .map(|(key, inv)| ThreeColRow {
            key: format!("`{}`", key),
            description: inv.description.clone(),
            extra: collect_issue(&inv.issue, issues),
        })
        .collect();
    let table = render_three_col_table(["Config Key", "Description", "Issue"], &rows);
    (table, String::new())
}

// ── Public entry point ──────────────────────────────────────────────────────

pub fn generate(overlay: &SchemaOverlay, template_path: &Path, out_dir: &Path) {
    let template_src = std::fs::read_to_string(template_path)
        .unwrap_or_else(|e| panic!("cannot read {}: {}", template_path.display(), e));

    // Partition supported entries by pipeline.
    let mut sup_by_pipeline: HashMap<&str, Vec<(&str, &Supported)>> = HashMap::new();
    for section in PIPELINE_SECTIONS {
        sup_by_pipeline.insert(section, Vec::new());
    }
    for (key, entry) in &overlay.supported {
        let pk = classify_pipeline(&entry.pipelines);
        sup_by_pipeline.get_mut(pk).unwrap().push((key.as_str(), entry));
    }

    // Partition unsupported entries by pipeline.
    let mut unsup_by_pipeline: HashMap<&str, Vec<(&str, &Unsupported)>> = HashMap::new();
    for section in PIPELINE_SECTIONS {
        unsup_by_pipeline.insert(section, Vec::new());
    }
    for (key, entry) in &overlay.unsupported {
        let pk = classify_pipeline(&entry.pipelines);
        unsup_by_pipeline.get_mut(pk).unwrap().push((key.as_str(), entry));
    }

    let inv_entries: Vec<(&str, &Investigate)> = overlay
        .investigate
        .iter()
        .map(|(k, v)| (k.as_str(), v))
        .collect();

    let mut issues: BTreeMap<u64, String> = BTreeMap::new();
    let mut ctx: HashMap<String, String> = HashMap::new();

    for &section in PIPELINE_SECTIONS {
        let sup = &sup_by_pipeline[section];
        let unsup = &unsup_by_pipeline[section];

        let (t, d) = build_supported_slice(sup, SupportLevel::Full, &mut issues);
        ctx.insert(format!("{}_transparent_table", section), t);
        ctx.insert(format!("{}_transparent_docs", section), d);

        let (t, d) = build_supported_slice(sup, SupportLevel::Partial, &mut issues);
        ctx.insert(format!("{}_behavioral_table", section), t);
        ctx.insert(format!("{}_behavioral_docs", section), d);

        let (t, d) = build_unsupported_slice(unsup, true, &mut issues);
        ctx.insert(format!("{}_working_on_table", section), t);
        ctx.insert(format!("{}_working_on_docs", section), d);

        let (t, d) = build_unsupported_slice(unsup, false, &mut issues);
        ctx.insert(format!("{}_not_planned_table", section), t);
        ctx.insert(format!("{}_not_planned_docs", section), d);
    }

    let (t, _) = build_investigate_slice(&inv_entries, &mut issues);
    ctx.insert("investigate_table".to_string(), t);

    let mut issue_refs = String::new();
    for (n, raw) in &issues {
        writeln!(issue_refs, "[{}]: {}{}", raw, ISSUE_BASE_URL, n).unwrap();
    }
    ctx.insert("issue_references".to_string(), issue_refs);

    let mut tt = TinyTemplate::new();
    tt.set_default_formatter(&tinytemplate::format_unescaped);
    tt.add_template("doc", &template_src)
        .unwrap_or_else(|e| panic!("template parse error: {}", e));

    let rendered = tt
        .render("doc", &ctx)
        .unwrap_or_else(|e| panic!("template render error: {}", e));

    let docs_dir = out_dir.join("docs");
    std::fs::create_dir_all(&docs_dir).unwrap();
    let out_path = docs_dir.join("configuration.md");
    std::fs::write(&out_path, rendered).unwrap_or_else(|e| panic!("cannot write {}: {}", out_path.display(), e));
}
