//! MCP tool handler: every tool helps an AI IDE build and run controllable
//! ComfyUI-Rust workflows against a live comfy-server.

use crate::client::ComfyClient;
use crate::workflow::{
    self, LinkSpec, NodeSpec, ValidationReport,
};
use rmcp::handler::server::wrapper::Parameters;
use rmcp::handler::server::ServerHandler;
use rmcp::model::{
    CallToolResult, ContentBlock, Implementation, ServerCapabilities, ServerConfig,
};
use rmcp::{tool, tool_router, ErrorData};
use serde::Deserialize;
use serde_json::{json, Map, Value};
use std::sync::Arc;

/// MCP server holding a shared REST client.
#[derive(Clone)]
pub struct ComfyMcpServer {
    client: Arc<ComfyClient>,
}

impl ComfyMcpServer {
    pub fn new(client: ComfyClient) -> Self {
        Self {
            client: Arc::new(client),
        }
    }

    pub fn client(&self) -> &ComfyClient {
        &self.client
    }

    fn json_result(value: Value) -> Result<CallToolResult, ErrorData> {
        let rendered = serde_json::to_string_pretty(&value)
            .map_err(|e| internal_err(format!("failed to serialize result: {e}")))?;
        Ok(CallToolResult::success(vec![ContentBlock::text(rendered)]))
    }
}

fn internal_err(message: impl Into<String>) -> ErrorData {
    ErrorData::new(rmcp::model::ErrorCode::INTERNAL_ERROR, message.into(), None)
}

fn client_err(e: crate::client::ClientError) -> ErrorData {
    ErrorData::new(
        rmcp::model::ErrorCode::INTERNAL_ERROR,
        format!("comfy-server request failed: {e}"),
        Some(json!({ "hint": "check COMFY_SERVER_URL and that comfy-server is running" })),
    )
}

// ---------- parameter schemas ----------

#[derive(Debug, Default, Deserialize, schemars::JsonSchema)]
struct ListNodesParams {
    /// Category prefix filter, e.g. "image", "video", "H3", "audio".
    #[serde(default)]
    category: Option<String>,
    /// Case-insensitive keyword matched against class/display name.
    #[serde(default)]
    keyword: Option<String>,
}

#[derive(Debug, Default, Deserialize, schemars::JsonSchema)]
struct NodeSchemaParams {
    /// Exact node class type, e.g. "KSampler" or "MiniMaxH3Pipeline".
    class_type: String,
}

#[derive(Debug, Default, Deserialize, schemars::JsonSchema)]
struct ListModelsParams {
    /// Model folder/type: checkpoints, diffusion_models, vae, lora, text_encoders, ...
    /// Omit to list every folder.
    #[serde(default)]
    model_type: Option<String>,
}

#[derive(Debug, Default, Deserialize, schemars::JsonSchema)]
struct TemplateParams {
    template_id: String,
}

#[derive(Debug, Default, Deserialize, schemars::JsonSchema)]
struct HistoryParams {
    #[serde(default)]
    max_items: Option<usize>,
}

#[derive(Debug, Default, Deserialize, schemars::JsonSchema)]
struct PromptIdParams {
    prompt_id: String,
}

/// Parameters for the high-level workflow builder.
#[derive(Debug, Default, Deserialize, schemars::JsonSchema)]
struct BuildWorkflowParams {
    /// Start from a built-in template id (see list_workflow_templates).
    #[serde(default)]
    template_id: Option<String>,
    /// Explicit nodes (used when template_id is omitted / to extend a template
    /// is not supported — provide the full node set together with links).
    #[serde(default)]
    nodes: Vec<NodeSpec>,
    /// Links between explicit nodes.
    #[serde(default)]
    links: Vec<LinkSpec>,
    /// Literal overrides keyed by node id -> input name -> value,
    /// e.g. {"5": {"seed": 123, "steps": 20}}. Applied after template/assembly.
    #[serde(default)]
    overrides: workflow::Overrides,
    /// When true, also run static validation and attach the report.
    #[serde(default)]
    validate: bool,
}

#[derive(Debug, Default, Deserialize, schemars::JsonSchema)]
struct ValidateWorkflowParams {
    /// The prompt graph: { node_id: { "class_type": ..., "inputs": { ... } } }.
    prompt: Map<String, Value>,
}

#[derive(Debug, Default, Deserialize, schemars::JsonSchema)]
struct SubmitWorkflowParams {
    /// The prompt graph (same format as POST /prompt's "prompt" field).
    prompt: Map<String, Value>,
    /// Optional client id for WebSocket progress correlation.
    #[serde(default)]
    client_id: Option<String>,
    /// Validate statically first and refuse to submit when errors exist.
    #[serde(default = "default_true")]
    validate_first: bool,
}

