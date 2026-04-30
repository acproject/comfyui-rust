use comfy_core::*;

fn make_node(class_type: &str, inputs: Vec<(&str, serde_json::Value)>) -> NodeDefinition {
    let mut node = NodeDefinition::new(class_type);
    for (key, value) in inputs {
        node = node.with_input(key, value);
    }
    node
}

fn link_value(from_node: &str, from_socket: usize) -> serde_json::Value {
    serde_json::json!([from_node, from_socket])
}

#[test]
fn test_dynamic_prompt_basic() {
    let mut prompt = std::collections::HashMap::new();
    prompt.insert("1".to_string(), make_node("KSampler", vec![]));
    prompt.insert("2".to_string(), make_node("CLIPTextEncode", vec![]));

    let dp = DynamicPrompt::new(prompt);

    assert!(dp.has_node("1"));
    assert!(dp.has_node("2"));
    assert!(!dp.has_node("3"));

    assert_eq!(dp.get_node("1").unwrap().class_type, "KSampler");
    assert_eq!(dp.get_node("2").unwrap().class_type, "CLIPTextEncode");
    assert!(dp.get_node("3").is_err());
}

#[test]
fn test_dynamic_prompt_ephemeral_node() {
    let mut prompt = std::collections::HashMap::new();
    prompt.insert("1".to_string(), make_node("KSampler", vec![]));

    let mut dp = DynamicPrompt::new(prompt);
    dp.add_ephemeral_node("2", make_node("KSampler", vec![]), "1", "1");

    assert!(dp.has_node("2"));
    assert_eq!(dp.get_node("2").unwrap().class_type, "KSampler");
    assert_eq!(dp.get_parent_node_id("2"), Some("1"));
    assert_eq!(dp.get_real_node_id("2"), "1");
    assert_eq!(dp.get_display_node_id("2"), "1");
}

#[test]
fn test_dynamic_prompt_chained_ephemeral() {
    let mut prompt = std::collections::HashMap::new();
    prompt.insert("1".to_string(), make_node("KSampler", vec![]));

    let mut dp = DynamicPrompt::new(prompt);
    dp.add_ephemeral_node("2", make_node("KSampler", vec![]), "1", "1");
    dp.add_ephemeral_node("3", make_node("KSampler", vec![]), "2", "1");

    assert_eq!(dp.get_real_node_id("3"), "1");
    assert_eq!(dp.get_real_node_id("2"), "1");
    assert_eq!(dp.get_real_node_id("1"), "1");
    assert_eq!(dp.get_display_node_id("3"), "1");
}

#[test]
fn test_dynamic_prompt_all_node_ids() {
    let mut prompt = std::collections::HashMap::new();
    prompt.insert("1".to_string(), make_node("KSampler", vec![]));
    prompt.insert("2".to_string(), make_node("CLIPTextEncode", vec![]));

    let mut dp = DynamicPrompt::new(prompt);
    dp.add_ephemeral_node("3", make_node("KSampler", vec![]), "1", "1");

    let ids = dp.all_node_ids();
    assert_eq!(ids.len(), 3);
    assert!(ids.contains("1"));
    assert!(ids.contains("2"));
    assert!(ids.contains("3"));
}

#[test]
fn test_dynamic_prompt_original_node_ids() {
    let mut prompt = std::collections::HashMap::new();
    prompt.insert("1".to_string(), make_node("KSampler", vec![]));
    prompt.insert("2".to_string(), make_node("CLIPTextEncode", vec![]));

    let dp = DynamicPrompt::new(prompt);
    let ids: Vec<&str> = dp.original_node_ids().collect();
    assert_eq!(ids.len(), 2);
}

#[test]
fn test_dynamic_prompt_get_input_links() {
    let mut prompt = std::collections::HashMap::new();
    prompt.insert("1".to_string(), make_node("KSampler", vec![
        ("model", link_value("2", 0)),
        ("positive", link_value("3", 0)),
        ("seed", serde_json::json!(42)),
    ]));
    prompt.insert("2".to_string(), make_node("CheckpointLoader", vec![]));
    prompt.insert("3".to_string(), make_node("CLIPTextEncode", vec![]));

    let dp = DynamicPrompt::new(prompt);
    let links = dp.get_input_links("1");
    assert_eq!(links.len(), 2);

    let model_link = links.iter().find(|(name, _, _)| name == "model").unwrap();
    assert_eq!(model_link.1, "2");
    assert_eq!(model_link.2, 0);

    let positive_link = links.iter().find(|(name, _, _)| name == "positive").unwrap();
    assert_eq!(positive_link.1, "3");
}

