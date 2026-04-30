import type {
  PromptRequest,
  PromptResponse,
  QueueInfo,
  HistoryEntry,
  ObjectInfoMap,
  NodeClassDef,
  ImageListResponse,
  UploadImageResponse,
  SaveWorkflowRequest,
  SaveWorkflowResponse,
  WorkflowListResponse,
  ServerConfig,
} from '@/types/api';

const API_BASE = '';

async function fetchJson<T>(url: string, options?: RequestInit): Promise<T> {
  const response = await fetch(`${API_BASE}${url}`, {
    headers: {
      'Content-Type': 'application/json',
      ...options?.headers,
    },
    ...options,
  });

  if (!response.ok) {
    const error = await response.json().catch(() => ({ error: { message: response.statusText } }));
    throw new Error(error.error?.message || `HTTP ${response.status}`);
  }

  return response.json();
}

export const api = {
  async submitPrompt(request: PromptRequest): Promise<PromptResponse> {
    return fetchJson<PromptResponse>('/prompt', {
      method: 'POST',
      body: JSON.stringify(request),
    });
  },

  async getQueueInfo(): Promise<QueueInfo> {
    return fetchJson<QueueInfo>('/prompt');
  },

  async getQueue(): Promise<QueueInfo> {
    return fetchJson<QueueInfo>('/queue');
  },

  async clearQueue(): Promise<void> {
    await fetchJson('/queue', {
      method: 'POST',
      body: JSON.stringify({ clear: true }),
    });
  },

  async deleteQueueItem(promptId: string): Promise<void> {
    await fetchJson('/queue', {
      method: 'POST',
      body: JSON.stringify({ delete: [promptId] }),
    });
  },

  async interrupt(): Promise<void> {
    await fetch('/interrupt', { method: 'POST' });
  },

  async getHistory(maxItems?: number, offset?: number): Promise<Record<string, HistoryEntry>> {
    const params = new URLSearchParams();
    if (maxItems !== undefined) params.set('max_items', String(maxItems));
    if (offset !== undefined) params.set('offset', String(offset));
    const qs = params.toString();
    return fetchJson<Record<string, HistoryEntry>>(`/history${qs ? `?${qs}` : ''}`);
  },

  async getHistoryById(promptId: string): Promise<Record<string, HistoryEntry>> {
    return fetchJson<Record<string, HistoryEntry>>(`/history/${promptId}`);
  },

  async clearHistory(): Promise<void> {
    await fetchJson('/history', {
      method: 'POST',
      body: JSON.stringify({ clear: true }),
    });
  },

  async deleteHistoryItem(promptId: string): Promise<void> {
    await fetchJson('/history', {
      method: 'POST',
      body: JSON.stringify({ delete: [promptId] }),
    });
  },

  async getObjectInfo(): Promise<ObjectInfoMap> {
    return fetchJson<ObjectInfoMap>('/object_info');
  },

  async getNodeInfo(classType: string): Promise<NodeClassDef> {
    return fetchJson<NodeClassDef>(`/object_info/${classType}`);
  },

  async getSystemStats(): Promise<unknown> {
    return fetchJson('/system_stats');
  },

  async getEmbeddings(): Promise<string[]> {
    return fetchJson('/embeddings');
  },

  async getModels(): Promise<string[]> {
    return fetchJson('/models');
  },

  async getExtensions(): Promise<string[]> {
    return fetchJson('/extensions');
  },

  getImageUrl(filename: string, subfolder?: string, type?: string): string {
    const params = new URLSearchParams();
    params.set('filename', filename);
    if (subfolder) params.set('subfolder', subfolder);
    if (type) params.set('type', type);
    return `${API_BASE}/view?${params.toString()}`;
  },

  async getImage(filename: string, subfolder?: string): Promise<Blob> {
    const params = new URLSearchParams();
    params.set('filename', filename);
    if (subfolder) params.set('subfolder', subfolder);
    const response = await fetch(`${API_BASE}/view?${params.toString()}`);
    if (!response.ok) throw new Error(`Failed to fetch image: ${response.statusText}`);
    return response.blob();
  },

  async listImages(subfolder?: string): Promise<ImageListResponse> {
    const params = new URLSearchParams();
    if (subfolder) params.set('subfolder', subfolder);
    const qs = params.toString();
    return fetchJson<ImageListResponse>(`/list_images${qs ? `?${qs}` : ''}`);
  },

  async uploadImage(file: File, subfolder?: string): Promise<UploadImageResponse> {
    const formData = new FormData();
    formData.append('image', file);
    if (subfolder) formData.append('subfolder', subfolder);

    const response = await fetch(`${API_BASE}/upload/image`, {
      method: 'POST',
      body: formData,
    });

    if (!response.ok) {
      const error = await response.json().catch(() => ({ error: { message: response.statusText } }));
      throw new Error(error.error?.message || `Upload failed: ${response.status}`);
    }

    return response.json();
  },

  async saveWorkflow(request: SaveWorkflowRequest): Promise<SaveWorkflowResponse> {
    return fetchJson<SaveWorkflowResponse>('/workflow', {
      method: 'POST',
      body: JSON.stringify(request),
    });
  },

  async loadWorkflow(name: string): Promise<unknown> {
    return fetchJson(`/workflow?name=${encodeURIComponent(name)}`);
  },

  async listWorkflows(): Promise<WorkflowListResponse> {
    return fetchJson<WorkflowListResponse>('/workflows');
  },

  async getConfig(): Promise<ServerConfig> {
    return fetchJson<ServerConfig>('/config');
  },

  async getInputImages(): Promise<{ images: string[] }> {
    return fetchJson<{ images: string[] }>('/input_images');
  },

  async uploadInputImage(file: File, subfolder?: string): Promise<{ name: string; subfolder: string; type: string }> {
    const formData = new FormData();
    formData.append('image', file);
    if (subfolder) formData.append('subfolder', subfolder);

    const response = await fetch(`${API_BASE}/upload/input_image`, {
      method: 'POST',
      body: formData,
    });

    if (!response.ok) {
      const error = await response.json().catch(() => ({ error: { message: response.statusText } }));
      throw new Error(error.error?.message || `Upload failed: ${response.status}`);
    }

    return response.json();
  },

  getInputImageUrl(filename: string, subfolder?: string): string {
    const params = new URLSearchParams();
    params.set('filename', filename);
    if (subfolder) params.set('subfolder', subfolder);
    return `${API_BASE}/view_input?${params.toString()}`;
  },

  async listCustomNodes(): Promise<{ nodes: Array<{ filename: string; definition: unknown }> }> {
    return fetchJson('/custom_nodes');
  },

  async saveCustomNode(filename: string, definition: unknown): Promise<{ filename: string; path: string }> {
    return fetchJson('/custom_nodes', {
      method: 'POST',
      body: JSON.stringify({ filename, definition }),
    });
  },

  async deleteCustomNode(filename: string): Promise<void> {
    const response = await fetch(`${API_BASE}/custom_nodes/${encodeURIComponent(filename)}`, {
      method: 'DELETE',
      headers: { 'Content-Type': 'application/json' },
    });
    if (!response.ok) {
      const error = await response.json().catch(() => ({ error: { message: response.statusText } }));
      throw new Error(error.error?.message || `Delete failed: ${response.status}`);
    }
  },
};
