import { create } from 'zustand';
import type { CustomNodeDef } from '@/types/customNode';
import { customNodeDefToNodeClassDef } from '@/types/customNode';
import type { NodeClassDef, ObjectInfoMap } from '@/types/api';

const STORAGE_KEY = 'comfyui_custom_nodes';

function loadFromStorage(): CustomNodeDef[] {
  try {
    const raw = localStorage.getItem(STORAGE_KEY);
    if (!raw) return [];
    return JSON.parse(raw) as CustomNodeDef[];
  } catch {
    return [];
  }
}

function saveToStorage(nodes: CustomNodeDef[]): void {
  try {
    localStorage.setItem(STORAGE_KEY, JSON.stringify(nodes));
  } catch (e) {
    console.error('Failed to save custom nodes:', e);
  }
}

interface CustomNodesState {
  customNodes: CustomNodeDef[];
  editingNode: CustomNodeDef | null;

  addCustomNode: (node: CustomNodeDef) => void;
  updateCustomNode: (id: string, updates: Partial<CustomNodeDef>) => void;
  removeCustomNode: (id: string) => void;
  duplicateCustomNode: (id: string) => void;
  importCustomNodes: (nodes: CustomNodeDef[]) => void;
  exportCustomNodes: () => string;

  setEditingNode: (node: CustomNodeDef | null) => void;

  getCustomNodeObjectInfo: () => ObjectInfoMap;
  getCustomNodeClassDef: (classType: string) => NodeClassDef | undefined;
  mergeWithObjectInfo: (objectInfo: ObjectInfoMap) => ObjectInfoMap;
}

export const useCustomNodesStore = create<CustomNodesState>((set, get) => ({
  customNodes: loadFromStorage(),
  editingNode: null,

  addCustomNode: (node) => {
    const { customNodes } = get();
    const updated = [...customNodes, node];
    saveToStorage(updated);
    set({ customNodes: updated });
  },

  updateCustomNode: (id, updates) => {
    const { customNodes } = get();
    const updated = customNodes.map((n) =>
      n.id === id ? { ...n, ...updates, updatedAt: Date.now() } : n
    );
    saveToStorage(updated);
    set({ customNodes: updated });
  },

  removeCustomNode: (id) => {
    const { customNodes } = get();
    const updated = customNodes.filter((n) => n.id !== id);
    saveToStorage(updated);
    set({ customNodes: updated });
  },

  duplicateCustomNode: (id) => {
    const { customNodes } = get();
    const source = customNodes.find((n) => n.id === id);
    if (!source) return;

    const now = Date.now();
    const dup: CustomNodeDef = {
      ...source,
      id: crypto.randomUUID(),
      classType: `${source.classType}_copy`,
      displayName: `${source.displayName} (Copy)`,
      createdAt: now,
      updatedAt: now,
    };
    const updated = [...customNodes, dup];
    saveToStorage(updated);
    set({ customNodes: updated });
  },

  importCustomNodes: (nodes) => {
    const { customNodes } = get();
    const existingIds = new Set(customNodes.map((n) => n.id));
    const existingClassTypes = new Set(customNodes.map((n) => n.classType));

    const toAdd: CustomNodeDef[] = [];
    for (const node of nodes) {
      if (existingIds.has(node.id)) continue;
      if (existingClassTypes.has(node.classType)) {
        node.classType = `${node.classType}_imported`;
        node.displayName = `${node.displayName} (Imported)`;
      }
      node.id = crypto.randomUUID();
      node.createdAt = Date.now();
      node.updatedAt = Date.now();
      toAdd.push(node);
    }

    const updated = [...customNodes, ...toAdd];
    saveToStorage(updated);
    set({ customNodes: updated });
  },

  exportCustomNodes: () => {
    const { customNodes } = get();
    return JSON.stringify(customNodes, null, 2);
  },

  setEditingNode: (node) => set({ editingNode: node }),

  getCustomNodeObjectInfo: () => {
    const { customNodes } = get();
    const result: ObjectInfoMap = {};
    for (const def of customNodes) {
      result[def.classType] = customNodeDefToNodeClassDef(def);
    }
    return result;
  },

  getCustomNodeClassDef: (classType) => {
    const { customNodes } = get();
    const def = customNodes.find((n) => n.classType === classType);
    if (!def) return undefined;
    return customNodeDefToNodeClassDef(def);
  },

  mergeWithObjectInfo: (objectInfo) => {
    const { customNodes } = get();
    const merged = { ...objectInfo };
    for (const def of customNodes) {
      merged[def.classType] = customNodeDefToNodeClassDef(def);
    }
    return merged;
  },
}));
