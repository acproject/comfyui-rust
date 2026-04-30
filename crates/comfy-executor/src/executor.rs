use crate::error::{ExecutorError, ValidationResult, ErrorDetail, NodeErrorInfo};
use crate::execution_context::{ExecutionContext, NodeOutput};
use crate::registry::NodeRegistry;
use comfy_core::{CacheEntry, CacheEntryValue, CacheView, DynamicPrompt, ExecutionList, NullCacheView};
use comfy_inference::InferenceBackend;
use serde_json::Value;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;

pub struct Executor {
    registry: Arc<NodeRegistry>,
    backend: Arc<dyn InferenceBackend>,
    cache: Arc<dyn CacheView>,
}

impl Executor {
    pub fn new(
        registry: NodeRegistry,
        backend: Arc<dyn InferenceBackend>,
    ) -> Self {
        Self {
            registry: Arc::new(registry),
            backend,
            cache: Arc::new(NullCacheView),
        }
    }

    pub fn with_cache(mut self, cache: Arc<dyn CacheView>) -> Self {
        self.cache = cache;
        self
    }

    pub fn registry(&self) -> &NodeRegistry {
        &self.registry
    }

    pub fn validate_prompt(
        &self,
        prompt: &HashMap<String, Value>,
    ) -> ValidationResult {
        let mut node_errors: HashMap<String, NodeErrorInfo> = HashMap::new();
        let mut output_nodes = Vec::new();

        for (node_id, node_data) in prompt {
            let class_type = match node_data.get("class_type").and_then(|v| v.as_str()) {
                Some(ct) => ct,
                None => {
                    let info = NodeErrorInfo {
                        node_id: node_id.clone(),
                        class_type: String::new(),
                        errors: vec![ErrorDetail {
                            error_type: "missing_node_type".to_string(),
                            message: "Node has no class_type".to_string(),
                            details: format!("Node ID '{}'", node_id),
                        }],
                    };
                    node_errors.insert(node_id.clone(), info);
                    continue;
                }
            };

            if !self.registry.has_node(class_type) {
                let info = NodeErrorInfo {
                    node_id: node_id.clone(),
                    class_type: class_type.to_string(),
                    errors: vec![ErrorDetail {
                        error_type: "missing_node_type".to_string(),
                        message: format!("Node type '{}' not found", class_type),
                        details: format!("Node ID '{}'", node_id),
                    }],
                };
                node_errors.insert(node_id.clone(), info);
                continue;
            }

            if self.registry.is_output_node(class_type) {
                output_nodes.push(node_id.clone());
            }
        }

        if output_nodes.is_empty() && node_errors.is_empty() {
            return ValidationResult {
                valid: false,
                error: Some(ErrorDetail {
                    error_type: "no_output_nodes".to_string(),
                    message: "Prompt has no output nodes".to_string(),
                    details: "At least one output node is required".to_string(),
                }),
                node_errors,
            };
        }

        if !node_errors.is_empty() {
            return ValidationResult {
                valid: false,
                error: Some(ErrorDetail {
                    error_type: "prompt_outputs_failed_validation".to_string(),
                    message: "Prompt outputs failed validation".to_string(),
                    details: format!("{} node(s) have errors", node_errors.len()),
                }),
                node_errors,
            };
        }

        ValidationResult {
            valid: true,
            error: None,
            node_errors,
        }
    }

    pub async fn execute(
        &self,
        dynprompt: Arc<DynamicPrompt>,
        prompt_id: impl Into<String>,
    ) -> Result<ExecutionResult, ExecutorError> {
        let prompt_id = prompt_id.into();
        let mut ctx = ExecutionContext::new(
            dynprompt.clone(),
            self.backend.clone(),
            &prompt_id,
        );

        let mut exec_list = ExecutionList::new(dynprompt.clone(), self.cache.clone());

        let output_node_ids: Vec<String> = dynprompt
            .original_node_ids()
            .filter(|id| {
                if let Ok(node) = dynprompt.get_node(id) {
                    self.registry.is_output_node(&node.class_type)
                } else {
                    false
                }
            })
            .map(|s| s.to_string())
            .collect();

        if output_node_ids.is_empty() {
            return Err(ExecutorError::NoOutputNodes);
        }

        for node_id in &output_node_ids {
            exec_list.add_node(node_id, false, None);
        }

        let mut executed: HashSet<String> = HashSet::new();

        loop {
            let node_id = match exec_list.stage_node_execution().await? {
                Some(id) => id,
                None => break,
            };

            let node = dynprompt.get_node(&node_id).map_err(|_| {
                ExecutorError::NodeNotFound {
                    node_id: node_id.clone(),
                }
            })?;

            let execute_fn = self.registry.get_execute_fn(&node.class_type).ok_or_else(|| {
                ExecutorError::NodeTypeNotRegistered {
                    class_type: node.class_type.clone(),
                }
            })?;

            tracing::info!(
                "Executing node {} (type: {})",
                node_id,
                node.class_type
            );

            match execute_fn(&ctx, node, &node_id).await {
                Ok(outputs) => {
                    let output = NodeOutput::new(outputs);
                    ctx.set_output(&node_id, output);
                    exec_list.complete_node_execution();
                    executed.insert(node_id.clone());
                    tracing::info!("Node {} completed successfully", node_id);
                }
                Err(e) => {
                    tracing::error!("Node {} failed: {}", node_id, e);
                    exec_list.complete_node_execution();
                    return Err(ExecutorError::NodeExecutionFailed {
                        node_id: node_id.clone(),
                        message: e.to_string(),
                    });
                }
            }
        }

        let mut outputs: HashMap<String, NodeOutput> = HashMap::new();
        for node_id in &executed {
            if let Some(output) = ctx.get_output(node_id) {
                outputs.insert(node_id.clone(), output.clone());
            }
        }

        Ok(ExecutionResult {
            prompt_id,
            outputs,
            executed: executed.into_iter().collect(),
        })
    }
}

#[derive(Debug)]
pub struct ExecutionResult {
    pub prompt_id: String,
    pub outputs: HashMap<String, NodeOutput>,
    pub executed: Vec<String>,
}

impl ExecutionResult {
    pub fn get_output(&self, node_id: &str) -> Option<&NodeOutput> {
        self.outputs.get(node_id)
    }

    pub fn output_value(&self, node_id: &str, index: usize) -> Option<&Value> {
        self.outputs.get(node_id).and_then(|o| o.get(index))
    }
}

#[allow(dead_code)]
struct ExecutorCache {
    entries: HashMap<String, Arc<CacheEntry>>,
}

#[allow(dead_code)]
impl ExecutorCache {
    fn new() -> Self {
        Self {
            entries: HashMap::new(),
        }
    }

    fn set(&mut self, node_id: impl Into<String>, entry: CacheEntry) {
        self.entries.insert(node_id.into(), Arc::new(entry));
    }

    fn get(&self, node_id: &str) -> Option<Arc<CacheEntry>> {
        self.entries.get(node_id).cloned()
    }
}

impl CacheView for ExecutorCache {
    fn get_local(&self, node_id: &str) -> Option<Arc<dyn CacheEntryValue>> {
        self.entries.get(node_id).map(|e| e.clone() as Arc<dyn CacheEntryValue>)
    }
}
