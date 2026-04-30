use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Workflow {
    pub last_node_id: u64,
    pub last_link_id: u64,
    pub nodes: Vec<WorkflowNode>,
    pub links: Vec<WorkflowLink>,
    pub groups: Vec<WorkflowGroup>,
    pub config: WorkflowConfig,
    pub extra: WorkflowExtra,
    pub version: f64,
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
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowNodeOutput {
    pub name: String,
    #[serde(rename = "type")]
    pub output_type: String,
    #[serde(default)]
    pub links: Option<Vec<u64>>,
    pub slot_index: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowNodeWidget {
    pub name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowLink {
    pub id: u64,
    pub origin_id: u64,
    pub origin_slot: u64,
    pub target_id: u64,
    pub target_slot: u64,
    #[serde(rename = "type")]
    pub link_type: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowGroup {
    pub title: String,
    pub bounding: [f64; 4],
    pub color: Option<String>,
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
                                    origin_id,
                                    origin_slot,
                                    target_id: node_id,
                                    target_slot: inputs.len() as u64,
                                    link_type: String::new(),
                                });

                                inputs.push(WorkflowNodeInput {
                                    name: input_name.clone(),
                                    input_type: String::new(),
                                    link: Some(link_id),
                                    widget: None,
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
            });
        }

        Workflow {
            last_node_id,
            last_link_id: link_counter,
            nodes,
            links,
            groups: Vec::new(),
            config: WorkflowConfig::default(),
            extra: WorkflowExtra::default(),
            version: 0.4,
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
    fn test_api_prompt_conversion() {
        let mut prompt_nodes = HashMap::new();
        let mut inputs = HashMap::new();
        inputs.insert("ckpt_name".to_string(), serde_json::json!("model.safetensors"));
        inputs.insert(
            "model".to_string(),
            serde_json::json!(["4", 0]),
        );

        prompt_nodes.insert(
            "3".to_string(),
            ApiPromptNode {
                class_type: "CheckpointLoaderSimple".to_string(),
                inputs,
            },
        );

        let api_prompt = ApiPrompt {
            prompt: prompt_nodes,
            extra_data: None,
            client_id: None,
        };

        let workflow = Workflow::from_api_prompt(&api_prompt);
        assert_eq!(workflow.nodes.len(), 1);
        assert_eq!(workflow.nodes[0].node_type, "CheckpointLoaderSimple");

        let back_to_api = workflow.to_api_prompt();
        assert_eq!(back_to_api.prompt.len(), 1);
    }
}
