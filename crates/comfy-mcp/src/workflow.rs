//! Deterministic workflow (prompt graph) construction and static validation.
//!
//! The /prompt API expects a map:
//! ```json
//! {
//!   "<node_id>": {"class_type": "KSampler", "inputs": {"seed": 1, "model": ["3", 0]}}
//! }
//! ```
//! Links are `[source_node_id, source_output_index]`. These helpers turn
//! high-level node/link specifications (or a built-in template) into such a
//! graph and validate it against the server's `/object_info` registry without
//! submitting anything, giving an AI IDE full control over the flow.

use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use std::collections::{BTreeMap, HashMap};

/// One explicit node specification.
#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
pub struct NodeSpec {
    /// Graph-unique node id (string, e.g. "10").
    pub id: String,
    /// Registered node class (must exist in `/object_info`).
    pub class_type: String,
    /// Literal input values (links are added via [`LinkSpec`]).
    #[serde(default)]
    pub inputs: Map<String, Value>,
    /// Optional display title (stored under `_meta.title`).
    #[serde(default)]
    pub title: Option<String>,
}

/// One connection between two nodes.
#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
pub struct LinkSpec {
    pub from_node: String,
    /// Source output socket index (0-based, as shown in `/object_info`).
    pub from_socket: u32,
    pub to_node: String,
    /// Target input name.
    pub to_input: String,
}

/// Literal input overrides: node id -> input name -> value.
/// (Uses `HashMap` rather than nested `serde_json::Map`, whose helper-trait
/// impls only cover `Map<String, Value>`.)
pub type Overrides = HashMap<String, HashMap<String, Value>>;

/// A validation finding.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationIssue {
    pub severity: String, // "error" | "warning"
    pub node_id: Option<String>,
    pub input: Option<String>,
    pub message: String,
}

/// Result of statically validating a prompt graph.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationReport {
    pub valid: bool,
    pub error_count: usize,
    pub warning_count: usize,
    pub issues: Vec<ValidationIssue>,
    pub node_count: usize,
    pub output_nodes: Vec<String>,
}

impl ValidationReport {
    fn new() -> Self {
        Self {
            valid: true,
            error_count: 0,
            warning_count: 0,
            issues: Vec::new(),
            node_count: 0,
            output_nodes: Vec::new(),
        }
    }

    fn error(&mut self, node_id: Option<String>, input: Option<String>, message: impl Into<String>) {
        self.valid = false;
        self.error_count += 1;
        self.issues.push(ValidationIssue {
            severity: "error".to_string(),
            node_id,
            input,
            message: message.into(),
        });
    }

    fn warning(&mut self, node_id: Option<String>, input: Option<String>, message: impl Into<String>) {
        self.warning_count += 1;
        self.issues.push(ValidationIssue {
            severity: "warning".to_string(),
            node_id,
            input,
            message: message.into(),
        });
    }
}

/// Assemble a prompt graph from explicit nodes and links.
///
/// Links overwrite any literal value set for the same target input. Returns an
/// error for duplicate node ids, self-loops or links referencing unknown
/// nodes/sockets (socket bounds need the registry — those are caught by
/// [`validate_graph`]; this function checks structural references only).
pub fn assemble_graph(nodes: &[NodeSpec], links: &[LinkSpec]) -> Result<Value, String> {
    let mut graph = Map::new();

    for node in nodes {
        if node.id.trim().is_empty() {
            return Err("node id must not be empty".to_string());
        }
        if graph.contains_key(&node.id) {
            return Err(format!("duplicate node id: {}", node.id));
        }
        let mut entry = Map::new();
        entry.insert("class_type".to_string(), Value::String(node.class_type.clone()));
        entry.insert("inputs".to_string(), Value::Object(node.inputs.clone()));
        if let Some(title) = &node.title {
            entry.insert(
                "_meta".to_string(),
                serde_json::json!({ "title": title }),
            );
        }
        graph.insert(node.id.clone(), Value::Object(entry));
    }

    for link in links {
        if !graph.contains_key(&link.to_node) {
            return Err(format!(
                "link target node '{}' does not exist",
                link.to_node
            ));
        }
        if !graph.contains_key(&link.from_node) {
            return Err(format!(
                "link source node '{}' does not exist",
                link.from_node
            ));
        }
        if link.from_node == link.to_node {
            return Err(format!(
                "self-loop on node '{}' (input '{}') is not allowed",
                link.to_node, link.to_input
            ));
        }
        let target = graph.get_mut(&link.to_node).expect("checked above");
        let inputs = target
            .get_mut("inputs")
            .and_then(|v| v.as_object_mut())
            .expect("inputs object");
        inputs.insert(
            link.to_input.clone(),
            Value::Array(vec![
                Value::String(link.from_node.clone()),
                Value::from(link.from_socket),
            ]),
        );
    }

    Ok(Value::Object(graph))
}

