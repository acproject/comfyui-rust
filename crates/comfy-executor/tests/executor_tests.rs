use comfy_core::{DynamicPrompt, NodeDefinition};
use comfy_executor::*;
use comfy_inference::NullBackend;
use serde_json::{json, Value};
use std::collections::HashMap;
use std::sync::Arc;

fn make_node(class_type: &str, inputs: Vec<(&str, Value)>) -> NodeDefinition {
    let mut node = NodeDefinition::new(class_type);
    for (key, value) in inputs {
        node = node.with_input(key, value);
    }
    node
}

fn link_value(from_node: &str, from_socket: usize) -> Value {
    json!([from_node, from_socket])
}

fn create_registry() -> NodeRegistry {
    let mut registry = NodeRegistry::new();
    builtin_nodes::register_builtin_nodes(&mut registry);
    registry
}

#[test]
fn test_registry_builtin_nodes() {
    let registry = create_registry();

    assert!(registry.has_node("CheckpointLoaderSimple"));
    assert!(registry.has_node("CLIPTextEncode"));
    assert!(registry.has_node("KSampler"));
    assert!(registry.has_node("SaveImage"));
    assert!(registry.has_node("EmptyLatentImage"));
    assert!(registry.has_node("VAEDecode"));
    assert!(registry.has_node("LoadImage"));

    assert!(!registry.has_node("NonExistentNode"));
}

#[test]
fn test_registry_output_node() {
    let registry = create_registry();

    assert!(registry.is_output_node("SaveImage"));
    assert!(!registry.is_output_node("KSampler"));
    assert!(!registry.is_output_node("CLIPTextEncode"));
}

#[test]
fn test_registry_output_types() {
    let registry = create_registry();

    let types = registry.output_types("CheckpointLoaderSimple").unwrap();
    assert_eq!(types.len(), 3);

    let names = registry.output_names("CheckpointLoaderSimple").unwrap();
    assert_eq!(names[0], "MODEL");
    assert_eq!(names[1], "CLIP");
    assert_eq!(names[2], "VAE");
}

#[test]
fn test_validate_prompt_valid() {
    let registry = create_registry();
    let executor = Executor::new(registry, Arc::new(NullBackend));

    let mut prompt = HashMap::new();
    prompt.insert("1".to_string(), json!({
        "class_type": "SaveImage",
        "inputs": { "images": ["2", 0] }
    }));
    prompt.insert("2".to_string(), json!({
        "class_type": "KSampler",
        "inputs": {}
    }));

    let result = executor.validate_prompt(&prompt);
    assert!(result.valid);
    assert!(result.error.is_none());
}

#[test]
fn test_validate_prompt_missing_class_type() {
    let registry = create_registry();
    let executor = Executor::new(registry, Arc::new(NullBackend));

    let mut prompt = HashMap::new();
    prompt.insert("1".to_string(), json!({
        "inputs": {}
    }));

    let result = executor.validate_prompt(&prompt);
    assert!(!result.valid);
    assert!(result.node_errors.contains_key("1"));
}

#[test]
fn test_validate_prompt_unknown_node_type() {
    let registry = create_registry();
    let executor = Executor::new(registry, Arc::new(NullBackend));

    let mut prompt = HashMap::new();
    prompt.insert("1".to_string(), json!({
        "class_type": "NonExistentNode",
        "inputs": {}
    }));

    let result = executor.validate_prompt(&prompt);
    assert!(!result.valid);
}

#[test]
fn test_validate_prompt_no_output_nodes() {
    let registry = create_registry();
    let executor = Executor::new(registry, Arc::new(NullBackend));

    let mut prompt = HashMap::new();
    prompt.insert("1".to_string(), json!({
        "class_type": "KSampler",
        "inputs": {}
    }));

    let result = executor.validate_prompt(&prompt);
    assert!(!result.valid);
}

