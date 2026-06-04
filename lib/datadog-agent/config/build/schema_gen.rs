use std::path::Path;

use indexmap::IndexMap;
use serde_yaml::Value;

pub enum FieldType {
    String,
    Bool,
    Integer,
    Float,
    StringList,
    Unknown,
}

pub struct FieldInfo {
    pub value_type: FieldType,
    pub env_vars: Vec<String>,
    pub default: Option<String>,
}

pub fn load_schema(schema_path: &Path) -> IndexMap<String, FieldInfo> {
    let src = std::fs::read_to_string(schema_path)
        .unwrap_or_else(|e| panic!("failed to read {}: {}", schema_path.display(), e));
    let doc: Value = serde_yaml::from_str(&src).unwrap_or_else(|e| panic!("failed to parse schema YAML: {}", e));
    let properties = doc
        .get("properties")
        .and_then(|v| v.as_mapping())
        .expect("schema root must have a 'properties' mapping");

    let mut entries = Vec::new();
    collect_entries(properties, &[], &mut entries);
    entries.sort_by(|a, b| a.0.cmp(&b.0));

    let mut map = IndexMap::new();
    for (yaml_path, info) in entries {
        map.insert(yaml_path, info);
    }
    map
}

fn collect_entries(mapping: &serde_yaml::Mapping, path_parts: &[&str], out: &mut Vec<(String, FieldInfo)>) {
    for (key, value) in mapping {
        let key_str = match key.as_str() {
            Some(s) => s,
            None => continue,
        };

        let mut parts = path_parts.to_vec();
        parts.push(key_str);

        let node_type = value.get("node_type").and_then(|v| v.as_str()).unwrap_or("");

        match node_type {
            "setting" => out.push(parse_setting(&parts, value)),
            "section" => {
                if let Some(props) = value.get("properties").and_then(|v| v.as_mapping()) {
                    collect_entries(props, &parts, out);
                }
            }
            _ => {
                if let Some(props) = value.get("properties").and_then(|v| v.as_mapping()) {
                    collect_entries(props, &parts, out);
                }
            }
        }
    }
}

fn parse_setting(path_parts: &[&str], value: &Value) -> (String, FieldInfo) {
    let yaml_path = path_parts.join(".");

    let has_no_env_tag = value
        .get("tags")
        .and_then(|v| v.as_sequence())
        .map(|tags| tags.iter().any(|t| t.as_str() == Some("no-env")))
        .unwrap_or(false);

    let env_vars: Vec<String> = if has_no_env_tag {
        Vec::new()
    } else {
        value
            .get("env_vars")
            .and_then(|v| v.as_sequence())
            .map(|seq| seq.iter().filter_map(|v| v.as_str()).map(|s| s.to_string()).collect())
            .unwrap_or_default()
    };

    let value_type = parse_value_type(value);
    let default = value.get("default").and_then(yaml_value_to_json_str);

    (
        yaml_path,
        FieldInfo {
            value_type,
            env_vars,
            default,
        },
    )
}

fn parse_value_type(value: &Value) -> FieldType {
    match value.get("type").and_then(|v| v.as_str()) {
        Some("string") => FieldType::String,
        Some("boolean") => FieldType::Bool,
        Some("integer") => FieldType::Integer,
        Some("number") => FieldType::Float,
        Some("array") => {
            let item_type = value.get("items").and_then(|v| v.get("type")).and_then(|v| v.as_str());
            if item_type == Some("string") {
                FieldType::StringList
            } else {
                FieldType::Unknown
            }
        }
        _ => FieldType::Unknown,
    }
}

fn yaml_value_to_json_str(value: &serde_yaml::Value) -> Option<String> {
    match value {
        serde_yaml::Value::Null => None,
        serde_yaml::Value::Bool(b) => Some(b.to_string()),
        serde_yaml::Value::Number(n) => Some(n.to_string()),
        serde_yaml::Value::String(s) => {
            let escaped = s.replace('\\', "\\\\").replace('"', "\\\"");
            Some(format!("\"{}\"", escaped))
        }
        serde_yaml::Value::Sequence(seq) => {
            let items: Option<Vec<String>> = seq.iter().map(yaml_value_to_json_str).collect();
            items.map(|elems| format!("[{}]", elems.join(",")))
        }
        serde_yaml::Value::Mapping(map) if map.is_empty() => Some("{}".to_string()),
        _ => None,
    }
}

pub fn field_type_as_rust(ft: &FieldType) -> &'static str {
    match ft {
        FieldType::String | FieldType::Unknown => "ValueType::String",
        FieldType::Bool => "ValueType::Bool",
        FieldType::Integer => "ValueType::Integer",
        FieldType::Float => "ValueType::Float",
        FieldType::StringList => "ValueType::StringList",
    }
}

pub fn is_unknown(ft: &FieldType) -> bool {
    matches!(ft, FieldType::Unknown)
}

pub fn escape_str(s: &str) -> String {
    s.replace('\\', "\\\\").replace('"', "\\\"")
}