/// Convert a built-in template (as returned by `/workflow_templates/{id}`)
/// into a prompt graph. `object_info` is used to resolve named source handles
/// (e.g. `"MODEL"`) into output socket indices.
pub fn template_to_graph(template: &Value, object_info: &Value) -> Result<Value, String> {
    let tnodes = template
        .get("nodes")
        .and_then(|v| v.as_array())
        .ok_or_else(|| "template has no 'nodes' array".to_string())?;

    let mut nodes: Vec<NodeSpec> = Vec::new();
    for tn in tnodes {
        let id = tn
            .get("id")
            .and_then(|v| v.as_str())
            .ok_or_else(|| "template node missing string 'id'".to_string())?
            .to_string();
        let class_type = tn
            .get("class_type")
            .and_then(|v| v.as_str())
            .ok_or_else(|| format!("template node {id} missing 'class_type'"))?
            .to_string();
        let inputs = match tn.get("inputs") {
            Some(Value::Object(m)) => m.clone(),
            _ => Map::new(),
        };
        let title = tn
            .get("title")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());
        nodes.push(NodeSpec {
            id,
            class_type,
            inputs,
            title,
        });
    }

    let mut links: Vec<LinkSpec> = Vec::new();
    if let Some(conns) = template.get("connections").and_then(|v| v.as_array()) {
        for c in conns {
            let source = c
                .get("source")
                .and_then(|v| v.as_str())
                .ok_or_else(|| "connection missing 'source'".to_string())?;
            let target = c
                .get("target")
                .and_then(|v| v.as_str())
                .ok_or_else(|| "connection missing 'target'".to_string())?;
            let target_handle = c
                .get("target_handle")
                .and_then(|v| v.as_str())
                .ok_or_else(|| "connection missing 'target_handle'".to_string())?;
            let source_handle = c
                .get("source_handle")
                .and_then(|v| v.as_str())
                .ok_or_else(|| "connection missing 'source_handle'".to_string())?;

            // source_handle may be an output name ("MODEL") or a numeric index.
            let socket = source_handle
                .parse::<u32>()
                .ok()
                .or_else(|| {
                    let source_class = nodes.iter().find(|n| n.id == source).map(|n| n.class_type.as_str());
                    source_class.and_then(|ct| output_index_of(object_info, ct, source_handle))
                })
                .ok_or_else(|| {
                    format!(
                        "cannot resolve source output '{source_handle}' of node '{source}' \
                         (not a numeric index and not found in output_names)"
                    )
                })?;
            links.push(LinkSpec {
                from_node: source.to_string(),
                from_socket: socket,
                to_node: target.to_string(),
                to_input: target_handle.to_string(),
            });
        }
    }

    assemble_graph(&nodes, &links)
}

/// Apply literal input overrides (`node_id -> input_name -> value`) to a graph.
pub fn apply_overrides(graph: &mut Value, overrides: &Overrides) {
    let Some(g) = graph.as_object_mut() else { return };
    for (node_id, patch) in overrides {
        if let Some(node) = g.get_mut(node_id) {
            if let Some(inputs) = node.get_mut("inputs").and_then(|v| v.as_object_mut()) {
                for (k, v) in patch {
                    inputs.insert(k.clone(), v.clone());
                }
            }
        }
    }
}

