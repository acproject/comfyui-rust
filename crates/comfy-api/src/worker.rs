use crate::queue::JobStatus;
use crate::state::AppState;
use crate::ws::WsMessage;
use comfy_core::NodeDefinition;
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

        match state.executor.execute(dynprompt, &prompt_id).await {
            Ok(result) => {
                for node_id in &result.executed {
                    state
                        .broadcaster
                        .send(WsMessage::executing(&prompt_id, Some(node_id)));
                }

                let output_json = serde_json::to_value(&result.outputs).unwrap_or(serde_json::json!({}));

                state
                    .broadcaster
                    .send(WsMessage::execution_success(&prompt_id, &output_json));

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
