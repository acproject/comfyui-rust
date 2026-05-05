use crate::model_knowledge;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::RwLock;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentConfig {
    #[serde(default = "default_enabled")]
    pub enabled: bool,
    #[serde(default = "default_provider")]
    pub provider: String,
    #[serde(default = "default_api_url")]
    pub api_url: String,
    #[serde(default)]
    pub api_key: Option<String>,
    #[serde(default = "default_model")]
    pub model: String,
    #[serde(default = "default_max_tokens")]
    pub max_tokens: u32,
    #[serde(default = "default_temperature")]
    pub temperature: f64,
    #[serde(default = "default_system_prompt")]
    pub system_prompt: String,
}

impl Default for AgentConfig {
    fn default() -> Self {
        Self {
            enabled: default_enabled(),
            provider: default_provider(),
            api_url: default_api_url(),
            api_key: None,
            model: default_model(),
            max_tokens: default_max_tokens(),
            temperature: default_temperature(),
            system_prompt: default_system_prompt(),
        }
    }
}

impl AgentConfig {
    pub fn from_env() -> Self {
        let mut config = Self::default();

        if let Ok(val) = std::env::var("COMFY_AGENT_ENABLED") {
            config.enabled = val == "true" || val == "1";
        }
        if let Ok(val) = std::env::var("COMFY_AGENT_PROVIDER") {
            config.provider = val;
        }
        if let Ok(val) = std::env::var("COMFY_AGENT_API_URL") {
            config.api_url = val;
        }
        if let Ok(val) = std::env::var("COMFY_AGENT_API_KEY") {
            config.api_key = Some(val);
        }
        if let Ok(val) = std::env::var("COMFY_AGENT_MODEL") {
            config.model = val;
        }
        if let Ok(val) = std::env::var("COMFY_AGENT_MAX_TOKENS") {
            if let Ok(n) = val.parse() {
                config.max_tokens = n;
            }
        }
        if let Ok(val) = std::env::var("COMFY_AGENT_TEMPERATURE") {
            if let Ok(t) = val.parse() {
                config.temperature = t;
            }
        }

        config
    }
}

fn default_enabled() -> bool {
    false
}

fn default_provider() -> String {
    "openai".to_string()
}

fn default_api_url() -> String {
    "https://api.openai.com/v1".to_string()
}

fn default_model() -> String {
    "gpt-4o-mini".to_string()
}

fn default_max_tokens() -> u32 {
    4096
}

fn default_temperature() -> f64 {
    0.7
}

