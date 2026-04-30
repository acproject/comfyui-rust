import { useEffect } from 'react';
import { api } from '@/api/client';
import { useWorkflowStore } from '@/store/workflow';
import { getPluginManager } from '@/plugins/manager';
import { useCustomNodesStore } from '@/store/customNodes';

export function useInitApp() {
  const setObjectInfo = useWorkflowStore((s) => s.setObjectInfo);

  useEffect(() => {
    const init = async () => {
      try {
        const objectInfo = await api.getObjectInfo();

        const pluginManager = getPluginManager();
        const customNodes = pluginManager.getCustomNodes();

        const customNodesStore = useCustomNodesStore.getState();
        const merged = customNodesStore.mergeWithObjectInfo(objectInfo);

        if (customNodes.size > 0) {
          for (const [classType, def] of customNodes) {
            merged[classType] = def;
          }
        }

        setObjectInfo(merged);
      } catch (err) {
        console.error('Failed to initialize app:', err);
      }
    };

    init();
  }, [setObjectInfo]);
}