#[tokio::test]
async fn test_execute_simple_workflow() {
    let registry = create_registry();
    let executor = Executor::new(registry, Arc::new(NullBackend));

    let mut prompt = HashMap::new();
    prompt.insert("1".to_string(), make_node("SaveImage", vec![
        ("images", link_value("2", 0)),
    ]));
    prompt.insert("2".to_string(), make_node("VAEDecode", vec![
        ("samples", link_value("3", 0)),
        ("vae", link_value("4", 2)),
    ]));
    prompt.insert("3".to_string(), make_node("KSampler", vec![
        ("model", link_value("4", 0)),
        ("positive", link_value("5", 0)),
        ("negative", link_value("6", 0)),
        ("latent_image", link_value("7", 0)),
        ("seed", json!(42)),
        ("steps", json!(20)),
        ("cfg", json!(7.0)),
        ("sampler_name", json!("euler_ancestral")),
        ("scheduler", json!("normal")),
    ]));
    prompt.insert("4".to_string(), make_node("CheckpointLoaderSimple", vec![
        ("ckpt_name", json!("model.safetensors")),
    ]));
    prompt.insert("5".to_string(), make_node("CLIPTextEncode", vec![
        ("text", json!("a beautiful sunset")),
        ("clip", link_value("4", 1)),
    ]));
    prompt.insert("6".to_string(), make_node("CLIPTextEncode", vec![
        ("text", json!("ugly, blurry")),
        ("clip", link_value("4", 1)),
    ]));
    prompt.insert("7".to_string(), make_node("EmptyLatentImage", vec![
        ("width", json!(512)),
        ("height", json!(512)),
        ("batch_size", json!(1)),
    ]));

    let dynprompt = Arc::new(DynamicPrompt::new(prompt));
    let result = executor.execute(dynprompt, "test-prompt-1").await;

    assert!(result.is_ok(), "Execution should succeed: {:?}", result.err());
    let exec_result = result.unwrap();
    assert_eq!(exec_result.prompt_id, "test-prompt-1");
    assert!(exec_result.executed.contains(&"4".to_string()));
    assert!(exec_result.executed.contains(&"5".to_string()));
    assert!(exec_result.executed.contains(&"6".to_string()));
    assert!(exec_result.executed.contains(&"7".to_string()));
    assert!(exec_result.executed.contains(&"3".to_string()));
    assert!(exec_result.executed.contains(&"2".to_string()));
    assert!(exec_result.executed.contains(&"1".to_string()));
}

#[tokio::test]
async fn test_execute_checkpoint_loader() {
    let registry = create_registry();
    let executor = Executor::new(registry, Arc::new(NullBackend));

    let mut prompt = HashMap::new();
    prompt.insert("1".to_string(), make_node("SaveImage", vec![
        ("images", link_value("2", 0)),
    ]));
    prompt.insert("2".to_string(), make_node("VAEDecode", vec![
        ("samples", link_value("3", 0)),
        ("vae", link_value("4", 2)),
    ]));
    prompt.insert("3".to_string(), make_node("KSampler", vec![
        ("model", link_value("4", 0)),
        ("positive", link_value("5", 0)),
        ("negative", link_value("6", 0)),
        ("latent_image", link_value("7", 0)),
        ("seed", json!(42)),
        ("steps", json!(20)),
        ("cfg", json!(7.0)),
        ("sampler_name", json!("euler_ancestral")),
        ("scheduler", json!("normal")),
    ]));
    prompt.insert("4".to_string(), make_node("CheckpointLoaderSimple", vec![
        ("ckpt_name", json!("test_model.safetensors")),
    ]));
    prompt.insert("5".to_string(), make_node("CLIPTextEncode", vec![
        ("text", json!("hello world")),
        ("clip", link_value("4", 1)),
    ]));
    prompt.insert("6".to_string(), make_node("CLIPTextEncode", vec![
        ("text", json!("")),
        ("clip", link_value("4", 1)),
    ]));
    prompt.insert("7".to_string(), make_node("EmptyLatentImage", vec![
        ("width", json!(512)),
        ("height", json!(512)),
        ("batch_size", json!(1)),
    ]));

    let dynprompt = Arc::new(DynamicPrompt::new(prompt));
    let result = executor.execute(dynprompt, "test-checkpoint").await.unwrap();

    let checkpoint_output = result.get_output("4").unwrap();
    assert_eq!(checkpoint_output.len(), 3);
    let model_output = checkpoint_output.get(0).unwrap();
    assert!(model_output["model_path"].as_str().unwrap().contains("test_model.safetensors"));
    assert_eq!(model_output["model_type"], "unknown");
    let clip_output = checkpoint_output.get(1).unwrap();
    assert_eq!(clip_output["type"], "clip");
    assert_eq!(clip_output["model_type"], "unknown");
    let vae_output = checkpoint_output.get(2).unwrap();
    assert_eq!(vae_output["type"], "vae");
    assert_eq!(vae_output["model_type"], "unknown");
}

