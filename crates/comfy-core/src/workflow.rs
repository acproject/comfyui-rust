use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Workflow {
    #[serde(default)]
    pub id: Option<String>,
    #[serde(default)]
    pub revision: Option<u64>,
    pub last_node_id: u64,
    pub last_link_id: u64,
    pub nodes: Vec<WorkflowNode>,
    pub links: Vec<WorkflowLink>,
    pub groups: Vec<WorkflowGroup>,
    pub config: WorkflowConfig,
    pub extra: WorkflowExtra,
    pub version: f64,
    #[serde(default)]
    pub definitions: Option<WorkflowDefinitions>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct WorkflowDefinitions {
    #[serde(default)]
    pub subgraphs: Vec<WorkflowSubgraph>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowSubgraph {
    pub id: String,
    #[serde(default)]
    pub version: Option<u64>,
    #[serde(default)]
    pub state: Option<serde_json::Value>,
    #[serde(default)]
    pub revision: Option<u64>,
    #[serde(default)]
    pub config: serde_json::Value,
    pub name: String,
    #[serde(default)]
    pub input_node: Option<serde_json::Value>,
    #[serde(default)]
    pub output_node: Option<serde_json::Value>,
    #[serde(default)]
    pub inputs: Vec<WorkflowSubgraphInput>,
    #[serde(default)]
    pub outputs: Vec<WorkflowSubgraphOutput>,
    #[serde(default)]
    pub widgets: Vec<serde_json::Value>,
    #[serde(default)]
    pub nodes: Vec<WorkflowNode>,
    #[serde(default)]
    pub groups: Vec<WorkflowGroup>,
    #[serde(default)]
    pub links: Vec<WorkflowLink>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowSubgraphInput {
    pub id: String,
    pub name: String,
    #[serde(rename = "type")]
    pub input_type: String,
    #[serde(default)]
    pub link_ids: Vec<u64>,
    #[serde(default)]
    pub label: Option<String>,
    #[serde(default)]
    pub localized_name: Option<String>,
    #[serde(default)]
    pub pos: Option<[f64; 2]>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowSubgraphOutput {
    pub id: String,
    pub name: String,
    #[serde(rename = "type")]
    pub output_type: String,
    #[serde(default)]
    pub link_ids: Vec<u64>,
    #[serde(default)]
    pub localized_name: Option<String>,
    #[serde(default)]
    pub pos: Option<[f64; 2]>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowNode {
    pub id: u64,
    #[serde(rename = "type")]
    pub node_type: String,
    pub pos: [f64; 2],
    pub size: [f64; 2],
    pub flags: WorkflowNodeFlags,
    pub order: u64,
    pub mode: u64,
    #[serde(default)]
    pub inputs: Vec<WorkflowNodeInput>,
    #[serde(default)]
    pub outputs: Vec<WorkflowNodeOutput>,
    #[serde(default)]
    pub properties: serde_json::Value,
    #[serde(default)]
    pub widgets_values: Vec<serde_json::Value>,
    #[serde(default)]
    pub title: Option<String>,
    #[serde(default)]
    pub color: Option<String>,
    #[serde(default)]
    pub bgcolor: Option<String>,
    #[serde(default)]
    pub show_advanced: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowNodeFlags {
    #[serde(default)]
    pub collapsed: bool,
}

impl Default for WorkflowNodeFlags {
    fn default() -> Self {
        Self { collapsed: false }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowNodeInput {
    pub name: String,
    #[serde(rename = "type")]
    pub input_type: String,
    pub link: Option<u64>,
    #[serde(default)]
    pub widget: Option<WorkflowNodeWidget>,
    #[serde(default)]
    pub label: Option<String>,
    #[serde(default)]
    pub localized_name: Option<String>,
    #[serde(default)]
    pub shape: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowNodeOutput {
    pub name: String,
    #[serde(rename = "type")]
    pub output_type: String,
    #[serde(default)]
    pub links: Option<Vec<u64>>,
    pub slot_index: Option<u64>,
    #[serde(default)]
    pub localized_name: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowNodeWidget {
    pub name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowLink {
    pub id: u64,
    pub origin_id: i64,
    pub origin_slot: u64,
    pub target_id: i64,
    pub target_slot: u64,
    #[serde(rename = "type")]
    pub link_type: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowGroup {
    pub title: String,
    pub bounding: [f64; 4],
    pub color: Option<String>,
    #[serde(default)]
    pub id: Option<u64>,
    #[serde(default)]
    pub font_size: Option<u64>,
    #[serde(default)]
    pub flags: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct WorkflowConfig {
    #[serde(default)]
    pub extra: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct WorkflowExtra {
    #[serde(default)]
    pub ds: Option<WorkflowDs>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowDs {
    pub scale: f64,
    pub offset: [f64; 2],
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiPrompt {
    pub prompt: HashMap<String, ApiPromptNode>,
    pub extra_data: Option<serde_json::Value>,
    pub client_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiPromptNode {
    pub class_type: String,
    pub inputs: HashMap<String, serde_json::Value>,
}

impl Workflow {
    pub fn from_json(json: &str) -> Result<Self, WorkflowError> {
        serde_json::from_str(json).map_err(WorkflowError::ParseError)
    }

    pub fn to_json(&self) -> Result<String, WorkflowError> {
        serde_json::to_string_pretty(self).map_err(WorkflowError::SerializeError)
    }

    pub fn has_subgraphs(&self) -> bool {
        self.definitions
            .as_ref()
            .map(|d| !d.subgraphs.is_empty())
            .unwrap_or(false)
    }

    pub fn flatten_subgraphs(mut self) -> Self {
        if !self.has_subgraphs() {
            return self;
        }

        let definitions = self.definitions.take();
        let subgraphs = definitions
            .as_ref()
            .map(|d| d.subgraphs.clone())
            .unwrap_or_default();

        let mut all_nodes: Vec<WorkflowNode> = Vec::new();
        let mut all_links: Vec<WorkflowLink> = Vec::new();
        let mut all_groups: Vec<WorkflowGroup> = Vec::new();

        for node in &self.nodes {
            let is_subgraph = subgraphs
                .iter()
                .any(|sg| sg.id == node.node_type);

            if is_subgraph {
                if let Some(sg) = subgraphs.iter().find(|sg| sg.id == node.node_type) {
                    all_nodes.extend(sg.nodes.iter().filter(|n| n.node_type != "MarkdownNote").cloned());
                    all_links.extend(sg.links.clone());
                    all_groups.extend(sg.groups.clone());

                    for top_link in &self.links {
                        if top_link.target_id == node.id as i64 {
                            if let Some(sg_input) = sg.inputs.get(top_link.target_slot as usize) {
                                for &link_id in &sg_input.link_ids {
                                    if let Some(sg_link) = sg.links.iter().find(|l| l.id == link_id) {
                                        all_links.push(WorkflowLink {
                                            id: top_link.id,
                                            origin_id: top_link.origin_id,
                                            origin_slot: top_link.origin_slot,
                                            target_id: sg_link.target_id,
                                            target_slot: sg_link.target_slot,
                                            link_type: top_link.link_type.clone(),
                                        });
                                    }
                                }
                            }
                        }
                        if top_link.origin_id == node.id as i64 {
                            if let Some(sg_output) = sg.outputs.get(top_link.origin_slot as usize) {
                                for &link_id in &sg_output.link_ids {
                                    if let Some(sg_link) = sg.links.iter().find(|l| l.id == link_id) {
                                        all_links.push(WorkflowLink {
                                            id: top_link.id,
                                            origin_id: sg_link.origin_id,
                                            origin_slot: sg_link.origin_slot,
                                            target_id: top_link.target_id,
                                            target_slot: top_link.target_slot,
                                            link_type: top_link.link_type.clone(),
                                        });
                                    }
                                }
                            }
                        }
                    }
                }
            } else {
                all_nodes.push(node.clone());
            }
        }

        let mut seen_link_ids = std::collections::HashSet::new();
        all_links.retain(|l| seen_link_ids.insert(l.id));

        let max_node_id = all_nodes.iter().map(|n| n.id).max().unwrap_or(0);
        let max_link_id = all_links.iter().map(|l| l.id).max().unwrap_or(0);

        Workflow {
            id: self.id,
            revision: self.revision,
            last_node_id: max_node_id,
            last_link_id: max_link_id,
            nodes: all_nodes,
            links: all_links,
            groups: all_groups,
            config: self.config,
            extra: self.extra,
            version: self.version,
            definitions: None,
        }
    }

    pub fn to_api_prompt(&self) -> ApiPrompt {
        let mut prompt = HashMap::new();

        for node in &self.nodes {
            let mut inputs = HashMap::new();

            for (i, input) in node.inputs.iter().enumerate() {
                if let Some(link_id) = input.link {
                    if let Some(link) = self.links.iter().find(|l| l.id == link_id) {
                        inputs.insert(
                            input.name.clone(),
                            serde_json::json!([link.origin_id.to_string(), link.origin_slot]),
                        );
                    }
                } else if i < node.widgets_values.len() {
                    inputs.insert(input.name.clone(), node.widgets_values[i].clone());
                }
            }

            for (i, value) in node.widgets_values.iter().enumerate() {
                let widget_name = if i < node.inputs.len() {
                    continue;
                } else {
                    format!("widget_{}", i - node.inputs.len())
                };
                inputs.entry(widget_name).or_insert_with(|| value.clone());
            }

            prompt.insert(
                node.id.to_string(),
                ApiPromptNode {
                    class_type: node.node_type.clone(),
                    inputs,
                },
            );
        }

        ApiPrompt {
            prompt,
            extra_data: None,
            client_id: None,
        }
    }

    pub fn from_api_prompt(api_prompt: &ApiPrompt) -> Self {
        let mut nodes = Vec::new();
        let mut links = Vec::new();
        let mut last_node_id: u64 = 0;
        let mut link_counter: u64 = 0;

        for (node_id_str, api_node) in &api_prompt.prompt {
            let node_id: u64 = node_id_str.parse().unwrap_or(0);
            if node_id > last_node_id {
                last_node_id = node_id;
            }

            let mut inputs = Vec::new();
            let mut widgets_values = Vec::new();

            for (input_name, value) in &api_node.inputs {
                if let Some(arr) = value.as_array() {
                    if arr.len() == 2 {
                        if let Some(origin_id_str) = arr[0].as_str() {
                            if let Ok(origin_id) = origin_id_str.parse::<u64>() {
                                let origin_slot = arr[1].as_u64().unwrap_or(0);
                                link_counter += 1;
                                let link_id = link_counter;

                                links.push(WorkflowLink {
                                    id: link_id,
                                    origin_id: origin_id as i64,
                                    origin_slot,
                                    target_id: node_id as i64,
                                    target_slot: inputs.len() as u64,
                                    link_type: String::new(),
                                });

                                inputs.push(WorkflowNodeInput {
                                    name: input_name.clone(),
                                    input_type: String::new(),
                                    link: Some(link_id),
                                    widget: None,
                                    label: None,
                                    localized_name: None,
                                    shape: None,
                                });
                                continue;
                            }
                        }
                    }
                }

                widgets_values.push(value.clone());
                inputs.push(WorkflowNodeInput {
                    name: input_name.clone(),
                    input_type: String::new(),
                    link: None,
                    widget: None,
                    label: None,
                    localized_name: None,
                    shape: None,
                });
            }

            nodes.push(WorkflowNode {
                id: node_id,
                node_type: api_node.class_type.clone(),
                pos: [0.0, 0.0],
                size: [200.0, 100.0],
                flags: WorkflowNodeFlags::default(),
                order: 0,
                mode: 0,
                inputs,
                outputs: Vec::new(),
                properties: serde_json::Value::Null,
                widgets_values,
                title: None,
                color: None,
                bgcolor: None,
                show_advanced: None,
            });
        }

        Workflow {
            id: None,
            revision: None,
            last_node_id,
            last_link_id: link_counter,
            nodes,
            links,
            groups: Vec::new(),
            config: WorkflowConfig::default(),
            extra: WorkflowExtra::default(),
            version: 0.4,
            definitions: None,
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum WorkflowError {
    #[error("Failed to parse workflow JSON: {0}")]
    ParseError(serde_json::Error),
    #[error("Failed to serialize workflow: {0}")]
    SerializeError(serde_json::Error),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_workflow_roundtrip() {
        let json = r#"{
            "last_node_id": 7,
            "last_link_id": 7,
            "nodes": [
                {
                    "id": 3,
                    "type": "KSampler",
                    "pos": [100, 200],
                    "size": [300, 200],
                    "flags": {"collapsed": false},
                    "order": 3,
                    "mode": 0,
                    "inputs": [
                        {"name": "model", "type": "MODEL", "link": 1},
                        {"name": "positive", "type": "CONDITIONING", "link": 2},
                        {"name": "negative", "type": "CONDITIONING", "link": 3},
                        {"name": "latent_image", "type": "LATENT", "link": 4}
                    ],
                    "outputs": [
                        {"name": "LATENT", "type": "LATENT", "links": [5], "slot_index": 0}
                    ],
                    "properties": {},
                    "widgets_values": [8566257, 20, 8, "euler", "normal", 1]
                }
            ],
            "links": [
                [1, 4, 0, 3, 0, "MODEL"],
                [2, 6, 0, 3, 1, "CONDITIONING"],
                [3, 7, 0, 3, 2, "CONDITIONING"],
                [4, 5, 0, 3, 3, "LATENT"],
                [5, 3, 0, 8, 0, "LATENT"]
            ],
            "groups": [],
            "config": {},
            "extra": {},
            "version": 0.4
        }"#;

        let workflow = Workflow::from_json(json).unwrap();
        let output = workflow.to_json().unwrap();
        let reparsed = Workflow::from_json(&output).unwrap();

        assert_eq!(reparsed.last_node_id, 7);
        assert_eq!(reparsed.nodes.len(), 1);
        assert_eq!(reparsed.nodes[0].node_type, "KSampler");
    }

    #[test]
    fn test_subgraph_parse() {
        let json = r#"{
            "id": "test-id",
            "revision": 0,
            "last_node_id": 100,
            "last_link_id": 200,
            "nodes": [
                {
                    "id": 1,
                    "type": "LoadImage",
                    "pos": [100, 200],
                    "size": [300, 200],
                    "flags": {},
                    "order": 0,
                    "mode": 0,
                    "inputs": [],
                    "outputs": [{"name": "IMAGE", "type": "IMAGE", "links": [1]}],
                    "properties": {},
                    "widgets_values": ["test.png", "image"]
                },
                {
                    "id": 2,
                    "type": "subgraph-uuid-123",
                    "pos": [500, 200],
                    "size": [300, 200],
                    "flags": {},
                    "order": 1,
                    "mode": 0,
                    "inputs": [{"name": "input", "type": "IMAGE", "link": 1}],
                    "outputs": [{"name": "VIDEO", "type": "VIDEO", "links": [2]}],
                    "properties": {},
                    "widgets_values": []
                },
                {
                    "id": 3,
                    "type": "SaveVideo",
                    "pos": [900, 200],
                    "size": [300, 200],
                    "flags": {},
                    "order": 2,
                    "mode": 0,
                    "inputs": [{"name": "video", "type": "VIDEO", "link": 2}],
                    "outputs": [],
                    "properties": {},
                    "widgets_values": ["output", "auto", "auto"]
                }
            ],
            "links": [
                [1, 1, 0, 2, 0, "IMAGE"],
                [2, 2, 0, 3, 0, "VIDEO"]
            ],
            "groups": [],
            "config": {},
            "extra": {},
            "version": 0.4,
            "definitions": {
                "subgraphs": [
                    {
                        "id": "subgraph-uuid-123",
                        "name": "Test Subgraph",
                        "nodes": [
                            {
                                "id": 10,
                                "type": "LTXLoader",
                                "pos": [100, 100],
                                "size": [300, 200],
                                "flags": {},
                                "order": 0,
                                "mode": 0,
                                "inputs": [],
                                "outputs": [
                                    {"name": "MODEL", "type": "MODEL", "links": [10]},
                                    {"name": "CLIP", "type": "CLIP", "links": [11]},
                                    {"name": "VAE", "type": "VAE", "links": [12]}
                                ],
                                "properties": {},
                                "widgets_values": ["model.safetensors"]
                            },
                            {
                                "id": 11,
                                "type": "CLIPTextEncode",
                                "pos": [500, 100],
                                "size": [300, 200],
                                "flags": {},
                                "order": 1,
                                "mode": 0,
                                "inputs": [{"name": "clip", "type": "CLIP", "link": 11}],
                                "outputs": [{"name": "CONDITIONING", "type": "CONDITIONING", "links": [13]}],
                                "properties": {},
                                "widgets_values": ["test prompt"]
                            }
                        ],
                        "links": [
                            [10, 10, 0, 12, 0, "MODEL"],
                            [11, 10, 1, 11, 0, "CLIP"],
                            [13, 11, 0, 12, 1, "CONDITIONING"]
                        ],
                        "groups": [],
                        "inputs": [
                            {"id": "in-1", "name": "input", "type": "IMAGE", "linkIds": []}
                        ],
                        "outputs": [
                            {"id": "out-1", "name": "VIDEO", "type": "VIDEO", "linkIds": []}
                        ]
                    }
                ]
            }
        }"#;

        let workflow = Workflow::from_json(json).unwrap();
        assert!(workflow.has_subgraphs());
        assert_eq!(workflow.id, Some("test-id".to_string()));
        assert_eq!(workflow.nodes.len(), 3);

        let definitions = workflow.definitions.as_ref().unwrap();
        assert_eq!(definitions.subgraphs.len(), 1);
        assert_eq!(definitions.subgraphs[0].name, "Test Subgraph");
        assert_eq!(definitions.subgraphs[0].nodes.len(), 2);

        let flattened = workflow.flatten_subgraphs();
        assert!(!flattened.has_subgraphs());
        assert!(flattened.nodes.len() >= 2);
        assert!(flattened.nodes.iter().any(|n| n.node_type == "LTXLoader"));
        assert!(flattened.nodes.iter().any(|n| n.node_type == "CLIPTextEncode"));
        assert!(flattened.nodes.iter().any(|n| n.node_type == "LoadImage"));
        assert!(flattened.nodes.iter().any(|n| n.node_type == "SaveVideo"));
    }

    #[test]
    fn test_parse_ltx_i2v_workflow() {
        let json = std::fs::read_to_string("../../workflows/video_ltx2_3_i2v.json")
            .expect("Failed to read i2v workflow file");
        let workflow = Workflow::from_json(&json).expect("Failed to parse i2v workflow");
        assert!(workflow.has_subgraphs());
        assert_eq!(workflow.nodes.len(), 4);
        let definitions = workflow.definitions.as_ref().unwrap();
        assert_eq!(definitions.subgraphs.len(), 1);
        assert_eq!(definitions.subgraphs[0].name, "Image to Video (LTX-2.3)");
        assert_eq!(definitions.subgraphs[0].nodes.len(), 45);
        assert!(!definitions.subgraphs[0].inputs.is_empty());
        assert!(!definitions.subgraphs[0].outputs.is_empty());

        let flattened = workflow.flatten_subgraphs();
        assert!(!flattened.has_subgraphs());
        assert!(flattened.nodes.len() > 4);
        assert!(flattened.nodes.iter().any(|n| n.node_type == "RandomNoise"));
        assert!(flattened.nodes.iter().any(|n| n.node_type == "KSamplerSelect"));
        assert!(flattened.nodes.iter().any(|n| n.node_type == "SamplerCustomAdvanced"));
    }

    #[test]
    fn test_parse_ltx_ia2v_workflow() {
        let json = std::fs::read_to_string("../../workflows/video_ltx2_3_ia2v.json")
            .expect("Failed to read ia2v workflow file");
        let workflow = Workflow::from_json(&json).expect("Failed to parse ia2v workflow");
        assert!(workflow.has_subgraphs());
        assert_eq!(workflow.nodes.len(), 6);
        let definitions = workflow.definitions.as_ref().unwrap();
        assert_eq!(definitions.subgraphs.len(), 1);
        assert_eq!(definitions.subgraphs[0].name, "Video Generation (LTX-2.3)");
        assert_eq!(definitions.subgraphs[0].nodes.len(), 48);

        let flattened = workflow.flatten_subgraphs();
        assert!(!flattened.has_subgraphs());
        assert!(flattened.nodes.len() > 6);
        assert!(flattened.nodes.iter().any(|n| n.node_type == "RandomNoise"));
        assert!(flattened.nodes.iter().any(|n| n.node_type == "LTXVAudioVAELoader"));
    }
}
