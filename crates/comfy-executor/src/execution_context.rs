use crate::error::ExecutorError;
use comfy_core::DynamicPrompt;
use comfy_inference::InferenceBackend;
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Arc;

pub type ProgressCallback = Arc<dyn Fn(&str, &str, f64, f64) + Send + Sync>;

pub struct ExecutionContext {
    outputs: HashMap<String, NodeOutput>,
    dynprompt: Arc<DynamicPrompt>,
    backend: Arc<dyn InferenceBackend>,
    extra_data: HashMap<String, Value>,
    prompt_id: String,
    progress_callback: Option<ProgressCallback>,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct NodeOutput {
    pub values: Vec<Value>,
    pub ui: Option<Value>,
}

impl NodeOutput {
    pub fn new(values: Vec<Value>) -> Self {
        Self { values, ui: None }
    }

    pub fn with_ui(mut self, ui: Value) -> Self {
        self.ui = Some(ui);
        self
    }

    pub fn get(&self, index: usize) -> Option<&Value> {
        self.values.get(index)
    }

    pub fn len(&self) -> usize {
        self.values.len()
    }

    pub fn is_empty(&self) -> bool {
        self.values.is_empty()
    }
}

impl ExecutionContext {
    pub fn new(
        dynprompt: Arc<DynamicPrompt>,
        backend: Arc<dyn InferenceBackend>,
        prompt_id: impl Into<String>,
    ) -> Self {
        Self {
            outputs: HashMap::new(),
            dynprompt,
            backend,
            extra_data: HashMap::new(),
            prompt_id: prompt_id.into(),
            progress_callback: None,
        }
    }

    pub fn with_extra_data(mut self, extra: HashMap<String, Value>) -> Self {
        self.extra_data = extra;
        self
    }

    pub fn with_progress_callback(mut self, callback: ProgressCallback) -> Self {
        self.progress_callback = Some(callback);
        self
    }

    /// Report generation progress (value, max) for the current node
    pub fn report_progress(&self, node_id: &str, value: f64, max: f64) {
        if let Some(ref cb) = self.progress_callback {
            cb(&self.prompt_id, node_id, value, max);
        }
    }

    /// Get a clone of the progress callback (for passing to backends)
    pub fn progress_callback(&self) -> Option<ProgressCallback> {
        self.progress_callback.clone()
    }

    pub fn set_output(&mut self, node_id: impl Into<String>, output: NodeOutput) {
        self.outputs.insert(node_id.into(), output);
    }

    pub fn get_output(&self, node_id: &str) -> Option<&NodeOutput> {
        self.outputs.get(node_id)
    }

    pub fn get_output_value(&self, node_id: &str, output_index: usize) -> Result<Value, ExecutorError> {
        let output = self.outputs.get(node_id).ok_or_else(|| {
            ExecutorError::NodeNotFound {
                node_id: node_id.to_string(),
            }
        })?;
        output
            .get(output_index)
            .cloned()
            .ok_or_else(|| ExecutorError::OutputIndexOutOfBounds {
                node_id: node_id.to_string(),
                index: output_index,
                max: output.len(),
            })
    }

    pub fn resolve_input(
        &self,
        node_id: &str,
        input_name: &str,
    ) -> Result<Value, ExecutorError> {
        let node = self.dynprompt.get_node(node_id).map_err(|_| {
            ExecutorError::NodeNotFound {
                node_id: node_id.to_string(),
            }
        })?;

        let value = node.inputs.get(input_name).ok_or_else(|| {
            ExecutorError::MissingInput {
                node_id: node_id.to_string(),
                input: input_name.to_string(),
            }
        })?;

        if comfy_core::is_link(value) {
            let arr = value.as_array().ok_or_else(|| {
                ExecutorError::NodeExecutionFailed {
                    node_id: node_id.to_string(),
                    message: format!("Invalid link format for input '{}'", input_name),
                }
            })?;
            let from_node = arr[0].as_str().ok_or_else(|| {
                ExecutorError::NodeExecutionFailed {
                    node_id: node_id.to_string(),
                    message: format!("Invalid link node id for input '{}'", input_name),
                }
            })?;
            let from_socket = arr[1].as_u64().ok_or_else(|| {
                ExecutorError::NodeExecutionFailed {
                    node_id: node_id.to_string(),
                    message: format!("Invalid link socket index for input '{}'", input_name),
                }
            })? as usize;
            self.get_output_value(from_node, from_socket)
        } else {
            Ok(value.clone())
        }
    }

    pub fn resolve_all_inputs(
        &self,
        node_id: &str,
    ) -> Result<HashMap<String, Value>, ExecutorError> {
        let node = self.dynprompt.get_node(node_id).map_err(|_| {
            ExecutorError::NodeNotFound {
                node_id: node_id.to_string(),
            }
        })?;

        let mut resolved = HashMap::new();
        for (input_name, value) in &node.inputs {
            if comfy_core::is_link(value) {
                let arr = value.as_array().ok_or_else(|| {
                    ExecutorError::NodeExecutionFailed {
                        node_id: node_id.to_string(),
                        message: format!("Invalid link format for input '{}'", input_name),
                    }
                })?;
                let from_node = arr[0]
                    .as_str()
                    .ok_or_else(|| ExecutorError::NodeExecutionFailed {
                        node_id: node_id.to_string(),
                        message: format!("Invalid link node id for input '{}'", input_name),
                    })?;
                let from_socket = arr[1]
                    .as_u64()
                    .ok_or_else(|| ExecutorError::NodeExecutionFailed {
                        node_id: node_id.to_string(),
                        message: format!("Invalid link socket index for input '{}'", input_name),
                    })? as usize;
                resolved.insert(
                    input_name.clone(),
                    self.get_output_value(from_node, from_socket)?,
                );
            } else {
                resolved.insert(input_name.clone(), value.clone());
            }
        }
        Ok(resolved)
    }

    pub fn dynprompt(&self) -> &DynamicPrompt {
        &self.dynprompt
    }

    pub fn backend(&self) -> &dyn InferenceBackend {
        self.backend.as_ref()
    }

    pub fn extra_data(&self) -> &HashMap<String, Value> {
        &self.extra_data
    }

    pub fn get_extra_data(&self, key: &str) -> Option<&Value> {
        self.extra_data.get(key)
    }

    pub fn prompt_id(&self) -> &str {
        &self.prompt_id
    }

    pub fn all_outputs(&self) -> &HashMap<String, NodeOutput> {
        &self.outputs
    }

    pub fn has_output(&self, node_id: &str) -> bool {
        self.outputs.contains_key(node_id)
    }
}