/// Resolve an output name to its socket index for a class.
fn output_index_of(object_info: &Value, class_type: &str, output_name: &str) -> Option<u32> {
    let def = object_info.get(class_type)?;
    def.get("output_names")
        .and_then(|v| v.as_array())?
        .iter()
        .position(|n| n.as_str() == Some(output_name))
        .map(|i| i as u32)
}

/// Statically validate a prompt graph against the `/object_info` registry.
pub fn validate_graph(graph: &Value, object_info: &Value) -> ValidationReport {
    let mut report = ValidationReport::new();
    let Some(nodes) = graph.as_object() else {
        report.error(None, None, "prompt graph must be a JSON object of node_id -> node");
        return report;
    };
    report.node_count = nodes.len();

    // Pass 1: class existence + per-node inputs.
    for (node_id, node) in nodes {
        let Some(class_type) = node.get("class_type").and_then(|v| v.as_str()) else {
            report.error(Some(node_id.clone()), None, "node missing string 'class_type'");
            continue;
        };
        let Some(def) = object_info.get(class_type) else {
            report.error(
                Some(node_id.clone()),
                None,
                format!("unknown node class_type '{class_type}' (not found in /object_info)"),
            );
            continue;
        };
        if def.get("is_output_node").and_then(|v| v.as_bool()) == Some(true) {
            report.output_nodes.push(node_id.clone());
        }

        let inputs = node.get("inputs").and_then(|v| v.as_object());
        let required = def
            .pointer("/input_types/required")
            .and_then(|v| v.as_object());
        let optional = def
            .pointer("/input_types/optional")
            .and_then(|v| v.as_object());

        // Required inputs present (either literal or link).
        if let Some(required) = required {
            for (name, spec) in required {
                let present = inputs.map(|m| m.contains_key(name)).unwrap_or(false);
                if !present && spec.get("default").is_none() {
                    report.error(
                        Some(node_id.clone()),
                        Some(name.clone()),
                        format!("missing required input '{name}'"),
                    );
                }
            }
        }

        // Unknown / type / COMBO checks on provided literals.
        if let Some(inputs) = inputs {
            for (name, value) in inputs {
                let spec = required
                    .and_then(|m| m.get(name))
                    .or_else(|| optional.and_then(|m| m.get(name)));
                let Some(spec) = spec else {
                    report.warning(
                        Some(node_id.clone()),
                        Some(name.clone()),
                        format!("input '{name}' is not declared by {class_type}"),
                    );
                    continue;
                };
                let type_name = spec.get("type_name").and_then(|v| v.as_str()).unwrap_or("");
                // Links are always arrays ["node", socket].
                if value.is_array() && value.as_array().map(|a| a.len() == 2).unwrap_or(false) {
                    continue;
                }
                if type_name == "COMBO" {
                    if let Some(choices) = spec.pointer("/extra/choices").and_then(|v| v.as_array()) {
                        let ok = choices.iter().any(|c| c == value);
                        if !ok && !choices.is_empty() {
                            report.warning(
                                Some(node_id.clone()),
                                Some(name.clone()),
                                format!(
                                    "value {value} is not one of the COMBO choices: {}",
                                    choices
                                        .iter()
                                        .take(12)
                                        .map(|c| c.to_string())
                                        .collect::<Vec<_>>()
                                        .join(", ")
                                ),
                            );
                        }
                    }
                } else {
                    check_literal_type(&mut report, node_id, name, type_name, value);
                }
            }
        }
    }

    // Pass 2: link integrity.
    for (node_id, node) in nodes {
        let class_type = node
            .get("class_type")
            .and_then(|v| v.as_str())
            .unwrap_or_default();
        let Some(def) = object_info.get(class_type) else { continue };
        let Some(inputs) = node.get("inputs").and_then(|v| v.as_object()) else {
            continue;
        };
        let required = def
            .pointer("/input_types/required")
            .and_then(|v| v.as_object());
        let optional = def
            .pointer("/input_types/optional")
            .and_then(|v| v.as_object());

        for (name, value) in inputs {
            let Some(link) = as_link(value) else { continue };
            let (src_id, src_socket) = link;
            let Some(src_node) = nodes.get(&src_id) else {
                report.error(
                    Some(node_id.clone()),
                    Some(name.clone()),
                    format!("link references unknown source node '{src_id}'"),
                );
                continue;
            };
            let src_class = src_node
                .get("class_type")
 .and_then(|v| v.as_str())
                .unwrap_or_default();
            if let Some(src_def) = object_info.get(src_class) {
                let out_count = src_def
                    .get("output_types")
                    .and_then(|v| v.as_array())
                    .map(|a| a.len())
                    .unwrap_or(0);
                if src_socket as usize >= out_count {
                    report.error(
                        Some(node_id.clone()),
                        Some(name.clone()),
                        format!(
                            "link source '{src_id}' output socket {src_socket} is out of range \
                             ({src_class} has {out_count} outputs)"
                        ),
                    );
                }
            }
            let declared = required
                .and_then(|m| m.get(name))
                .or_else(|| optional.and_then(|m| m.get(name)))
                .is_some();
            if !declared {
                report.warning(
                    Some(node_id.clone()),
                    Some(name.clone()),
                    format!("link targets undeclared input '{name}'"),
                );
            }
        }
    }

    if report.output_nodes.is_empty() {
        report.warning(
            None,
            None,
            "graph contains no output node (e.g. SaveImage/SaveVideo/SaveAudio); \
             it will run but produce no saved artifact",
        );
    }
    report.output_nodes.sort();
    report
}

