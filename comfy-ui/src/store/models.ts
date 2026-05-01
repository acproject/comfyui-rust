import { create } from 'zustand';
import type { ModelFileInfo, ModelTypeMap } from '@/types/api';
import { api } from '@/api/client';

const MODEL_TYPES = [
  'checkpoints',
  'loras',
  'vae',
  'text_encoders',
  'diffusion_models',
  'clip_vision',
  'style_models',
  'embeddings',
  'diffusers',
  'vae_approx',
  'controlnet',
  'gligen',
  'upscale_models',
  'latent_upscale_models',
  'hypernetworks',
  'photomarker',
  'classifiers',
  'model_patches',
  'audio_encoders',
] as const;

export type ModelType = (typeof MODEL_TYPES)[number];

interface ModelManagerState {
  models: ModelTypeMap;
  loading: boolean;
  error: string | null;
  selectedType: ModelType | null;

  loadModels: () => Promise<void>;
  loadModelsByType: (modelType: ModelType) => Promise<void>;
  deleteModel: (modelType: string, path: string) => Promise<void>;
  uploadModel: (file: File, modelType: string) => Promise<void>;
  setSelectedType: (modelType: ModelType | null) => void;
}

export const useModelManagerStore = create<ModelManagerState>((set, get) => ({
  models: {},
  loading: false,
  error: null,
  selectedType: null,

  loadModels: async () => {
    set({ loading: true, error: null });
    try {
      const result = await api.listModelFiles();
      set({ models: result, loading: false });
    } catch (err) {
      set({ error: String(err), loading: false });
    }
  },

  loadModelsByType: async (modelType: ModelType) => {
    set({ loading: true, error: null });
    try {
      const result = await api.listModelFiles(modelType);
      const { models } = get();
      set({
        models: { ...models, ...result },
        loading: false,
      });
    } catch (err) {
      set({ error: String(err), loading: false });
    }
  },

  deleteModel: async (modelType: string, path: string) => {
    try {
      await api.deleteModelFile({ model_type: modelType, path });
      const { models } = get();
      const typeFiles = models[modelType] || [];
      set({
        models: {
          ...models,
          [modelType]: typeFiles.filter((f: ModelFileInfo) => f.path !== path),
        },
      });
    } catch (err) {
      set({ error: String(err) });
      throw err;
    }
  },

  uploadModel: async (file: File, modelType: string) => {
    try {
      const result = await api.uploadModelFile(file, modelType);
      const { models } = get();
      const typeFiles = models[modelType] || [];
      const newFile: ModelFileInfo = {
        name: result.name,
        path: result.path,
        size: file.size,
        modified: Math.floor(Date.now() / 1000),
      };
      set({
        models: {
          ...models,
          [modelType]: [...typeFiles, newFile],
        },
      });
    } catch (err) {
      set({ error: String(err) });
      throw err;
    }
  },

  setSelectedType: (modelType) => set({ selectedType: modelType }),
}));

export { MODEL_TYPES };