fn default_true() -> bool {
    true
}

// ---------- tools ----------

#[tool_router]
impl ComfyMcpServer {
    /// Check that comfy-server is reachable and return system statistics.
    #[tool(description = "Check comfy-server connectivity and return system stats (server URL, device info).")]
    async fn server_ping(&self) -> Result<CallToolResult, ErrorData> {
        let stats = self.client.ping().await.map_err(client_err)?;
        Self::json_result(json!({
            "ok": true,
            "server_url": self.client.base_url(),
            "system_stats": stats,
        }))
    }

    /// List registered node classes, optionally filtered.
    #[tool(
        description = "List available ComfyUI-Rust node classes (class_type, category, display_name). Use category prefix or keyword to filter; call get_node_schema for one class."
    )]
    async fn list_nodes(
        &self,
        Parameters(params): Parameters<ListNodesParams>,
    ) -> Result<CallToolResult, ErrorData> {
        let info = self.client.object_info().await.map_err(client_err)?;
        let nodes = workflow::summarize_object_info(
            &info,
            params.category.as_deref(),
            params.keyword.as_deref(),
        );
        let counts = workflow::category_counts(&info);
        Self::json_result(json!({
            "count": nodes.len(),
            "total_registered": info.as_object().map(|m| m.len()).unwrap_or(0),
            "category_counts": counts,
            "nodes": nodes,
        }))
    }

    /// Full schema for one node class.
    #[tool(
        description = "Get the full schema of one node class: required/optional inputs (types, defaults, COMBO choices), output socket names/types, category. Required before wiring a node into a workflow."
    )]
    async fn get_node_schema(
        &self,
        Parameters(params): Parameters<NodeSchemaParams>,
    ) -> Result<CallToolResult, ErrorData> {
        let value = self
            .client
            .object_info_class(&params.class_type)
            .await
            .map_err(client_err)?;
        // Server may return {ClassName: {...}} or the bare object.
        let def = value
            .get(&params.class_type)
            .cloned()
            .unwrap_or(value);
        if def.is_null() {
            return Err(ErrorData::new(
                rmcp::model::ErrorCode::INVALID_PARAMS,
                format!("node class '{}' not found", params.class_type),
                None,
            ));
        }
        Self::json_result(json!({ "class_type": params.class_type, "schema": def }))
    }

    /// List local model files.
    #[tool(
        description = "List locally installed model files per folder (checkpoints, diffusion_models, vae, lora, text_encoders, audio_encoders, ...). Omit model_type to list all folders."
    )]
    async fn list_models(
        &self,
        Parameters(params): Parameters<ListModelsParams>,
    ) -> Result<CallToolResult, ErrorData> {
        let value = self
            .client
            .list_models(params.model_type.as_deref())
            .await
            .map_err(client_err)?;
        Self::json_result(value)
    }

    /// List built-in workflow templates.
    #[tool(
        description = "List built-in workflow templates (id, name, category, description). A template is a ready-made controllable graph; use build_workflow with its template_id."
    )]
    async fn list_workflow_templates(&self) -> Result<CallToolResult, ErrorData> {
        let value = self.client.list_templates().await.map_err(client_err)?;
        Self::json_result(value)
    }

    /// Fetch one template in full.
    #[tool(
        description = "Get one workflow template with its nodes and connections in full."
    )]
    async fn get_workflow_template(
        &self,
        Parameters(params): Parameters<TemplateParams>,
    ) -> Result<CallToolResult, ErrorData> {
        let value = self
            .client
            .get_template(&params.template_id)
            .await
            .map_err(client_err)?;
        Self::json_result(value)
    }

    /// Build a controllable workflow graph (without submitting it).
    #[tool(
        description = "Build a controllable workflow prompt graph WITHOUT submitting. Either start from template_id and apply overrides ({nodeId:{input:value}}), or provide explicit nodes and links (links are [from_node, from_socket] wired to to_input). Returns the graph ready for validate_workflow/submit_workflow."
    )]
    async fn build_workflow(
        &self,
        Parameters(mut params): Parameters<BuildWorkflowParams>,
    ) -> Result<CallToolResult, ErrorData> {
        let mut graph = if let Some(template_id) = params.template_id.as_deref() {
            let template = self.client.get_template(template_id).await.map_err(client_err)?;
            let info = self.client.object_info().await.map_err(client_err)?;
            workflow::template_to_graph(&template, &info).map_err(|e| {
                ErrorData::new(
                    rmcp::model::ErrorCode::INVALID_PARAMS,
                    format!("failed to convert template '{template_id}': {e}"),
                    None,
                )
            })?
        } else {
            if params.nodes.is_empty() {
                return Err(ErrorData::new(
                    rmcp::model::ErrorCode::INVALID_PARAMS,
                    "provide either template_id or at least one node",
                    None,
                ));
            }
            workflow::assemble_graph(&params.nodes, &params.links).map_err(|e| {
                ErrorData::new(rmcp::model::ErrorCode::INVALID_PARAMS, e, None)
            })?
        };

        workflow::apply_overrides(&mut graph, &params.overrides);

        let mut report: Option<ValidationReport> = None;
        if params.validate {
            let info = self.client.object_info().await.map_err(client_err)?;
            report = Some(workflow::validate_graph(&graph, &info));
        }

        // Silence unused field (template mode ignores nodes/links by design).
        params.nodes.clear();
        params.links.clear();

        Self::json_result(json!({
            "prompt": graph,
            "validation": report,
            "hint": "review the graph, then call validate_workflow and submit_workflow",
        }))
    }

    /// Statically validate a workflow graph.
    #[tool(
        description = "Statically validate a prompt graph against the live node registry: unknown classes, missing required inputs, bad COMBO choices, broken links, missing output nodes. Does not execute anything."
    )]
    async fn validate_workflow(
        &self,
        Parameters(params): Parameters<ValidateWorkflowParams>,
    ) -> Result<CallToolResult, ErrorData> {
        let graph = Value::Object(params.prompt);
        let info = self.client.object_info().await.map_err(client_err)?;
        let report = workflow::validate_graph(&graph, &info);
        Self::json_result(json!(report))
    }

    /// Submit a workflow for execution.
    #[tool(
        description = "Submit a prompt graph for execution (POST /prompt). Validates statically first unless validate_first=false. Returns prompt_id; poll get_prompt_status."
    )]
    async fn submit_workflow(
        &self,
        Parameters(params): Parameters<SubmitWorkflowParams>,
    ) -> Result<CallToolResult, ErrorData> {
        let graph = Value::Object(params.prompt);
        if params.validate_first {
            let info = self.client.object_info().await.map_err(client_err)?;
            let report = workflow::validate_graph(&graph, &info);
            if !report.valid {
                return Self::json_result(json!({
                    "submitted": false,
                    "reason": "static validation failed",
                    "validation": report,
                }));
            }
        }
        match self
            .client
            .submit_prompt(graph, params.client_id.as_deref())
            .await
        {
            Ok(resp) => Self::json_result(json!({ "submitted": true, "response": resp })),
            Err(e) => Err(client_err(e)),
        }
    }

    /// Current queue (running + pending).
    #[tool(description = "Get the execution queue: currently running and pending prompts.")]
    async fn get_queue(&self) -> Result<CallToolResult, ErrorData> {
        let value = self.client.queue().await.map_err(client_err)?;
        Self::json_result(value)
    }

    /// Recent execution history.
    #[tool(description = "Get recent prompt execution history (statuses, outputs).")]
    async fn get_history(&self, Parameters(params): Parameters<HistoryParams>) -> Result<CallToolResult, ErrorData> {
        let value = self
            .client
            .history(params.max_items)
            .await
            .map_err(client_err)?;
        Self::json_result(value)
    }

    /// Status + outputs of one prompt.
    #[tool(
        description = "Get the status and produced outputs (images/videos/audio filenames) of one prompt_id."
    )]
    async fn get_prompt_status(
        &self,
        Parameters(params): Parameters<PromptIdParams>,
    ) -> Result<CallToolResult, ErrorData> {
        match self.client.history_by_id(&params.prompt_id).await {
            Ok(value) => Self::json_result(value),
            Err(crate::client::ClientError::Http { status: 404, .. }) => {
                Self::json_result(json!({
                    "prompt_id": params.prompt_id,
                    "status": "unknown_or_running",
                    "hint": "not in history yet — still queued/running, or invalid id",
                }))
            }
            Err(e) => Err(client_err(e)),
        }
    }

    /// Interrupt the current generation.
    #[tool(description = "Interrupt the currently running generation (POST /interrupt).")]
    async fn interrupt(&self) -> Result<CallToolResult, ErrorData> {
        self.client.interrupt().await.map_err(client_err)?;
        Self::json_result(json!({ "interrupted": true }))
    }
}

#[rmcp::tool_handler]
impl ServerHandler for ComfyMcpServer {
    fn get_info(&self) -> ServerConfig {
        ServerConfig::new(
            ServerCapabilities::builder().enable_tools().build(),
        )
        .with_server_info(Implementation::new(
            "comfyui-rust-mcp",
            env!("CARGO_PKG_VERSION"),
        ))
        .with_instructions(
            "ComfyUI-Rust workflow MCP. Build controllable generation flows: \
             list_nodes/get_node_schema -> list_models -> build_workflow -> \
             validate_workflow -> submit_workflow -> get_prompt_status.",
        )
    }
}
