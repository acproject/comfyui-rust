use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeDefinition {
    pub class_type: String,
    #[serde(default)]
    pub inputs: HashMap<String, serde_json::Value>,
    #[serde(default)]
    pub is_changed: Option<serde_json::Value>,
}

impl NodeDefinition {
    pub fn new(class_type: impl Into<String>) -> Self {
        Self {
            class_type: class_type.into(),
            inputs: HashMap::new(),
            is_changed: None,
        }
    }

    pub fn with_input(mut self, key: impl Into<String>, value: serde_json::Value) -> Self {
        self.inputs.insert(key.into(), value);
        self
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InputTypeInfo {
    pub type_name: String,
    pub category: InputCategory,
    pub extra: HashMap<String, serde_json::Value>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum InputCategory {
    Required,
    Optional,
    Hidden,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeClassDef {
    pub class_type: String,
    pub display_name: String,
    pub category: String,
    pub input_types: NodeInputTypes,
    pub output_types: Vec<IoType>,
    pub output_names: Vec<String>,
    pub output_is_list: Vec<bool>,
    pub is_output_node: bool,
    pub has_intermediate_output: bool,
    pub is_changed: Option<String>,
    pub not_idempotent: bool,
    pub function_name: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct NodeInputTypes {
    pub required: HashMap<String, InputTypeSpec>,
    pub optional: HashMap<String, InputTypeSpec>,
    pub hidden: HashMap<String, HiddenInputSpec>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InputTypeSpec {
    pub type_name: String,
    pub extra: HashMap<String, serde_json::Value>,
}

impl InputTypeSpec {
    pub fn is_lazy(&self) -> bool {
        self.extra
            .get("lazy")
            .and_then(|v| v.as_bool())
            .unwrap_or(false)
    }

    pub fn is_raw_link(&self) -> bool {
        self.extra
            .get("rawLink")
            .and_then(|v| v.as_bool())
            .unwrap_or(false)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HiddenInputSpec {
    pub kind: HiddenInputKind,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum HiddenInputKind {
    Prompt,
    DynPrompt,
    ExtraPngInfo,
    UniqueId,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum IoType {
    Any,
    String,
    Int,
    Float,
    Boolean,
    Model,
    Clip,
    Vae,
    Image,
    Mask,
    Latent,
    Conditioning,
    ControlNet,
    Combo(Vec<String>),
    Custom(String),
}

impl IoType {
    pub fn io_type_str(&self) -> String {
        match self {
            IoType::Any => "*".to_string(),
            IoType::String => "STRING".to_string(),
            IoType::Int => "INT".to_string(),
            IoType::Float => "FLOAT".to_string(),
            IoType::Boolean => "BOOLEAN".to_string(),
            IoType::Model => "MODEL".to_string(),
            IoType::Clip => "CLIP".to_string(),
            IoType::Vae => "VAE".to_string(),
            IoType::Image => "IMAGE".to_string(),
            IoType::Mask => "MASK".to_string(),
            IoType::Latent => "LATENT".to_string(),
            IoType::Conditioning => "CONDITIONING".to_string(),
            IoType::ControlNet => "CONTROL_NET".to_string(),
            IoType::Combo(opts) => opts.join(","),
            IoType::Custom(s) => s.clone(),
        }
    }

    pub fn from_io_type_str(s: &str) -> Self {
        match s {
            "*" => IoType::Any,
            "STRING" => IoType::String,
            "INT" => IoType::Int,
            "FLOAT" => IoType::Float,
            "BOOLEAN" => IoType::Boolean,
            "MODEL" => IoType::Model,
            "CLIP" => IoType::Clip,
            "VAE" => IoType::Vae,
            "IMAGE" => IoType::Image,
            "MASK" => IoType::Mask,
            "LATENT" => IoType::Latent,
            "CONDITIONING" => IoType::Conditioning,
            "CONTROL_NET" => IoType::ControlNet,
            other => IoType::Custom(other.to_string()),
        }
    }
}