fn default_system_prompt() -> String {
    let knowledge = model_knowledge::get_model_knowledge_base();
    let templates = model_knowledge::get_workflow_templates();

    let mut model_section = String::from("## Model Knowledge Base\n\nYou have knowledge of the following popular Stable Diffusion models:\n\n");

    for model in &knowledge {
        model_section.push_str(&format!(
            "### {}\n- Category: {:?}\n- Description: {}\n- Base Model: {:?}\n",
            model.name, model.category, model.description, model.base_model
        ));

        if !model.trigger_tokens.is_empty() {
            model_section.push_str(&format!("- Trigger Tokens: {}\n", model.trigger_tokens.join(", ")));
        }

        model_section.push_str(&format!(
            "- Recommended: steps={}, cfg={}, sampler={}, scheduler={}, resolution={}, denoise={}\n",
            model.recommended_settings.steps,
            model.recommended_settings.cfg,
            model.recommended_settings.sampler,
            model.recommended_settings.scheduler,
            model.recommended_settings.resolution,
            model.recommended_settings.denoise,
        ));

        if !model.style_tags.is_empty() {
            model_section.push_str(&format!("- Style Tags: {}\n", model.style_tags.join(", ")));
        }

        if !model.negative_prompt_suggestion.is_empty() {
            model_section.push_str(&format!("- Suggested Negative Prompt: {}\n", model.negative_prompt_suggestion));
        }

        model_section.push('\n');
    }

    let mut template_section = String::from("## Workflow Templates\n\nAvailable workflow templates:\n\n");
    for template in &templates {
        template_section.push_str(&format!(
            "- **{}** (id: `{}`): {}\n",
            template.name, template.id, template.description
        ));
    }

    format!(
r#"You are a ComfyUI AI Assistant — an expert in Stable Diffusion image generation workflows. You help users build, modify, and optimize image generation workflows.

## Core Capabilities

You can perform the following actions by including JSON blocks in your response:

1. **Add a node**: ```action
{{"type": "add_node", "payload": {{"classType": "NodeClassName", "x": 100, "y": 100}}}}
```
2. **Connect nodes**: ```action
{{"type": "connect", "payload": {{"sourceId": "1", "sourceHandle": "output_name", "targetId": "2", "targetHandle": "input_name"}}}}
```
3. **Set a parameter**: ```action
{{"type": "set_param", "payload": {{"nodeId": "1", "inputName": "param_name", "value": "value"}}}}
```
4. **Load a workflow template**: ```action
{{"type": "load_workflow_template", "payload": {{"templateId": "txt2img_sd15"}}}}
```
5. **Recommend a model**: ```action
{{"type": "recommend_model", "payload": {{"modelName": "ChilloutMix", "reason": "Best for photorealistic portraits"}}}}
```
6. **Run workflow**: ```action
{{"type": "run_workflow", "payload": {{}}}}
```
7. **Validate workflow**: ```action
{{"type": "validate_workflow", "payload": {{}}}}
```
8. **Clear workflow**: ```action
{{"type": "clear_workflow", "payload": {{}}}}
```

## Common Node Types

### Model Loading
- **CheckpointLoaderSimple**: Load a model checkpoint (outputs: MODEL, CLIP, VAE)
- **UNETLoader**: Load a diffusion model/UNET (inputs: unet_name, weight_dtype; outputs: MODEL)
- **DualCLIPLoader**: Load dual CLIP models for SDXL/Flux (inputs: clip_name1, clip_name2, type; outputs: CLIP)
- **VAELoader**: Load a standalone VAE (inputs: vae_name; outputs: VAE)
- **LoraLoader**: Load a LoRA (inputs: model, clip, lora_name, strength_model, strength_clip; outputs: MODEL, CLIP)

### Text Encoding
- **CLIPTextEncode**: Encode text prompt (inputs: clip, text; outputs: CONDITIONING)
- **CLIPSetLastLayer**: Set CLIP layer (inputs: clip, stop_at_clip_layer; outputs: CLIP)

### Latent Operations
- **EmptyLatentImage**: Create empty latent (inputs: width, height, batch_size; outputs: LATENT)
- **VAEEncode**: Encode image to latent (inputs: pixels, vae; outputs: LATENT)
- **VAEDecode**: Decode latent to image (inputs: samples, vae; outputs: IMAGE)

### Sampling
- **KSampler**: Sample latent space (inputs: model, positive, negative, latent_image, seed, steps, cfg, sampler_name, scheduler, denoise; outputs: LATENT)

### Image I/O
- **SaveImage**: Save output image (inputs: images, filename_prefix)
- **LoadImage**: Load an input image (inputs: image; outputs: IMAGE, MASK)
- **PreviewImage**: Preview image without saving (inputs: images)

### Upscaling
- **UpscaleModelLoader**: Load upscale model (inputs: model_name; outputs: UPSCALE_MODEL)
- **ImageUpscaleWithModel**: Upscale image with model (inputs: upscale_model, image; outputs: IMAGE)

### ControlNet
- **ControlNetLoader**: Load ControlNet model (inputs: control_net_name; outputs: CONTROL_NET)
- **ApplyControlNet**: Apply ControlNet (inputs: conditioning, control_net, image; outputs: CONDITIONING)

### Mask Operations
- **SetLatentNoiseMask**: Apply mask to latent (inputs: samples, mask; outputs: LATENT)

{model_section}

{template_section}

## Prompt Engineering Tips

1. **Positive Prompts**: Be descriptive and specific. Use quality tags like "masterpiece, best quality, highly detailed" for better results.
2. **Negative Prompts**: Always include negative prompts to avoid unwanted artifacts. Common negative prompts include: "lowres, bad anatomy, bad hands, text, error, missing fingers, extra digit, fewer digits, cropped, worst quality, low quality, normal quality, jpeg artifacts, signature, watermark, username, blurry"
3. **Trigger Tokens**: Some models require specific trigger tokens to activate their style (listed in model knowledge above). Always include them in your prompt when using those models.
4. **CFG Scale**: Higher CFG (7-12) follows the prompt more closely; lower CFG (3-5) gives more creative freedom. FLUX models work best with CFG 3.5.
5. **Steps**: More steps = better quality but slower. 20-30 steps is usually sufficient. FLUX schnell only needs 4 steps.
6. **Samplers**: 
   - dpmpp_2m + karras: Best general-purpose combo for SD 1.5/SDXL
   - euler_ancestral + karras: Good for anime/artistic styles
   - euler + simple: Best for FLUX models
7. **Resolution**: SD 1.5 works best at 512x768; SDXL/FLUX at 1024x1024

## Workflow Building Guidelines

When a user asks to create a workflow:
1. First, understand what type of image they want (anime, realistic, artistic, etc.)
2. Recommend an appropriate model based on their needs
3. Use the `load_workflow_template` action to create the base workflow, then customize it
4. Or build the workflow step by step using `add_node` and `connect` actions
5. Set appropriate parameters based on the model's recommended settings
6. Include helpful negative prompts
7. Always explain what you're doing and why

When a user asks for model recommendations:
1. Ask about their desired style (anime, realistic, artistic, etc.)
2. Consider their hardware constraints (SD 1.5 models need less VRAM than SDXL/FLUX)
3. Use the `recommend_model` action to suggest appropriate models
4. Explain the model's strengths and trigger tokens

Always respond in the same language as the user."#,
        model_section = model_section,
        template_section = template_section,
    )
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage {
    pub role: String,
    pub content: String,
}

#[derive(Debug, Deserialize)]
pub struct ChatRequest {
    pub messages: Vec<ChatMessage>,
    #[serde(default)]
    pub context: Option<ChatContext>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatContext {
    #[serde(default)]
    pub available_nodes: Vec<String>,
    #[serde(default)]
    pub current_workflow_nodes: Vec<WorkflowNodeInfo>,
    #[serde(default)]
    pub current_workflow_edges: Vec<WorkflowEdgeInfo>,
    #[serde(default)]
    pub available_checkpoints: Vec<String>,
    #[serde(default)]
    pub available_loras: Vec<String>,
    #[serde(default)]
    pub available_vae: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowNodeInfo {
    pub id: String,
    pub class_type: String,
    pub title: String,
    pub inputs: serde_json::Value,
    pub outputs: Vec<OutputInfo>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OutputInfo {
    pub name: String,
    pub type_name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowEdgeInfo {
    pub source: String,
    pub source_handle: String,
    pub target: String,
    pub target_handle: String,
}

#[derive(Debug, Serialize)]
pub struct ChatResponse {
    pub message: ChatMessage,
    pub actions: Vec<AgentAction>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentAction {
    #[serde(rename = "type")]
    pub action_type: String,
    pub payload: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelInfo {
    pub id: String,
    #[serde(default)]
    pub owned_by: String,
}

pub struct AgentService {
    config: Arc<RwLock<AgentConfig>>,
    client: reqwest::Client,
}

impl AgentService {
    pub fn new(config: AgentConfig) -> Self {
        Self {
            config: Arc::new(RwLock::new(config)),
            client: reqwest::Client::new(),
        }
    }

    pub fn config_arc(&self) -> Arc<RwLock<AgentConfig>> {
        self.config.clone()
    }

    pub async fn get_config(&self) -> AgentConfig {
        self.config.read().await.clone()
    }

    pub async fn set_config(&self, new_config: AgentConfig) {
        *self.config.write().await = new_config;
    }

    pub async fn chat(&self, request: ChatRequest) -> Result<ChatResponse, AgentError> {
        let config = self.config.read().await.clone();

        if !config.enabled {
            return Err(AgentError::Disabled);
        }

        let api_key = config
            .api_key
            .as_deref()
            .ok_or(AgentError::NoApiKey)?;

        let mut messages = Vec::new();

        let mut system_prompt = if config.system_prompt.is_empty() {
            default_system_prompt()
        } else {
            config.system_prompt.clone()
        };

        if let Some(ctx) = &request.context {
            system_prompt = Self::enrich_system_prompt(&system_prompt, ctx);
        }
        messages.push(ChatMessage {
            role: "system".to_string(),
            content: system_prompt,
        });

        messages.extend(request.messages);

        let body = serde_json::json!({
            "model": config.model,
            "messages": messages,
            "max_tokens": config.max_tokens,
            "temperature": config.temperature,
        });

        let url = format!("{}/chat/completions", config.api_url.trim_end_matches('/'));
        tracing::info!("Agent chat: calling {} with model={}", url, config.model);

        let response = self
            .client
            .post(&url)
            .header("Authorization", format!("Bearer {}", api_key))
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await
            .map_err(|e| AgentError::RequestFailed(e.to_string()))?;

        if !response.status().is_success() {
            let status = response.status();
            let text = response.text().await.unwrap_or_default();
            return Err(AgentError::LlmError(format!(
                "LLM API returned {}: {}",
                status, text
            )));
        }

        let response_json: serde_json::Value = response
            .json()
            .await
            .map_err(|e| AgentError::ParseError(e.to_string()))?;

        let content = response_json["choices"][0]["message"]["content"]
            .as_str()
            .ok_or_else(|| AgentError::ParseError("No content in LLM response".to_string()))?
            .to_string();

        let actions = Self::parse_actions(&content);

        Ok(ChatResponse {
            message: ChatMessage {
                role: "assistant".to_string(),
                content,
            },
            actions,
        })
    }

    pub async fn list_models(&self) -> Result<Vec<ModelInfo>, AgentError> {
        let config = self.config.read().await.clone();

        if config.api_key.is_none() || config.api_key.as_deref() == Some("") {
            return Ok(vec![]);
        }

        let url = format!("{}/models", config.api_url.trim_end_matches('/'));

        let mut request = self
            .client
            .get(&url)
            .header("Content-Type", "application/json");

        if let Some(ref api_key) = config.api_key {
            request = request.header("Authorization", format!("Bearer {}", api_key));
        }

        let response = request
            .send()
            .await
            .map_err(|e| AgentError::RequestFailed(e.to_string()))?;

        if !response.status().is_success() {
            let status = response.status();
            let text = response.text().await.unwrap_or_default();
            return Err(AgentError::LlmError(format!(
                "Models API returned {}: {}",
                status, text
            )));
        }

        let response_json: serde_json::Value = response
            .json()
            .await
            .map_err(|e| AgentError::ParseError(e.to_string()))?;

        let models = if let Some(data) = response_json.get("data").and_then(|d| d.as_array()) {
            data.iter()
                .filter_map(|item| {
                    let id = item.get("id")?.as_str()?.to_string();
                    let owned = item.get("owned_by").and_then(|v| v.as_str()).unwrap_or("").to_string();
                    Some(ModelInfo { id, owned_by: owned })
                })
                .collect()
        } else {
            vec![]
        };

        Ok(models)
    }

    fn enrich_system_prompt(base: &str, ctx: &ChatContext) -> String {
        let mut enriched = base.to_string();

        if !ctx.available_nodes.is_empty() {
            enriched.push_str("\n\nAvailable node types on this server:\n");
            for node_type in &ctx.available_nodes {
                enriched.push_str(&format!("- {}\n", node_type));
            }
        }

        if !ctx.current_workflow_nodes.is_empty() {
            enriched.push_str("\n\nCurrent workflow nodes:\n");
            for node in &ctx.current_workflow_nodes {
                enriched.push_str(&format!(
                    "- Node {} ({}): {}\n",
                    node.id, node.class_type, node.title
                ));
            }
        }

        if !ctx.current_workflow_edges.is_empty() {
            enriched.push_str("\n\nCurrent workflow connections:\n");
            for edge in &ctx.current_workflow_edges {
                enriched.push_str(&format!(
                    "- {}.{} -> {}.{}\n",
                    edge.source, edge.source_handle, edge.target, edge.target_handle
                ));
            }
        }

        if !ctx.available_checkpoints.is_empty() {
            enriched.push_str("\n\nAvailable checkpoint models:\n");
            for ckpt in &ctx.available_checkpoints {
                enriched.push_str(&format!("- {}\n", ckpt));
            }
        }

        if !ctx.available_loras.is_empty() {
            enriched.push_str("\n\nAvailable LoRA models:\n");
            for lora in &ctx.available_loras {
                enriched.push_str(&format!("- {}\n", lora));
            }
        }

        if !ctx.available_vae.is_empty() {
            enriched.push_str("\n\nAvailable VAE models:\n");
            for vae in &ctx.available_vae {
                enriched.push_str(&format!("- {}\n", vae));
            }
        }

        enriched
    }

    fn parse_actions(text: &str) -> Vec<AgentAction> {
        let mut actions = Vec::new();
        let action_regex = regex::Regex::new(r"```action\n([\s\S]*?)```").unwrap();

        for cap in action_regex.captures_iter(text) {
            if let Some(action_str) = cap.get(1) {
                match serde_json::from_str::<AgentAction>(action_str.as_str()) {
                    Ok(action) => actions.push(action),
                    Err(_) => continue,
                }
            }
        }

        actions
    }
}

#[derive(Debug, thiserror::Error)]
pub enum AgentError {
    #[error("AI Agent is disabled")]
    Disabled,
    #[error("No API key configured")]
    NoApiKey,
    #[error("Request failed: {0}")]
    RequestFailed(String),
    #[error("LLM error: {0}")]
    LlmError(String),
    #[error("Parse error: {0}")]
    ParseError(String),
}
