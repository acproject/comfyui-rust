import { useEffect, useRef } from 'react';
import { getWsClient } from '@/api/websocket';
import { useWorkflowStore } from '@/store/workflow';
import type { WsMessage, ProgressData, ExecutingData, ExecutionCachedData, StatusData } from '@/types/api';

export function useWebSocket() {
  const wsRef = useRef<ReturnType<typeof getWsClient> | null>(null);

  const setExecuting = useWorkflowStore((s) => s.setExecuting);
  const setCachedNodes = useWorkflowStore((s) => s.setCachedNodes);
  const setProgress = useWorkflowStore((s) => s.setProgress);
  const clearProgress = useWorkflowStore((s) => s.clearProgress);
  const setQueueInfo = useWorkflowStore((s) => s.setQueueInfo);
  const setOutputImages = useWorkflowStore((s) => s.setOutputImages);

  useEffect(() => {
    const ws = getWsClient();
    wsRef.current = ws;

    const unsubs: Array<() => void> = [];

    unsubs.push(
      ws.on('execution_start', (msg: WsMessage) => {
        const data = msg.data as { prompt_id: string };
        setExecuting(data.prompt_id, null);
        clearProgress();
      })
    );

    unsubs.push(
      ws.on('executing', (msg: WsMessage) => {
        const data = msg.data as ExecutingData;
        if (data.node === null) {
          setExecuting(null, null);
          clearProgress();
        } else {
          setExecuting(data.prompt_id, data.node);
        }
      })
    );

    unsubs.push(
      ws.on('progress', (msg: WsMessage) => {
        const data = msg.data as ProgressData;
        setProgress(data.value, data.max);
      })
    );

    unsubs.push(
      ws.on('execution_cached', (msg: WsMessage) => {
        const data = msg.data as ExecutionCachedData;
        console.log('Cached nodes:', data.nodes);
        if (data.nodes && Array.isArray(data.nodes)) {
          setCachedNodes(data.nodes.map(String));
        }
      })
    );

    unsubs.push(
      ws.on('status', (msg: WsMessage) => {
        const data = msg.data as StatusData;
        if (data.status) {
          setQueueInfo(data.status);
        }
      })
    );

    unsubs.push(
      ws.on('execution_success', (msg: WsMessage) => {
        const data = msg.data as { prompt_id: string; output?: Record<string, unknown> };
        console.log('Execution success:', data.prompt_id);
        if (data.output && typeof data.output === 'object') {
          for (const [nodeId, nodeOutput] of Object.entries(data.output)) {
            if (nodeOutput && typeof nodeOutput === 'object' && 'images' in nodeOutput) {
              const images = (nodeOutput as { images: Array<{ filename: string; subfolder: string; type: string }> }).images;
              if (Array.isArray(images)) {
                setOutputImages(nodeId, images);
              }
            }
          }
        }
        setExecuting(null, null);
        clearProgress();
      })
    );

    unsubs.push(
      ws.on('execution_error', (msg: WsMessage) => {
        const data = msg.data as { prompt_id: string; error: string };
        console.error('Execution error:', data.error);
        setExecuting(null, null);
        clearProgress();
      })
    );

    ws.connect();

    return () => {
      unsubs.forEach((unsub) => unsub());
      ws.disconnect();
    };
  }, [setExecuting, setCachedNodes, setProgress, clearProgress, setQueueInfo, setOutputImages]);
}