#[test]
fn test_edge_is_link() {
    assert!(is_link(&serde_json::json!(["node1", 0])));
    assert!(is_link(&serde_json::json!(["node1", 1])));
    assert!(!is_link(&serde_json::json!(42)));
    assert!(!is_link(&serde_json::json!("hello")));
    assert!(!is_link(&serde_json::json!(["node1"])));
    assert!(!is_link(&serde_json::json!(["node1", "not_number"])));
    assert!(!is_link(&serde_json::json!([1, 0])));
}

#[test]
fn test_edge_parse_link() {
    let link = parse_link(&serde_json::json!(["node1", 0]));
    assert!(link.is_some());
    let link = link.unwrap();
    assert_eq!(link.from_node, "node1");
    assert_eq!(link.from_socket, 0);

    let link = parse_link(&serde_json::json!(42));
    assert!(link.is_none());
}

#[test]
fn test_io_type_str_roundtrip() {
    let types = vec![
        IoType::Any,
        IoType::String,
        IoType::Int,
        IoType::Float,
        IoType::Boolean,
        IoType::Model,
        IoType::Clip,
        IoType::Vae,
        IoType::Image,
        IoType::Mask,
        IoType::Latent,
        IoType::Conditioning,
        IoType::ControlNet,
    ];

    for io_type in types {
        let s = io_type.io_type_str();
        let roundtrip = IoType::from_io_type_str(&s);
        assert_eq!(io_type, roundtrip, "Roundtrip failed for {:?}", io_type);
    }

    let custom = IoType::Custom("MY_TYPE".to_string());
    assert_eq!(custom.io_type_str(), "MY_TYPE");
    let parsed = IoType::from_io_type_str("MY_TYPE");
    assert_eq!(parsed, IoType::Custom("MY_TYPE".to_string()));
}

#[test]
fn test_graph_builder_basic() {
    let mut builder = GraphBuilder::new("test_");

    let id1 = builder.node("KSampler", Some("1"), HashMap::new());
    assert_eq!(id1, "test_1");

    let id2 = builder.node("CLIPTextEncode", Some("2"), HashMap::new());
    assert_eq!(id2, "test_2");
}

#[test]
fn test_graph_builder_auto_id() {
    let mut builder = GraphBuilder::new("");
    let id1 = builder.node("KSampler", None, HashMap::new());
    let id2 = builder.node("CLIPTextEncode", None, HashMap::new());
    assert_eq!(id1, "1");
    assert_eq!(id2, "2");
}

#[test]
fn test_graph_builder_finalize() {
    let mut builder = GraphBuilder::new("");

    let mut ksampler_inputs = HashMap::new();
    ksampler_inputs.insert("model".to_string(), link_value("2", 0));
    builder.node("KSampler", Some("1"), ksampler_inputs);

    builder.node("CheckpointLoader", Some("2"), HashMap::new());

    let prompt = builder.finalize();
    assert_eq!(prompt.len(), 2);
    assert_eq!(prompt.get("1").unwrap().class_type, "KSampler");
    assert_eq!(prompt.get("2").unwrap().class_type, "CheckpointLoader");
}

#[test]
fn test_graph_builder_replace_node_output() {
    let mut builder = GraphBuilder::new("");

    let mut ksampler_inputs = HashMap::new();
    ksampler_inputs.insert("model".to_string(), link_value("2", 0));
    builder.node("KSampler", Some("1"), ksampler_inputs);
    builder.node("CheckpointLoader", Some("2"), HashMap::new());

    builder.replace_node_output("2", 0, Some(serde_json::json!("replaced")));

    let prompt = builder.finalize();
    assert_eq!(prompt["1"].inputs["model"], serde_json::json!("replaced"));
}

#[test]
fn test_graph_builder_remove_node() {
    let mut builder = GraphBuilder::new("");
    builder.node("KSampler", Some("1"), HashMap::new());
    builder.node("CheckpointLoader", Some("2"), HashMap::new());

    builder.remove_node("2");

    let prompt = builder.finalize();
    assert_eq!(prompt.len(), 1);
    assert!(prompt.contains_key("1"));
    assert!(!prompt.contains_key("2"));
}

#[test]
fn test_node_out_and_set_input() {
    let mut node = Node {
        id: "1".to_string(),
        class_type: "KSampler".to_string(),
        inputs: HashMap::new(),
    };

    let out = node.out(0);
    assert_eq!(out, serde_json::json!(["1", 0]));

    node.set_input("seed", Some(serde_json::json!(42)));
    assert_eq!(node.get_input("seed"), Some(&serde_json::json!(42)));

    node.set_input("seed", None);
    assert_eq!(node.get_input("seed"), None);
}

