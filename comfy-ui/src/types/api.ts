export interface NodeDefinition {
  class_type: string;
  inputs: Record<string, InputValue>;
  is_changed?: unknown;
}

export type InputValue = string | number | boolean | null | LinkValue;

export type LinkValue = [string, number];

export function isLink(value: InputValue): value is LinkValue {
  return Array.isArray(value) && value.length === 2 && typeof value[0] === 'string' && typeof value[1] === 'number';
}

export interface NodeClassDef {
  class_type: string;
  display_name: string;
  category: string;
  input_types: NodeInputTypes;
  output_types: IoType[];
  output_names: string[];
  output_is_list: boolean[];
  is_output_node: boolean;
  has_intermediate_output: boolean;
  function_name: string;
}

export interface NodeInputTypes {
  required: Record<string, InputTypeSpec>;
  optional?: Record<string, InputTypeSpec>;
  hidden?: Record<string, InputTypeSpec>;
}

export interface InputTypeSpec {
  type_name: string;
  extra: Record<string, unknown>;
}

export type IoType =
  | '*'
  | 'STRING'
  | 'INT'
  | 'FLOAT'
  | 'BOOLEAN'
  | 'MODEL'
  | 'CLIP'
  | 'VAE'
  | 'IMAGE'
  | 'MASK'
  | 'LATENT'
  | 'CONDITIONING'
  | 'CONTROL_NET'
  | string;

export interface PromptRequest {
  prompt: Record<string, NodeDefinition>;
  extra_data?: Record<string, unknown>;
  client_id?: string;
  prompt_id?: string;
  front?: boolean;
}

export interface PromptResponse {
  prompt_id: string;
  number: number;
  node_errors: Record<string, NodeErrorInfo>;
}

export interface NodeErrorInfo {
  node_id: string;
  class_type: string;
  errors: ErrorDetail[];
}

export interface ErrorDetail {
  error_type: string;
  message: string;
  details: string;
}

export interface QueueInfo {
  queue_running: QueueItem[];
  queue_pending: QueueItem[];
}

export interface QueueItem {
  number: number;
  prompt_id: string;
}

export interface HistoryEntry {
  prompt_id: string;
  prompt: Record<string, NodeDefinition>;
  outputs: Record<string, unknown>;
  status: JobStatus;
  created_at: number;
  completed_at?: number;
}

export type JobStatus = 'Pending' | 'Running' | 'Completed' | { Failed: string } | 'Interrupted';

export interface WsMessage {
  type: string;
  data: unknown;
}

export interface ExecutionStartData {
  prompt_id: string;
}

export interface ExecutingData {
  prompt_id: string;
  node: string | null;
}

export interface ProgressData {
  prompt_id: string;
  value: number;
  max: number;
}

export interface ExecutionCachedData {
  prompt_id: string;
  nodes: string[];
}

export interface StatusData {
  status: QueueInfo;
  sid: string;
}

export type NodeCategory = string;

export interface ObjectInfoMap {
  [classType: string]: NodeClassDef;
}

export interface ImageEntry {
  filename: string;
  subfolder: string;
  type: string;
}

export interface ImageListResponse {
  images: ImageEntry[];
}

export interface UploadImageResponse {
  name: string;
  subfolder: string;
  type: string;
}

export interface SaveWorkflowRequest {
  name: string;
  workflow: unknown;
  description?: string;
}

export interface SaveWorkflowResponse {
  name: string;
  path: string;
}

export interface WorkflowListItem {
  name: string;
  size: number;
  modified: number | null;
}

export interface WorkflowListResponse {
  workflows: WorkflowListItem[];
}

export interface ServerConfig {
  server: {
    host: string;
    port: number;
    cors_origins: string[];
    static_dir: string | null;
  };
  models: {
    base_dir: string;
    checkpoints: string;
    clip: string;
    vae: string;
    lora: string;
    controlnet: string;
    upscale: string;
    embeddings: string;
    text_encoders: string;
    diffusion_models: string;
    clip_vision: string;
    style_models: string;
    diffusers: string;
    vae_approx: string;
    gligen: string;
    latent_upscale_models: string;
    hypernetworks: string;
    photomarker: string;
    classifiers: string;
    model_patches: string;
    audio_encoders: string;
  };
  inference: {
    backend: string;
    n_threads: number;
    vae_decode_only: boolean;
    free_params_immediately: boolean;
    enable_mmap: boolean;
    flash_attn: boolean;
    offload_params_to_cpu: boolean;
    remote_url: string | null;
    sd_cli_path: string | null;
  };
  output: {
    dir: string;
    save_metadata: boolean;
    format: string;
  };
  extra?: Record<string, unknown>;
}

export interface AgentConfig {
  enabled: boolean;
  provider: string;
  api_url: string;
  api_key: string | null;
  model: string;
  max_tokens: number;
  temperature: number;
  system_prompt: string;
}

export interface AgentChatMessage {
  role: 'user' | 'assistant' | 'system';
  content: string;
}

export interface WorkflowNodeInfo {
  id: string;
  class_type: string;
  title: string;
  inputs: Record<string, unknown>;
  outputs: Array<{ name: string; type_name: string }>;
}

export interface WorkflowEdgeInfo {
  source: string;
  source_handle: string;
  target: string;
  target_handle: string;
}

export interface AgentChatContext {
  available_nodes: string[];
  current_workflow_nodes: WorkflowNodeInfo[];
  current_workflow_edges: WorkflowEdgeInfo[];
}

export interface AgentChatRequest {
  messages: AgentChatMessage[];
  context?: AgentChatContext;
}

export interface AgentAction {
  type: 'add_node' | 'connect' | 'set_param' | 'run_workflow' | 'validate_workflow' | 'clear_workflow';
  payload: Record<string, unknown>;
}

export interface AgentChatResponse {
  message: AgentChatMessage;
  actions: AgentAction[];
}

export interface AgentModelInfo {
  id: string;
  owned_by: string;
}

export interface AgentModelsResponse {
  models: AgentModelInfo[];
}

export interface ModelFileInfo {
  name: string;
  path: string;
  size: number;
  modified: number | null;
}

export type ModelTypeMap = Record<string, ModelFileInfo[]>;

export interface ModelListResponse extends ModelTypeMap {}

export interface DeleteModelRequest {
  model_type: string;
  path: string;
}

export interface DeleteModelResponse {
  success: boolean;
  path: string;
}

export interface UploadModelResponse {
  name: string;
  model_type: string;
  path: string;
}