/// Interpret a JSON value as a `[source_node, socket_index]` link.
fn as_link(value: &Value) -> Option<(String, u32)> {
    let arr = value.as_array()?;
    if arr.len() != 2 {
        return None;
    }
    let id = arr[0].as_str()?.to_string();
    let socket = arr[1].as_u64()? as u32;
    Some((id, socket))
}

fn check_literal_type(
    report: &mut ValidationReport,
    node_id: &str,
    input: &str,
    type_name: &str,
    value: &Value,
) {
    let ok = match type_name {
        "STRING" => value.is_string(),
        "INT" => value.is_i64() || value.is_u64() || value.is_f64(),
        "FLOAT" => value.is_number(),
        "BOOLEAN" => value.is_boolean(),
        _ => true, // structural types (IMAGE/MODEL/...) normally arrive via links
    };
    if !ok {
        report.warning(
            Some(node_id.to_string()),
            Some(input.to_string()),
            format!("literal value {value} does not match declared type {type_name}"),
        );
    }
}

/// Compact summary of the registry for `list_nodes`: class + category +
/// display name, optionally filtered by category prefix / keyword.
pub fn summarize_object_info(
    object_info: &Value,
    category: Option<&str>,
    keyword: Option<&str>,
) -> Vec<Value> {
    let mut rows: Vec<(String, String, String)> = Vec::new();
    if let Some(map) = object_info.as_object() {
        for (class_type, def) in map {
            let cat = def
                .get("category")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            let display = def
                .get("display_name")
                .and_then(|v| v.as_str())
                .unwrap_or(class_type)
                .to_string();
            rows.push((class_type.clone(), cat, display));
        }
    }
    rows.sort_by(|a, b| a.1.cmp(&b.1).then_with(|| a.0.cmp(&b.0)));

    let kw = keyword.map(|s| s.to_lowercase());
    rows.into_iter()
        .filter(|(class_type, cat, disp)| {
            category.map(|c| cat.starts_with(c)).unwrap_or(true)
                && match &kw {
                    Some(k) => {
                        disp.to_lowercase().contains(k)
                            || class_type.to_lowercase().contains(k)
                            || k.is_empty()
                    }
                    None => true,
                }
        })
        .map(|(class_type, cat, display)| {
            serde_json::json!({
                "class_type": class_type,
                "category": cat,
                "display_name": display,
            })
        })
        .collect()
}

/// Count node classes per category (handy overview for the IDE).
pub fn category_counts(object_info: &Value) -> BTreeMap<String, usize> {
    let mut counts = BTreeMap::new();
    if let Some(map) = object_info.as_object() {
        for def in map.values() {
            let cat = def
                .get("category")
                .and_then(|v| v.as_str())
                .unwrap_or("(uncategorized)")
                .to_string();
            *counts.entry(cat).or_insert(0) += 1;
        }
    }
    counts
}
