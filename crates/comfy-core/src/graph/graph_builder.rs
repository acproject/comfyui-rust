use crate::graph::edge::is_link;
use crate::graph::node::NodeDefinition;
use std::collections::HashMap;

pub struct GraphBuilder {
    prefix: String,
    nodes: HashMap<String, BuilderNode>,
    id_gen: usize,
}

pub struct BuilderNode {
    pub class_type: String,
    pub inputs: HashMap<String, serde_json::Value>,
    #[allow(dead_code)]
    pub override_display_id: Option<String>,
}

impl GraphBuilder {
    pub fn new(prefix: impl Into<String>) -> Self {
        Self {
            prefix: prefix.into(),
            nodes: HashMap::new(),
            id_gen: 1,
        }
    }

    pub fn node(&mut self, class_type: impl Into<String>, id: Option<&str>, inputs: HashMap<String, serde_json::Value>) -> String {
        let id = match id {
            Some(id) => format!("{}{}", self.prefix, id),
            None => {
                let id = format!("{}{}", self.prefix, self.id_gen);
                self.id_gen += 1;
                id
            }
        };

        if !self.nodes.contains_key(&id) {
            self.nodes.insert(
                id.clone(),
                BuilderNode {
                    class_type: class_type.into(),
                    inputs,
                    override_display_id: None,
                },
            );
        }

        id
    }

    pub fn lookup_node(&self, id: &str) -> Option<&BuilderNode> {
        let full_id = format!("{}{}", self.prefix, id);
        self.nodes.get(&full_id)
    }

    pub fn finalize(&self) -> HashMap<String, NodeDefinition> {
        let mut output = HashMap::new();
        for (node_id, node) in &self.nodes {
            output.insert(
                node_id.clone(),
                NodeDefinition {
                    class_type: node.class_type.clone(),
                    inputs: node.inputs.clone(),
                    is_changed: None,
                },
            );
        }
        output
    }

    pub fn replace_node_output(&mut self, node_id: &str, index: usize, new_value: Option<serde_json::Value>) {
        let full_id = format!("{}{}", self.prefix, node_id);
        let mut to_remove = Vec::new();

        for (nid, node) in &mut self.nodes {
            let keys_to_update: Vec<String> = node.inputs.keys().cloned().collect();
            for key in keys_to_update {
                if let Some(value) = node.inputs.get(&key) {
                    if is_link(value) {
                        if let Some(arr) = value.as_array() {
                            if arr.len() == 2 {
                                if let (Some(from_node), Some(from_socket)) =
                                    (arr[0].as_str(), arr[1].as_u64())
                                {
                                    if from_node == full_id && from_socket as usize == index {
                                        match &new_value {
                                            Some(v) => {
                                                node.inputs.insert(key, v.clone());
                                            }
                                            None => {
                                                to_remove.push((nid.clone(), key.clone()));
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        for (nid, key) in to_remove {
            if let Some(node) = self.nodes.get_mut(&nid) {
                node.inputs.remove(&key);
            }
        }
    }

    pub fn remove_node(&mut self, id: &str) {
        let full_id = format!("{}{}", self.prefix, id);
        self.nodes.remove(&full_id);
    }
}

pub struct Node {
    pub id: String,
    pub class_type: String,
    pub inputs: HashMap<String, serde_json::Value>,
}

impl Node {
    pub fn out(&self, index: usize) -> serde_json::Value {
        serde_json::json!([self.id, index])
    }

    pub fn set_input(&mut self, key: impl Into<String>, value: Option<serde_json::Value>) {
        let key = key.into();
        match value {
            Some(v) => {
                self.inputs.insert(key, v);
            }
            None => {
                self.inputs.remove(&key);
            }
        }
    }

    pub fn get_input(&self, key: &str) -> Option<&serde_json::Value> {
        self.inputs.get(key)
    }
}
