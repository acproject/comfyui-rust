import { useCallback, useRef, useState, type FC } from 'react';
import {
  ReactFlow,
  Background,
  Controls,
  MiniMap,
  type Connection,
  type Edge,
  type Node,
  BackgroundVariant,
  useReactFlow,
} from '@xyflow/react';
import '@xyflow/react/dist/style.css';

import { ComfyNodeComponent } from '@/components/nodes/ComfyNode';
import { ContextMenu, type ContextMenuState } from '@/components/editor/ContextMenu';
import { useWorkflowStore, type ComfyNodeData } from '@/store/workflow';

const nodeTypes = {
  comfyNode: ComfyNodeComponent,
};

const GraphEditor: FC = () => {
  const nodes = useWorkflowStore((s) => s.nodes);
  const edges = useWorkflowStore((s) => s.edges);
  const onNodesChange = useWorkflowStore((s) => s.onNodesChange);
  const onEdgesChange = useWorkflowStore((s) => s.onEdgesChange);
  const addNode = useWorkflowStore((s) => s.addNode);
  const removeNode = useWorkflowStore((s) => s.removeNode);
  const setSelectedNodeId = useWorkflowStore((s) => s.setSelectedNodeId);
  const selectedNodeId = useWorkflowStore((s) => s.selectedNodeId);
  const getWorkflowAsJson = useWorkflowStore((s) => s.getWorkflowAsJson);
  const reactFlowWrapper = useRef<HTMLDivElement>(null);
  const { screenToFlowPosition } = useReactFlow();

  const [contextMenu, setContextMenu] = useState<ContextMenuState | null>(null);

  const onConnect = useCallback(
    (connection: Connection) => {
      if (!connection.source || !connection.target) return;
      const sourceHandle = connection.sourceHandle || '0';
      const targetHandle = connection.targetHandle || '0';

      const newEdge: Edge = {
        id: `e-${connection.source}-${sourceHandle}-${connection.target}-${targetHandle}`,
        source: connection.source,
        sourceHandle,
        target: connection.target,
        targetHandle,
      };

      const store = useWorkflowStore.getState();
      const filtered = store.edges.filter(
        (e) => !(e.target === connection.target && e.targetHandle === targetHandle)
      );
      useWorkflowStore.setState({ edges: [...filtered, newEdge] });
    },
    []
  );

  const onDragOver = useCallback((event: React.DragEvent) => {
    event.preventDefault();
    event.dataTransfer.dropEffect = 'move';
  }, []);

  const onDrop = useCallback(
    (event: React.DragEvent) => {
      event.preventDefault();

      const classType = event.dataTransfer.getData('application/comfy-node');
      if (!classType || !reactFlowWrapper.current) return;

      const position = screenToFlowPosition({
        x: event.clientX,
        y: event.clientY,
      });

      addNode(classType, position);
    },
    [addNode, screenToFlowPosition]
  );

  const onNodeClick = useCallback(
    (_event: React.MouseEvent, node: Node) => {
      setSelectedNodeId(node.id);
    },
    [setSelectedNodeId]
  );

  const onPaneClick = useCallback(() => {
    setSelectedNodeId(null);
    setContextMenu(null);
  }, [setSelectedNodeId]);

  const onNodeContextMenu = useCallback(
    (event: React.MouseEvent, node: Node) => {
      event.preventDefault();
      setContextMenu({
        x: event.clientX,
        y: event.clientY,
        type: 'node',
        nodeId: node.id,
      });
    },
    []
  );

  const onPaneContextMenu = useCallback(
    (event: React.MouseEvent) => {
      event.preventDefault();
      setContextMenu({
        x: event.clientX,
        y: event.clientY,
        type: 'canvas',
      });
      const store = useWorkflowStore.getState();
      store.setNodes(store.nodes);
    },
    []
  );

  const onKeyDown = useCallback(
    (event: React.KeyboardEvent) => {
      if (event.key === 'Delete' || event.key === 'Backspace') {
        if (selectedNodeId && !(event.target instanceof HTMLInputElement) && !(event.target instanceof HTMLTextAreaElement) && !(event.target instanceof HTMLSelectElement)) {
          removeNode(selectedNodeId);
          setSelectedNodeId(null);
        }
      }
      if ((event.metaKey || event.ctrlKey) && event.key === 's') {
        event.preventDefault();
        const json = getWorkflowAsJson();
        const blob = new Blob([JSON.stringify(json, null, 2)], { type: 'application/json' });
        const url = URL.createObjectURL(blob);
        const a = document.createElement('a');
        a.href = url;
        a.download = 'workflow.json';
        a.click();
        URL.revokeObjectURL(url);
      }
    },
    [selectedNodeId, removeNode, setSelectedNodeId, getWorkflowAsJson]
  );

  return (
    <div
      ref={reactFlowWrapper}
      style={{ flex: 1, height: '100%' }}
      onKeyDown={onKeyDown}
      tabIndex={0}
    >
      <ReactFlow
        nodes={nodes}
        edges={edges}
        onNodesChange={onNodesChange as never}
        onEdgesChange={onEdgesChange as never}
        onConnect={onConnect}
        onDragOver={onDragOver}
        onDrop={onDrop}
        onNodeClick={onNodeClick}
        onPaneClick={onPaneClick}
        onNodeContextMenu={onNodeContextMenu}
        onPaneContextMenu={onPaneContextMenu}
        nodeTypes={nodeTypes}
        fitView
        fitViewOptions={{ padding: 0.2 }}
        style={{ background: '#0f1117' }}
        defaultEdgeOptions={{
          type: 'smoothstep',
          style: { stroke: '#555', strokeWidth: 2 },
          animated: false,
        }}
        nodesDraggable
        nodesConnectable
        elementsSelectable
        zoomOnScroll
        panOnScroll
        panOnDrag
        minZoom={0.1}
        maxZoom={4}
        selectionOnDrag
        selectNodesOnDrag
        deleteKeyCode={null}
      >
        <Background variant={BackgroundVariant.Dots} gap={20} size={1} color="#2d3748" />
        <Controls
          style={{ background: '#1e1e2e', borderRadius: 6, border: '1px solid #333' }}
          position="bottom-right"
        />
        <MiniMap
          style={{ background: '#1e1e2e', border: '1px solid #333' }}
          nodeColor={(node) => {
            const data = node.data as ComfyNodeData;
            const cat = data?.category || '';
            if (cat.includes('loaders')) return '#5b8c5a';
            if (cat.includes('conditioning')) return '#c78030';
            if (cat.includes('sampling')) return '#5a6abf';
            if (cat.includes('latent')) return '#7a5bbf';
            if (cat.includes('image')) return '#bf5b7a';
            return '#4a5568';
          }}
          maskColor="rgba(0,0,0,0.7)"
        />
      </ReactFlow>

      {contextMenu && (
        <ContextMenu
          menu={contextMenu}
          onClose={() => setContextMenu(null)}
        />
      )}
    </div>
  );
};

export { GraphEditor };