#[test]
fn test_topological_sort_simple_chain() {
    let mut prompt = std::collections::HashMap::new();
    prompt.insert("1".to_string(), make_node("SaveImage", vec![
        ("image", link_value("2", 0)),
    ]));
    prompt.insert("2".to_string(), make_node("VAEDecode", vec![
        ("latent", link_value("3", 0)),
    ]));
    prompt.insert("3".to_string(), make_node("KSampler", vec![]));

    let dp = std::sync::Arc::new(DynamicPrompt::new(prompt));
    let mut topo = TopologicalSort::new(dp);

    topo.add_node("1", false, None);

    assert_eq!(topo.pending_count(), 3);

    let ready = topo.get_ready_nodes();
    assert_eq!(ready.len(), 1);
    assert!(ready.contains(&"3"));

    topo.pop_node("3");
    let ready = topo.get_ready_nodes();
    assert_eq!(ready.len(), 1);
    assert!(ready.contains(&"2"));

    topo.pop_node("2");
    let ready = topo.get_ready_nodes();
    assert_eq!(ready.len(), 1);
    assert!(ready.contains(&"1"));

    topo.pop_node("1");
    assert!(topo.is_empty());
}

#[test]
fn test_topological_sort_parallel_nodes() {
    let mut prompt = std::collections::HashMap::new();
    prompt.insert("1".to_string(), make_node("SaveImage", vec![
        ("image", link_value("2", 0)),
    ]));
    prompt.insert("2".to_string(), make_node("VAEDecode", vec![]));
    prompt.insert("3".to_string(), make_node("SaveImage", vec![
        ("image", link_value("4", 0)),
    ]));
    prompt.insert("4".to_string(), make_node("VAEDecode", vec![]));

    let dp = std::sync::Arc::new(DynamicPrompt::new(prompt));
    let mut topo = TopologicalSort::new(dp);

    topo.add_node("1", false, None);
    topo.add_node("3", false, None);

    assert_eq!(topo.pending_count(), 4);

    let ready = topo.get_ready_nodes();
    assert_eq!(ready.len(), 2);
    assert!(ready.contains(&"2"));
    assert!(ready.contains(&"4"));
}

#[test]
fn test_topological_sort_diamond_dependency() {
    let mut prompt = std::collections::HashMap::new();
    prompt.insert("1".to_string(), make_node("KSampler", vec![
        ("model", link_value("4", 0)),
        ("positive", link_value("2", 0)),
        ("negative", link_value("3", 0)),
    ]));
    prompt.insert("2".to_string(), make_node("CLIPTextEncode", vec![
        ("clip", link_value("4", 1)),
    ]));
    prompt.insert("3".to_string(), make_node("CLIPTextEncode", vec![
        ("clip", link_value("4", 1)),
    ]));
    prompt.insert("4".to_string(), make_node("CheckpointLoader", vec![]));

    let dp = std::sync::Arc::new(DynamicPrompt::new(prompt));
    let mut topo = TopologicalSort::new(dp);

    topo.add_node("1", false, None);

    assert_eq!(topo.pending_count(), 4);

    let ready = topo.get_ready_nodes();
    assert_eq!(ready.len(), 1);
    assert!(ready.contains(&"4"));

    topo.pop_node("4");
    let ready = topo.get_ready_nodes();
    assert_eq!(ready.len(), 2);
    assert!(ready.contains(&"2"));
    assert!(ready.contains(&"3"));

    topo.pop_node("2");
    topo.pop_node("3");
    let ready = topo.get_ready_nodes();
    assert_eq!(ready.len(), 1);
    assert!(ready.contains(&"1"));
}

#[test]
fn test_topological_sort_cycle_detection() {
    let mut prompt = std::collections::HashMap::new();
    prompt.insert("1".to_string(), make_node("NodeA", vec![
        ("input", link_value("2", 0)),
    ]));
    prompt.insert("2".to_string(), make_node("NodeB", vec![
        ("input", link_value("1", 0)),
    ]));

    let dp = std::sync::Arc::new(DynamicPrompt::new(prompt));
    let mut topo = TopologicalSort::new(dp);

    topo.add_node("1", false, None);

    let cycle_nodes = topo.get_nodes_in_cycle();
    assert_eq!(cycle_nodes.len(), 2);
}