#[tokio::test]
async fn test_execute_clip_text_encode() {
    let registry = create_registry();
    let executor = Executor::new(registry, Arc::new(NullBackend));

    let mut prompt = HashMap::new();
    prompt.insert("1".to_string(), make_node("SaveImage", vec![
        ("images", link_value("2", 0)),
    ]));
    prompt.insert("2".to_string(), make_node("VAEDecode", vec![
        ("samples", link_value("3", 0)),
        ("vae", link_value("4", 2)),
    ]));
    prompt.insert("3".to_string(), make_node("KSampler", vec![
        ("model", link_value("4", 0)),
        ("positive", link_value("5", 0)),
        ("negative", link_value("6", 0)),
        ("latent_image", link_value("7", 0)),
        ("seed", json!(42)),
        ("steps", json!(20)),
        ("cfg", json!(7.0)),
        ("sampler_name", json!("euler_ancestral")),
        ("scheduler", json!("normal")),
    ]));
    prompt.insert("4".to_string(), make_node("CheckpointLoaderSimple", vec![
        ("ckpt_name", json!("model.safetensors")),
    ]));
    prompt.insert("5".to_string(), make_node("CLIPTextEncode", vec![
        ("text", json!("a beautiful landscape")),
        ("clip", link_value("4", 1)),
    ]));
    prompt.insert("6".to_string(), make_node("CLIPTextEncode", vec![
        ("text", json!("dark, ugly")),
        ("clip", link_value("4", 1)),
    ]));
    prompt.insert("7".to_string(), make_node("EmptyLatentImage", vec![
        ("width", json!(512)),
        ("height", json!(512)),
        ("batch_size", json!(1)),
    ]));

    let dynprompt = Arc::new(DynamicPrompt::new(prompt));
    let result = executor.execute(dynprompt, "test-clip").await.unwrap();

    let clip_output = result.get_output("5").unwrap();
    assert_eq!(clip_output.len(), 1);
    let cond = clip_output.get(0).unwrap();
    assert_eq!(cond["type"], "conditioning");
    assert_eq!(cond["text"], "a beautiful landscape");
}

#[tokio::test]
async fn test_execute_empty_latent_image() {
    let registry = create_registry();
    let executor = Executor::new(registry, Arc::new(NullBackend));

    let mut prompt = HashMap::new();
    prompt.insert("1".to_string(), make_node("SaveImage", vec![
        ("images", link_value("2", 0)),
    ]));
    prompt.insert("2".to_string(), make_node("VAEDecode", vec![
        ("samples", link_value("3", 0)),
        ("vae", link_value("4", 2)),
    ]));
    prompt.insert("3".to_string(), make_node("KSampler", vec![
        ("model", link_value("4", 0)),
        ("positive", link_value("5", 0)),
        ("negative", link_value("6", 0)),
        ("latent_image", link_value("7", 0)),
        ("seed", json!(42)),
        ("steps", json!(20)),
        ("cfg", json!(7.0)),
        ("sampler_name", json!("euler_ancestral")),
        ("scheduler", json!("normal")),
    ]));
    prompt.insert("4".to_string(), make_node("CheckpointLoaderSimple", vec![]));
    prompt.insert("5".to_string(), make_node("CLIPTextEncode", vec![
        ("text", json!("test")),
        ("clip", link_value("4", 1)),
    ]));
    prompt.insert("6".to_string(), make_node("CLIPTextEncode", vec![
        ("text", json!("")),
        ("clip", link_value("4", 1)),
    ]));
    prompt.insert("7".to_string(), make_node("EmptyLatentImage", vec![
        ("width", json!(768)),
        ("height", json!(1024)),
        ("batch_size", json!(2)),
    ]));

    let dynprompt = Arc::new(DynamicPrompt::new(prompt));
    let result = executor.execute(dynprompt, "test-latent").await.unwrap();

    let latent_output = result.get_output("7").unwrap();
    assert_eq!(latent_output.len(), 1);
    let latent = latent_output.get(0).unwrap();
    assert_eq!(latent["type"], "latent");
    assert_eq!(latent["width"], 768);
    assert_eq!(latent["height"], 1024);
    assert_eq!(latent["batch_size"], 2);
}

