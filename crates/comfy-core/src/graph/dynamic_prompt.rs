use crate::graph::edge::is_link;
use crate::graph::node::NodeDefinition;
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct DynamicPrompt {
    original_prompt: HashMap<String, NodeDefinition>,
    ephemeral_prompt: HashMap<String, NodeDefinition>,
    ephemeral_parents: HashMap<String, String>,
    ephemeral_display: HashMap<String, String>,
}

impl DynamicPrompt {
    pub fn new(original_prompt: HashMap<String, NodeDefinition>) -> Self {
        Self {
            original_prompt,
            ephemeral_prompt: HashMap::new(),
            ephemeral_parents: HashMap::new(),
            ephemeral_display: HashMap::new(),
        }
    }

    pub fn get_node(&self, node_id: &str) -> Result<&NodeDefinition, NodeNotFoundError> {
        if let Some(node) = self.ephemeral_prompt.get(node_id) {
            return Ok(node);
        }
        if let Some(node) = self.original_prompt.get(node_id) {
            return Ok(node);
        }
        Err(NodeNotFoundError {
            node_id: node_id.to_string(),
        })
    }

    pub fn has_node(&self, node_id: &str) -> bool {
        self.original_prompt.contains_key(node_id) || self.ephemeral_prompt.contains_key(node_id)
    }

    pub fn add_ephemeral_node(
        &mut self,
        node_id: impl Into<String>,
        node_info: NodeDefinition,
        parent_id: impl Into<String>,
        display_id: impl Into<String>,
    ) {
        let node_id = node_id.into();
        let parent_id = parent_id.into();
        let display_id = display_id.into();
        self.ephemeral_prompt.insert(node_id.clone(), node_info);
        self.ephemeral_parents.insert(node_id.clone(), parent_id);
        self.ephemeral_display.insert(node_id, display_id);
    }

    pub fn get_real_node_id<'a>(&'a self, node_id: &'a str) -> &'a str {
        let mut current = node_id;
        while let Some(parent) = self.ephemeral_parents.get(current) {
            current = parent;
        }
        current
    }

    pub fn get_parent_node_id(&self, node_id: &str) -> Option<&str> {
        self.ephemeral_parents.get(node_id).map(|s| s.as_str())
    }

    pub fn get_display_node_id<'a>(&'a self, node_id: &'a str) -> &'a str {
        let mut current = node_id;
        while let Some(display) = self.ephemeral_display.get(current) {
            current = display;
        }
        current
    }

    pub fn all_node_ids(&self) -> std::collections::HashSet<&str> {
        let mut ids: std::collections::HashSet<&str> = self.original_prompt.keys().map(|s| s.as_str()).collect();
        for id in self.ephemeral_prompt.keys() {
            ids.insert(id.as_str());
        }
        ids
    }

    pub fn original_node_ids(&self) -> impl Iterator<Item = &str> {
        self.original_prompt.keys().map(|s| s.as_str())
    }

    pub fn get_original_prompt(&self) -> &HashMap<String, NodeDefinition> {
        &self.original_prompt
    }

    pub fn get_input_links(&self, node_id: &str) -> Vec<(String, String, usize)> {
        let node = match self.get_node(node_id) {
            Ok(n) => n,
            Err(_) => return vec![],
        };
        let mut links = Vec::new();
        for (input_name, value) in &node.inputs {
            if is_link(value) {
                if let Some(arr) = value.as_array() {
                    if arr.len() == 2 {
                        if let (Some(from_node), Some(from_socket)) = (arr[0].as_str(), arr[1].as_u64()) {
                            links.push((input_name.clone(), from_node.to_string(), from_socket as usize));
                        }
                    }
                }
            }
        }
        links
    }
}

#[derive(Debug, thiserror::Error)]
#[error("Node not found: {node_id}")]
pub struct NodeNotFoundError {
    pub node_id: String,
}
