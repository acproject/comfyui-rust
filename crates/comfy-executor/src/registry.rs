use crate::error::ExecutorError;
use crate::execution_context::ExecutionContext;
use comfy_core::{IoType, NodeClassDef, NodeDefinition};
use serde_json::Value;
use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

pub type NodeExecuteFn = Arc<
    dyn for<'a> Fn(
            &'a ExecutionContext,
            &'a NodeDefinition,
            &'a str,
        ) -> Pin<Box<dyn Future<Output = Result<Vec<Value>, ExecutorError>> + Send + 'a>>
        + Send
        + Sync,
>;

#[derive(Clone)]
pub struct NodeRegistry {
    nodes: HashMap<String, NodeEntry>,
}

#[derive(Clone)]
struct NodeEntry {
    class_def: Arc<NodeClassDef>,
    execute_fn: NodeExecuteFn,
}

impl NodeRegistry {
    pub fn new() -> Self {
        Self {
            nodes: HashMap::new(),
        }
    }

    pub fn register(
        &mut self,
        class_def: NodeClassDef,
        execute_fn: NodeExecuteFn,
    ) {
        self.nodes.insert(
            class_def.class_type.clone(),
            NodeEntry {
                class_def: Arc::new(class_def),
                execute_fn,
            },
        );
    }

    pub fn get_class_def(&self, class_type: &str) -> Option<&NodeClassDef> {
        self.nodes.get(class_type).map(|e| e.class_def.as_ref())
    }

    pub fn get_execute_fn(&self, class_type: &str) -> Option<&NodeExecuteFn> {
        self.nodes.get(class_type).map(|e| &e.execute_fn)
    }

    pub fn has_node(&self, class_type: &str) -> bool {
        self.nodes.contains_key(class_type)
    }

    pub fn registered_types(&self) -> Vec<&str> {
        self.nodes.keys().map(|s| s.as_str()).collect()
    }

    pub fn get_all_class_defs(&self) -> Vec<(&str, &NodeClassDef)> {
        self.nodes
            .iter()
            .map(|(k, v)| (k.as_str(), v.class_def.as_ref()))
            .collect()
    }

    pub fn is_output_node(&self, class_type: &str) -> bool {
        self.nodes
            .get(class_type)
            .map(|e| e.class_def.is_output_node)
            .unwrap_or(false)
    }

    pub fn output_types(&self, class_type: &str) -> Option<&[IoType]> {
        self.nodes
            .get(class_type)
            .map(|e| e.class_def.output_types.as_slice())
    }

    pub fn output_names(&self, class_type: &str) -> Option<&[String]> {
        self.nodes
            .get(class_type)
            .map(|e| e.class_def.output_names.as_slice())
    }
}

impl Default for NodeRegistry {
    fn default() -> Self {
        Self::new()
    }
}