#[tokio::test]
async fn test_execute_no_output_nodes() {
    let registry = create_registry();
    let executor = Executor::new(registry, Arc::new(NullBackend));

    let mut prompt = HashMap::new();
    prompt.insert("1".to_string(), make_node("KSampler", vec![]));

    let dynprompt = Arc::new(DynamicPrompt::new(prompt));
    let result = executor.execute(dynprompt, "test-no-output").await;

    assert!(result.is_err());
    match result.unwrap_err() {
        ExecutorError::NoOutputNodes => {}
        e => panic!("Expected NoOutputNodes error, got: {}", e),
    }
}

#[tokio::test]
async fn test_execute_cycle_detection() {
    let registry = create_registry();
    let executor = Executor::new(registry, Arc::new(NullBackend));

    let mut prompt = HashMap::new();
    prompt.insert("1".to_string(), make_node("KSampler", vec![
        ("model", link_value("2", 0)),
    ]));
    prompt.insert("2".to_string(), make_node("KSampler", vec![
        ("model", link_value("1", 0)),
    ]));

    let dynprompt = Arc::new(DynamicPrompt::new(prompt));
    let result = executor.execute(dynprompt, "test-cycle").await;

    assert!(result.is_err());
}

#[test]
fn test_node_output() {
    let output = NodeOutput::new(vec![json!(1), json!("hello"), json!(null)]);

    assert_eq!(output.len(), 3);
    assert!(!output.is_empty());
    assert_eq!(output.get(0), Some(&json!(1)));
    assert_eq!(output.get(1), Some(&json!("hello")));
    assert_eq!(output.get(2), Some(&json!(null)));
    assert_eq!(output.get(3), None);

    let output_with_ui = output.with_ui(json!({"progress": 0.5}));
    assert!(output_with_ui.ui.is_some());
}

#[test]
fn test_custom_node_registration() {
    let mut registry = NodeRegistry::new();

    let class_def = comfy_core::NodeClassDef {
        class_type: "CustomNode".to_string(),
        display_name: "Custom Node".to_string(),
        category: "custom".to_string(),
        input_types: comfy_core::NodeInputTypes::default(),
        output_types: vec![comfy_core::IoType::String],
        output_names: vec!["OUTPUT".to_string()],
        output_is_list: vec![false],
        is_output_node: false,
        has_intermediate_output: false,
        is_changed: None,
        not_idempotent: false,
        function_name: "execute".to_string(),
    };

    registry.register(class_def, Arc::new(|_ctx, _node, _node_id| {
        Box::pin(async move {
            Ok(vec![json!("custom output")])
        })
    }));

    assert!(registry.has_node("CustomNode"));
    assert!(!registry.is_output_node("CustomNode"));
}

#[test]
fn test_execution_result() {
    let mut outputs = HashMap::new();
    outputs.insert("1".to_string(), NodeOutput::new(vec![json!("result1")]));
    outputs.insert("2".to_string(), NodeOutput::new(vec![json!(42), json!("hello")]));

    let result = ExecutionResult {
        prompt_id: "test".to_string(),
        outputs,
        executed: vec!["1".to_string(), "2".to_string()],
    };

    assert_eq!(result.prompt_id, "test");
    assert_eq!(result.output_value("1", 0), Some(&json!("result1")));
    assert_eq!(result.output_value("2", 0), Some(&json!(42)));
    assert_eq!(result.output_value("2", 1), Some(&json!("hello")));
    assert_eq!(result.output_value("3", 0), None);
}

#[test]
fn test_error_types() {
    let err = ExecutorError::NodeNotFound {
        node_id: "test".to_string(),
    };
    assert!(err.to_string().contains("test"));

    let err = ExecutorError::NodeTypeNotRegistered {
        class_type: "Foo".to_string(),
    };
    assert!(err.to_string().contains("Foo"));

    let err = ExecutorError::MissingInput {
        node_id: "1".to_string(),
        input: "model".to_string(),
    };
    assert!(err.to_string().contains("model"));
}
