//! Schema tree: XML-style (attributes/children) and TOML-style (map as children). Shared walk and prefixes.

use serde_json::{Map, Value};

use super::consts::{SchemaKeys, SectionKeys, tree_prefixes};
use super::format;
use super::sections::{Section, SingleColumnListSection};

/// Emit lines for one schema node. Uses `walk_schema_children` for both XML and TOML children so the recursion and tree prefixes live in one place.
fn schema_node_lines(
    value: &Value,
    line_prefix: &str,
    continuation: &str,
    label: &str,
) -> Vec<String> {
    let mut out = Vec::new();
    let Value::Object(map) = value else {
        out.push(format::prefixed_label_with_value(
            line_prefix,
            label,
            &format::value_to_string(value, format::DEFAULT_MAX_ARRAY_INLINE),
        ));
        return out;
    };

    if map.is_empty() {
        out.push(format::prefixed_label(line_prefix, label));
        return out;
    }

    out.push(format::prefixed_label(line_prefix, label));

    if SchemaKeys::has_children_or_attributes(map) {
        let children = map.get(SchemaKeys::CHILDREN).and_then(Value::as_object);
        let has_children = children.is_some_and(|c| !c.is_empty());
        if let Some(attrs) = map.get(SchemaKeys::ATTRIBUTES).and_then(Value::as_object)
            && !attrs.is_empty()
        {
            let n = attrs.len();
            for (i, (k, v)) in attrs.iter().enumerate() {
                let is_last = i == n.saturating_sub(1) && !has_children;
                let (branch_prefix, _) = tree_prefixes(continuation, is_last);
                out.push(format!(
                    "{} {}: {}",
                    branch_prefix,
                    format::format_key(k),
                    format::value_to_string(v, format::DEFAULT_MAX_ARRAY_INLINE)
                ));
            }
        }
        if let Some(children_map) = children.filter(|c| !c.is_empty()) {
            out.extend(walk_schema_children(
                children_map,
                continuation,
                schema_node_lines,
            ));
        }
        return out;
    }

    // TOML-style: each key is a child node or leaf; same visitor as XML children
    out.extend(walk_schema_children(map, continuation, schema_node_lines));
    out
}

/// Shared child walk: enumerate entries, compute tree prefixes, call visitor for each. Used for XML children (object) and TOML children (full map).
fn walk_schema_children<F>(
    map: &Map<String, Value>,
    continuation: &str,
    mut visit: F,
) -> Vec<String>
where
    F: FnMut(&Value, &str, &str, &str) -> Vec<String>,
{
    let n = map.len();
    map.iter()
        .enumerate()
        .flat_map(|(i, (name, val))| {
            let (child_line, child_cont) = tree_prefixes(continuation, i == n.saturating_sub(1));
            visit(val, &child_line, &child_cont, name)
        })
        .collect()
}

fn schema_value_to_list(value: &Value) -> Vec<String> {
    let lines = match value {
        Value::Object(map) if !map.is_empty() => {
            let mut lines = Vec::new();
            for (idx, (name, node_val)) in map.iter().enumerate() {
                if idx > 0 {
                    lines.push(String::new());
                }
                lines.extend(schema_node_lines(node_val, "", "", name));
            }
            lines
        }
        Value::Array(arr) if !arr.is_empty() => arr
            .iter()
            .flat_map(|v| {
                let label = v
                    .as_object()
                    .and_then(|o| {
                        o.get("name")
                            .or_else(|| o.get("type"))
                            .or_else(|| o.get("id"))
                            .and_then(Value::as_str)
                    })
                    .unwrap_or("…");
                schema_node_lines(v, "", "", label)
            })
            .collect::<Vec<_>>(),
        _ => schema_node_lines(value, "", "", "…"),
    };
    if lines.is_empty() && !value.is_null() {
        vec!["—".to_string()]
    } else {
        lines
    }
}

/// Push a single schema tree section (`SingleColumnList`) onto `sections`.
pub fn push_schema_section(sections: &mut Vec<Section>, value: &Value) {
    let mut lines = schema_value_to_list(value);
    if lines.is_empty() {
        lines.push("—".to_string());
    }
    sections.push(Section::SingleColumnList(SingleColumnListSection {
        title: format::format_key(SectionKeys::SCHEMA),
        values: lines,
    }));
}