#[test]
fn test_topological_sort_subgraph() {
    let mut prompt = std::collections::HashMap::new();
    prompt.insert("1".to_string(), make_node("KSampler", vec![
        ("model", link_value("2", 0)),
    ]));
    prompt.insert("2".to_string(), make_node("CheckpointLoader", vec![]));
    prompt.insert("3".to_string(), make_node("SaveImage", vec![]));

    let dp = std::sync::Arc::new(DynamicPrompt::new(prompt));
    let mut topo = TopologicalSort::new(dp);

    let subgraph: std::collections::HashSet<String> = ["1".to_string()].into_iter().collect();
    topo.add_node("1", false, Some(&subgraph));

    assert_eq!(topo.pending_count(), 1);
}

#[test]
fn test_topological_sort_external_block() {
    let mut prompt = std::collections::HashMap::new();
    prompt.insert("1".to_string(), make_node("KSampler", vec![]));

    let dp = std::sync::Arc::new(DynamicPrompt::new(prompt));
    let mut topo = TopologicalSort::new(dp);

    topo.add_node("1", false, None);

    let ready = topo.get_ready_nodes();
    assert_eq!(ready.len(), 1);

    let guard = topo.add_external_block("1").unwrap();
    let ready = topo.get_ready_nodes();
    assert_eq!(ready.len(), 0);
    assert_eq!(topo.external_blocks(), 1);

    drop(guard);
    let ready = topo.get_ready_nodes();
    assert_eq!(ready.len(), 1);
    assert_eq!(topo.external_blocks(), 0);
}

#[tokio::test]
async fn test_execution_list_simple() {
    let mut prompt = std::collections::HashMap::new();
    prompt.insert("1".to_string(), make_node("SaveImage", vec![
        ("image", link_value("2", 0)),
    ]));
    prompt.insert("2".to_string(), make_node("KSampler", vec![]));

    let dp = std::sync::Arc::new(DynamicPrompt::new(prompt));
    let mut exec_list = ExecutionList::with_null_cache(dp);

    exec_list.add_node("1", false, None);

    let staged = exec_list.stage_node_execution().await.unwrap();
    assert_eq!(staged, Some("2".to_string()));

    exec_list.complete_node_execution();

    let staged = exec_list.stage_node_execution().await.unwrap();
    assert_eq!(staged, Some("1".to_string()));

    exec_list.complete_node_execution();

    let staged = exec_list.stage_node_execution().await.unwrap();
    assert_eq!(staged, None);
    assert!(exec_list.is_empty());
}

#[tokio::test]
async fn test_execution_list_cycle_detection() {
    let mut prompt = std::collections::HashMap::new();
    prompt.insert("1".to_string(), make_node("NodeA", vec![
        ("input", link_value("2", 0)),
    ]));
    prompt.insert("2".to_string(), make_node("NodeB", vec![
        ("input", link_value("1", 0)),
    ]));

    let dp = std::sync::Arc::new(DynamicPrompt::new(prompt));
    let mut exec_list = ExecutionList::with_null_cache(dp);

    exec_list.add_node("1", false, None);

    let result = exec_list.stage_node_execution().await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_execution_list_unstage() {
    let mut prompt = std::collections::HashMap::new();
    prompt.insert("1".to_string(), make_node("KSampler", vec![]));

    let dp = std::sync::Arc::new(DynamicPrompt::new(prompt));
    let mut exec_list = ExecutionList::with_null_cache(dp);

    exec_list.add_node("1", false, None);

    let staged = exec_list.stage_node_execution().await.unwrap();
    assert_eq!(staged, Some("1".to_string()));
    assert_eq!(exec_list.staged_node_id(), Some("1"));

    exec_list.unstage_node_execution();
    assert_eq!(exec_list.staged_node_id(), None);

    let staged = exec_list.stage_node_execution().await.unwrap();
    assert_eq!(staged, Some("1".to_string()));

    exec_list.complete_node_execution();
    assert!(exec_list.is_empty());
}

#[test]
fn test_input_type_spec_lazy() {
    let mut extra = HashMap::new();
    extra.insert("lazy".to_string(), serde_json::json!(true));

    let spec = InputTypeSpec {
        type_name: "MODEL".to_string(),
        extra,
    };
    assert!(spec.is_lazy());

    let spec_not_lazy = InputTypeSpec {
        type_name: "MODEL".to_string(),
        extra: HashMap::new(),
    };
    assert!(!spec_not_lazy.is_lazy());
}

#[test]
fn test_input_type_spec_raw_link() {
    let mut extra = HashMap::new();
    extra.insert("rawLink".to_string(), serde_json::json!(true));

    let spec = InputTypeSpec {
        type_name: "MODEL".to_string(),
        extra,
    };
    assert!(spec.is_raw_link());

    let spec_not_raw = InputTypeSpec {
        type_name: "MODEL".to_string(),
        extra: HashMap::new(),
    };
    assert!(!spec_not_raw.is_raw_link());
}

use std::collections::HashMap;
