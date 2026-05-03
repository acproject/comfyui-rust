use crate::queue::JobStatus;
use crate::state::AppState;
use crate::ws::WsMessage;
use comfy_core::NodeDefinition;
use comfy_executor::NodeEventCallback;
use std::collections::HashMap;
use std::sync::Arc;

pub async fn run_executor(state: AppState) {
    loop {
        if state.queue.is_shutdown().await {
            break;
        }

        let item = match state.queue.get_next().await {
            Some(item) => item,
            None => {
                tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
                continue;
            }
        };

        let prompt_id = item.prompt_id.clone();

        state
            .broadcaster
            .send(WsMessage::execution_start(&prompt_id));

        let mut prompt = HashMap::new();
        for (node_id, node_data) in &item.prompt {
            match serde_json::from_value::<NodeDefinition>(node_data.clone()) {
                Ok(node_def) => {
                    prompt.insert(node_id.clone(), node_def);
                }
                Err(e) => {
                    tracing::warn!("Failed to parse node {}: {}", node_id, e);
                }
            }
        }

        let dynprompt = Arc::new(comfy_core::DynamicPrompt::new(prompt));

        let broadcaster = state.broadcaster.clone();
        let pid = prompt_id.clone();
        let on_node_event: NodeEventCallback = Arc::new(move |prompt_id: &str, node_id: &str| {
            broadcaster.send(WsMessage::executing(prompt_id, Some(node_id)));
        });

        let executor = comfy_executor::Executor::new(
            state.executor.registry().clone(),
            state.executor.backend().clone(),
        )
        .with_node_event_callback(on_node_event);

        match executor.execute(dynprompt, &prompt_id).await {
            Ok(result) => {
                let mut output_json = serde_json::Map::new();
                for (node_id, node_output) in &result.outputs {
                    let mut node_output_json = serde_json::Map::new();

                    if let Some(ui) = &node_output.ui {
                        if let Some(ui_obj) = ui.as_object() {
                            for (key, value) in ui_obj {
                                node_output_json.insert(key.clone(), value.clone());
                            }
                        }
                    }

                    for (i, value) in node_output.values.iter().enumerate() {
                        if let Some(images) = value.get("images").and_then(|v| v.as_array()) {
                            node_output_json.insert(
                                "images".to_string(),
                                serde_json::Value::Array(images.clone()),
                            );
                        } else if let Some(obj) = value.as_object() {
                            if obj.contains_key("filename") {
                                let images = node_output_json
                                    .entry("images".to_string())
                                    .or_insert_with(|| serde_json::Value::Array(vec![]));
                                if let Some(arr) = images.as_array_mut() {
                                    arr.push(value.clone());
                                }
                            } else {
                                node_output_json.insert(
                                    format!("output_{}", i),
                                    value.clone(),
                                );
                            }
                        } else {
                            node_output_json.insert(
                                format!("output_{}", i),
                                value.clone(),
                            );
                        }
                    }

                    output_json.insert(
                        node_id.clone(),
                        serde_json::Value::Object(node_output_json),
                    );
                }

                state
                    .broadcaster
                    .send(WsMessage::executing(&pid, None));

                state
                    .broadcaster
                    .send(WsMessage::execution_success(&prompt_id, &serde_json::Value::Object(output_json)));

                state
                    .queue
                    .complete_current(&prompt_id, result.outputs, JobStatus::Completed)
                    .await;
            }
            Err(e) => {
                state
                    .broadcaster
                    .send(WsMessage::execution_error(&prompt_id, &e.to_string()));

                state
                    .queue
                    .complete_current(
                        &prompt_id,
                        HashMap::new(),
                        JobStatus::Failed(e.to_string()),
                    )
                    .await;
            }
        }

        let queue_info = state.queue.get_queue_info().await;
        state.broadcaster.send(WsMessage::status(queue_info, ""));
    }
}
