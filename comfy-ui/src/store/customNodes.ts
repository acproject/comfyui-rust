import { create } from 'zustand';
import type { CustomNodeDef } from '@/types/customNode';
import { customNodeDefToNodeClassDef } from '@/types/customNode';
import type { NodeClassDef, ObjectInfoMap } from '@/types/api';
import { api } from '@/api/client';

interface CustomNodesState {
  customNodes: CustomNodeDef[];
  loaded: boolean;
  editingNode: CustomNodeDef | null;

  loadFromServer: () => Promise<void>;
  addCustomNode: (node: CustomNodeDef) => Promise<void>;
  updateCustomNode: (id: string, updates: Partial<CustomNodeDef>) => Promise<void>;
  removeCustomNode: (id: string) => Promise<void>;
  duplicateCustomNode: (id: string) => Promise<void>;
  importCustomNodes: (nodes: CustomNodeDef[]) => Promise<void>;
  exportCustomNodes: () => string;

  setEditingNode: (node: CustomNodeDef | null) => void;

  getCustomNodeObjectInfo: () => ObjectInfoMap;
  getCustomNodeClassDef: (classType: string) => NodeClassDef | undefined;
  mergeWithObjectInfo: (objectInfo: ObjectInfoMap) => ObjectInfoMap;
}

export const useCustomNodesStore = create<CustomNodesState>((set, get) => ({
  customNodes: [],
  loaded: false,
  editingNode: null,

  loadFromServer: async () => {
    try {
      const result = await api.listCustomNodes();
      const nodes: CustomNodeDef[] = [];
      for (const item of result.nodes) {
        try {
          const def = item.definition as CustomNodeDef;
          if (def && def.classType) {
            nodes.push(def);
          }
        } catch {
          console.warn(`Failed to parse custom node: ${item.filename}`);
        }
      }
      set({ customNodes: nodes, loaded: true });
    } catch (err) {
      console.error('Failed to load custom nodes from server:', err);
      set({ loaded: true });
    }
  },

  addCustomNode: async (node) => {
    await api.saveCustomNode(node.classType, node);
    const { customNodes } = get();
    set({ customNodes: [...customNodes, node] });
  },

  updateCustomNode: async (id, updates) => {
    const { customNodes } = get();
    const existing = customNodes.find((n) => n.id === id);
    if (!existing) return;

    const updated = { ...existing, ...updates, updatedAt: Date.now() };
    await api.saveCustomNode(updated.classType, updated);

    set({
      customNodes: customNodes.map((n) => (n.id === id ? updated : n)),
    });
  },

  removeCustomNode: async (id) => {
    const { customNodes } = get();
    const node = customNodes.find((n) => n.id === id);
    if (!node) return;

    await api.deleteCustomNode(node.classType);
    set({ customNodes: customNodes.filter((n) => n.id !== id) });
  },

  duplicateCustomNode: async (id) => {
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

    await api.saveCustomNode(dup.classType, dup);
    set({ customNodes: [...customNodes, dup] });
  },

  importCustomNodes: async (nodes) => {
    const { customNodes } = get();
    const existingClassTypes = new Set(customNodes.map((n) => n.classType));

    const toAdd: CustomNodeDef[] = [];
    for (const node of nodes) {
      if (existingClassTypes.has(node.classType)) {
        node.classType = `${node.classType}_imported`;
        node.displayName = `${node.displayName} (Imported)`;
      }
      node.id = crypto.randomUUID();
      node.createdAt = Date.now();
      node.updatedAt = Date.now();

      await api.saveCustomNode(node.classType, node);
      toAdd.push(node);
    }

    set({ customNodes: [...customNodes, ...toAdd] });
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
