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
        await customNodesStore.loadFromServer();
        const merged = customNodesStore.mergeWithObjectInfo(objectInfo);

        if (customNodes.size > 0) {
          for (const [classType, def] of customNodes) {
            merged[classType] = def;
          }
        }

        setObjectInfo(merged);
        console.log('[useInitApp] objectInfo loaded, keys count:', Object.keys(merged).length);
        console.log('[useInitApp] objectInfo keys:', Object.keys(merged).join(', '));
      } catch (err) {
        console.error('Failed to initialize app:', err);
      }
    };

    init();
  }, [setObjectInfo]);
}
