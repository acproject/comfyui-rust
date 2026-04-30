import type { NodeClassDef } from '@/types/api';
import type { CustomNodeDef } from '@/types/customNode';
import { customNodeDefToNodeClassDef } from '@/types/customNode';

export interface ComfyPlugin {
  name: string;
  version: string;
  description?: string;
  registerNodes?: (registry: NodeRegistry) => void;
  onInit?: () => void;
  onDestroy?: () => void;
}

export interface NodeRegistry {
  registerNode: (classType: string, definition: NodeClassDef) => void;
}

class PluginManager {
  private plugins: Map<string, ComfyPlugin> = new Map();
  private customNodes: Map<string, NodeClassDef> = new Map();

  registerPlugin(plugin: ComfyPlugin): void {
    if (this.plugins.has(plugin.name)) {
      console.warn(`Plugin "${plugin.name}" is already registered. Skipping.`);
      return;
    }

    this.plugins.set(plugin.name, plugin);

    if (plugin.registerNodes) {
      const registry: NodeRegistry = {
        registerNode: (classType, definition) => {
          this.customNodes.set(classType, definition);
        },
      };
      plugin.registerNodes(registry);
    }

    plugin.onInit?.();
    console.log(`Plugin registered: ${plugin.name} v${plugin.version}`);
  }

  unregisterPlugin(name: string): void {
    const plugin = this.plugins.get(name);
    if (plugin) {
      plugin.onDestroy?.();
      this.plugins.delete(name);
    }
  }

  registerCustomNodeDef(def: CustomNodeDef): void {
    this.customNodes.set(def.classType, customNodeDefToNodeClassDef(def));
  }

  unregisterCustomNodeDef(classType: string): void {
    this.customNodes.delete(classType);
  }

  syncCustomNodeDefs(defs: CustomNodeDef[]): void {
    const existingPluginNodes = new Set<string>();
    for (const [key] of this.customNodes) {
      if (!key.startsWith('Custom_')) {
        existingPluginNodes.add(key);
      }
    }
    this.customNodes.clear();
    for (const key of existingPluginNodes) {
      const def = this.customNodes.get(key);
      if (def) this.customNodes.set(key, def);
    }
    for (const def of defs) {
      this.customNodes.set(def.classType, customNodeDefToNodeClassDef(def));
    }
  }

  getCustomNodes(): Map<string, NodeClassDef> {
    return new Map(this.customNodes);
  }

  getPlugins(): ComfyPlugin[] {
    return Array.from(this.plugins.values());
  }

  getPlugin(name: string): ComfyPlugin | undefined {
    return this.plugins.get(name);
  }
}

let _instance: PluginManager | null = null;

export function getPluginManager(): PluginManager {
  if (!_instance) {
    _instance = new PluginManager();
  }
  return _instance;
}
