import { create } from 'zustand';
import type { ObjectInfoMap, QueueInfo } from '@/types/api';
import { applyNodeChanges, applyEdgeChanges, type Node, type Edge, type NodeChange, type EdgeChange } from '@xyflow/react';
import {
  validateConnection,
  validatePrompt,
  getExecutionOrder,
  detectCycleInGraph,
  type ValidationError,
  type ValidationResult,
  type ExecutionOrderResult,
} from '@/dag';

interface ComfyNodeData extends Record<string, unknown> {
  classType: string;
  title: string;
  inputs: Record<string, unknown>;
  outputs: { name: string; type: string }[];
  isOutputNode: boolean;
  category: string;
}

interface OutputImage {
  filename: string;
  subfolder: string;
  type: string;
}

type ComfyNode = Node<ComfyNodeData>;

interface WorkflowState {
  nodes: ComfyNode[];
  edges: Edge[];

  selectedNodeId: string | null;

  objectInfo: ObjectInfoMap;
  objectInfoLoaded: boolean;

  queueInfo: QueueInfo | null;

  executingPromptId: string | null;
  executingNodeId: string | null;
  progress: { value: number; max: number } | null;

  clientId: string;

  outputImages: Record<string, OutputImage[]>;

  validationErrors: ValidationError[];

  setNodes: (nodes: ComfyNode[]) => void;
  setEdges: (edges: Edge[]) => void;
  onNodesChange: (changes: NodeChange<ComfyNode>[]) => void;
  onEdgesChange: (changes: EdgeChange<Edge>[]) => void;
  addNode: (classType: string, position: { x: number; y: number }) => void;
  removeNode: (nodeId: string) => void;
  updateNodeInput: (nodeId: string, inputName: string, value: unknown) => void;
  connectNodes: (source: string, sourceHandle: string, target: string, targetHandle: string) => ValidationError | null;
  disconnectNode: (edgeId: string) => void;

  setSelectedNodeId: (nodeId: string | null) => void;

  setObjectInfo: (info: ObjectInfoMap) => void;
  setQueueInfo: (info: QueueInfo) => void;

  setExecuting: (promptId: string | null, nodeId: string | null) => void;
  setProgress: (value: number, max: number) => void;
  clearProgress: () => void;
  setOutputImages: (nodeId: string, images: OutputImage[]) => void;

  getPrompt: () => Record<string, unknown>;
  clearWorkflow: () => void;
  loadWorkflowFromJson: (workflow: Record<string, unknown>) => void;
  getWorkflowAsJson: () => Record<string, unknown>;

  validateWorkflow: () => ValidationResult;
  getExecutionOrderForWorkflow: () => ExecutionOrderResult;
  validateConnection: (source: string, sourceHandle: string, target: string, targetHandle: string) => ValidationError | null;
  setValidationErrors: (errors: ValidationError[]) => void;
  clearValidationErrors: () => void;
}

let _nodeIdCounter = 0;
function nextNodeId(): string {
  return String(++_nodeIdCounter);
}

export const useWorkflowStore = create<WorkflowState>((set, get) => ({
  nodes: [],
  edges: [],
  selectedNodeId: null,
  objectInfo: {},
  objectInfoLoaded: false,
  queueInfo: null,
  executingPromptId: null,
  executingNodeId: null,
  progress: null,
  clientId: crypto.randomUUID(),
  outputImages: {},
  validationErrors: [],

  setNodes: (nodes) => set({ nodes }),
  setEdges: (edges) => set({ edges }),

  onNodesChange: (changes) => {
    const { nodes } = get();
    const updatedNodes = applyNodeChanges(changes, nodes);
    set({ nodes: updatedNodes });
  },

  onEdgesChange: (changes) => {
    const { edges } = get();
    const updatedEdges = applyEdgeChanges(changes, edges);
    set({ edges: updatedEdges });
  },

  addNode: (classType, position) => {
    const { objectInfo, nodes } = get();
    const classDef = objectInfo[classType];
    if (!classDef) return;

    const nodeId = nextNodeId();
    const newNode: ComfyNode = {
      id: nodeId,
      type: 'comfyNode',
      position,
      data: {
        classType,
        title: classDef.display_name || classType,
        inputs: {},
        outputs: classDef.output_names.map((name, i) => ({
          name,
          type: classDef.output_types[i] || '*',
        })),
        isOutputNode: classDef.is_output_node,
        category: classDef.category,
      },
    };

    if (classDef.input_types.required) {
      for (const [key, spec] of Object.entries(classDef.input_types.required)) {
        if (spec.type_name === 'INT') {
          newNode.data.inputs[key] = 0;
        } else if (spec.type_name === 'FLOAT') {
          newNode.data.inputs[key] = 0.0;
        } else if (spec.type_name === 'STRING') {
          newNode.data.inputs[key] = '';
        } else if (spec.type_name === 'BOOLEAN') {
          newNode.data.inputs[key] = false;
        } else if (spec.type_name === 'COMBO') {
          const choices = (spec.extra?.choices as string[]) || [];
          newNode.data.inputs[key] = choices.length > 0 ? choices[0] : '';
        }
      }
    }

    if (classDef.input_types.optional) {
      for (const [key, spec] of Object.entries(classDef.input_types.optional)) {
        if (spec.type_name === 'INT') {
          newNode.data.inputs[key] = 0;
        } else if (spec.type_name === 'FLOAT') {
          newNode.data.inputs[key] = 0.0;
        } else if (spec.type_name === 'STRING') {
          newNode.data.inputs[key] = '';
        } else if (spec.type_name === 'BOOLEAN') {
          newNode.data.inputs[key] = false;
        } else if (spec.type_name === 'COMBO') {
          const choices = (spec.extra?.choices as string[]) || [];
          newNode.data.inputs[key] = choices.length > 0 ? choices[0] : '';
        }
      }
    }

    set({ nodes: [...nodes, newNode] });
  },

  removeNode: (nodeId) => {
    const { nodes, edges } = get();
    set({
      nodes: nodes.filter((n) => n.id !== nodeId),
      edges: edges.filter((e) => e.source !== nodeId && e.target !== nodeId),
    });
  },

  updateNodeInput: (nodeId, inputName, value) => {
    const { nodes } = get();
    set({
      nodes: nodes.map((n) =>
        n.id === nodeId
          ? { ...n, data: { ...n.data, inputs: { ...n.data.inputs, [inputName]: value } } }
          : n
      ),
    });
  },

  connectNodes: (source, sourceHandle, target, targetHandle) => {
    const { objectInfo, nodes, edges } = get();

    const sourceNode = nodes.find((n) => n.id === source);
    if (!sourceNode) return { type: 'node_not_found', message: 'Source node not found', details: source };

    const sourceDef = objectInfo[sourceNode.data.classType];
    if (!sourceDef) return { type: 'missing_node_type', message: 'Source node type not found', details: sourceNode.data.classType };

    const sourceOutputIndex = sourceDef.output_names.indexOf(sourceHandle);
    if (sourceOutputIndex < 0) return { type: 'invalid_output', message: 'Invalid output handle', details: sourceHandle };

    const connError = validateConnection(
      sourceNode.data.classType,
      sourceOutputIndex,
      nodes.find((n) => n.id === target)?.data.classType || '',
      targetHandle,
      objectInfo
    );
    if (connError) return connError;

    const tempEdges = [
      ...edges.filter((e) => !(e.target === target && e.targetHandle === targetHandle)),
      { id: `e-${source}-${sourceHandle}-${target}-${targetHandle}`, source, sourceHandle, target, targetHandle },
    ];

    const tempNodes: Record<string, { inputs: Record<string, unknown> }> = {};
    for (const node of nodes) {
      const inputs: Record<string, unknown> = {};
      for (const [key, value] of Object.entries(node.data.inputs)) {
        inputs[key] = value;
      }
      for (const edge of tempEdges) {
        if (edge.target === node.id && edge.targetHandle) {
          const sn = nodes.find((n) => n.id === edge.source);
          if (sn) {
            const sd = objectInfo[sn.data.classType];
            if (sd) {
              const oi = sd.output_names.indexOf(edge.sourceHandle || '');
              if (oi >= 0) inputs[edge.targetHandle] = [edge.source, oi];
            }
          }
        }
      }
      tempNodes[node.id] = { inputs };
    }

    const cycle = detectCycleInGraph(tempNodes);
    if (cycle) {
      return {
        type: 'dependency_cycle',
        message: 'Connection would create a dependency cycle',
        details: cycle.join(' -> '),
        extraInfo: { cycleNodes: cycle },
      };
    }

    const edgeId = `e-${source}-${sourceHandle}-${target}-${targetHandle}`;
    const newEdge: Edge = {
      id: edgeId,
      source,
      sourceHandle,
      target,
      targetHandle,
    };
    const filtered = edges.filter(
      (e) => !(e.target === target && e.targetHandle === targetHandle)
    );
    set({ edges: [...filtered, newEdge], validationErrors: [] });
    return null;
  },

  disconnectNode: (edgeId) => {
    const { edges } = get();
    set({ edges: edges.filter((e) => e.id !== edgeId) });
  },

  setSelectedNodeId: (nodeId) => set({ selectedNodeId: nodeId }),

  setObjectInfo: (info) => set({ objectInfo: info, objectInfoLoaded: true }),
  setQueueInfo: (info) => set({ queueInfo: info }),

  setExecuting: (promptId, nodeId) =>
    set({ executingPromptId: promptId, executingNodeId: nodeId }),
  setProgress: (value, max) => set({ progress: { value, max } }),
  clearProgress: () => set({ progress: null }),

  setOutputImages: (nodeId, images) => {
    const { outputImages } = get();
    set({ outputImages: { ...outputImages, [nodeId]: images } });
  },

  getPrompt: () => {
    const { nodes, edges } = get();
    const prompt: Record<string, unknown> = {};

    for (const node of nodes) {
      const inputs: Record<string, unknown> = {};
      for (const [key, value] of Object.entries(node.data.inputs)) {
        inputs[key] = value;
      }

      for (const edge of edges) {
        if (edge.target === node.id && edge.targetHandle) {
          const inputName = edge.targetHandle;
          const sourceNode = nodes.find((n) => n.id === edge.source);
          if (sourceNode) {
            const outputIndex = sourceNode.data.outputs.findIndex(
              (o) => o.name === edge.sourceHandle
            );
            if (outputIndex >= 0) {
              inputs[inputName] = [edge.source, outputIndex];
            }
          }
        }
      }

      prompt[node.id] = {
        class_type: node.data.classType,
        inputs,
      };
    }

    return prompt;
  },

  clearWorkflow: () => {
    _nodeIdCounter = 0;
    set({ nodes: [], edges: [], selectedNodeId: null, executingPromptId: null, executingNodeId: null, progress: null });
  },

  loadWorkflowFromJson: (workflow) => {
    const { objectInfo } = get();
    const workflowNodes = (workflow.nodes as Array<Record<string, unknown>>) || [];
    const workflowLinks = workflow.links;

    console.log('[loadWorkflowFromJson] Starting...');
    console.log('[loadWorkflowFromJson] objectInfo keys count:', Object.keys(objectInfo).length);
    console.log('[loadWorkflowFromJson] workflowNodes count:', workflowNodes.length);
    console.log('[loadWorkflowFromJson] workflowLinks:', workflowLinks ? (Array.isArray(workflowLinks) ? `${workflowLinks.length} links` : 'not array') : 'undefined');

    _nodeIdCounter = 0;
    const nodeIdMap: Record<string, string> = {};
    const newNodes: ComfyNode[] = [];
    const newEdges: Edge[] = [];

    const nodeOutputsMap: Record<string, Array<{ name: string; type: string }>> = {};

    const isPrimitive = (typeName: string) =>
      ['INT', 'FLOAT', 'STRING', 'BOOLEAN', 'COMBO'].includes(typeName);

    for (const wn of workflowNodes) {
      const classType = (wn.type as string) || '';
      const classDef = objectInfo[classType];
      const oldId = String(wn.id);
      const newId = nextNodeId();
      nodeIdMap[oldId] = newId;

      if (!classDef) {
        console.warn(`[loadWorkflowFromJson] No classDef for classType "${classType}" (node id=${oldId}). Available types:`, Object.keys(objectInfo).slice(0, 10));
      }

      const rawPos = wn.pos;
      let posX = 0, posY = 0;
      if (Array.isArray(rawPos)) {
        posX = rawPos[0] || 0;
        posY = rawPos[1] || 0;
      } else if (rawPos && typeof rawPos === 'object') {
        posX = (rawPos as Record<string, number>)['0'] || 0;
        posY = (rawPos as Record<string, number>)['1'] || 0;
      }

      const widgetsValues = Array.isArray(wn.widgets_values)
        ? (wn.widgets_values as unknown[])
        : [];

      const wnOutputs = (wn.outputs as Array<{ name: string; type: string | string[] }>) || [];

      const outputs = classDef?.output_names.map((name, i) => ({
        name,
        type: classDef.output_types[i] || '*',
      })) || wnOutputs.map((o) => ({
        name: o.name,
        type: Array.isArray(o.type) ? o.type[0] : (o.type || '*'),
      }));

      nodeOutputsMap[oldId] = outputs;

      const allInputNames: Array<{ name: string; typeName: string }> = [];
      if (classDef?.input_types.required) {
        for (const [key, spec] of Object.entries(classDef.input_types.required)) {
          allInputNames.push({ name: key, typeName: spec.type_name });
        }
      }
      if (classDef?.input_types.optional) {
        for (const [key, spec] of Object.entries(classDef.input_types.optional)) {
          allInputNames.push({ name: key, typeName: spec.type_name });
        }
      }

      const wnInputs = (wn.inputs as Array<{ name: string; type: string | string[]; link?: number | null }>) || [];
      const wnInputMap: Record<string, { type: string; link: number | null }> = {};
      for (const inp of wnInputs) {
        wnInputMap[inp.name] = {
          type: Array.isArray(inp.type) ? inp.type[0] : (inp.type || '*'),
          link: inp.link ?? null,
        };
      }

      const inputs: Record<string, unknown> = {};
      let widgetIdx = 0;

      for (const { name, typeName } of allInputNames) {
        const primitive = isPrimitive(typeName);
        const wnInput = wnInputMap[name];
        const isConnected = wnInput && wnInput.link !== null && wnInput.link !== undefined;

        if (!primitive) {
          if (isConnected) {
            inputs[name] = null;
          }
        } else {
          if (widgetIdx < widgetsValues.length) {
            const wv = widgetsValues[widgetIdx];
            inputs[name] = wv;
          } else if (typeName === 'INT') {
            inputs[name] = 0;
          } else if (typeName === 'FLOAT') {
            inputs[name] = 0.0;
          } else if (typeName === 'STRING') {
            inputs[name] = '';
          } else if (typeName === 'BOOLEAN') {
            inputs[name] = false;
          } else if (typeName === 'COMBO') {
            const spec = classDef?.input_types.required?.[name] || classDef?.input_types.optional?.[name];
            const choices = (spec?.extra?.choices as string[]) || [];
            inputs[name] = choices.length > 0 ? choices[0] : '';
          }
          widgetIdx++;
        }
      }

      newNodes.push({
        id: newId,
        type: 'comfyNode',
        position: { x: posX, y: posY },
        data: {
          classType,
          title: (wn.title as string) || classDef?.display_name || classType,
          inputs,
          outputs,
          isOutputNode: classDef?.is_output_node || false,
          category: classDef?.category || '',
        },
      });

      if (Number(wn.id) > _nodeIdCounter) {
        _nodeIdCounter = Number(wn.id);
      }
    }

    if (Array.isArray(workflowLinks)) {
      for (const link of workflowLinks) {
        let originId: number | string, originSlot: number, targetId: number | string, targetSlot: number;

        if (Array.isArray(link) && link.length >= 5) {
          [, originId, originSlot, targetId, targetSlot] = link as [number, number, number, number, number, string];
        } else if (link && typeof link === 'object' && !Array.isArray(link)) {
          const obj = link as Record<string, unknown>;
          originId = obj.origin_id as number;
          originSlot = obj.origin_slot as number;
          targetId = obj.target_id as number;
          targetSlot = obj.target_slot as number;
        } else {
          continue;
        }

        const newSourceId = nodeIdMap[String(originId)];
        const newTargetId = nodeIdMap[String(targetId)];
        if (!newSourceId || !newTargetId) continue;

        const sourceNode = newNodes.find((n) => n.id === newSourceId);
        const targetNode = newNodes.find((n) => n.id === newTargetId);
        if (!sourceNode || !targetNode) continue;

        const sourceHandle = sourceNode.data.outputs[originSlot]?.name || String(originSlot);

        const targetClassDef = objectInfo[targetNode.data.classType];
        let targetHandle: string;
        if (targetClassDef) {
          const allInputNames: string[] = [];
          if (targetClassDef.input_types.required) {
            allInputNames.push(...Object.keys(targetClassDef.input_types.required));
          }
          if (targetClassDef.input_types.optional) {
            allInputNames.push(...Object.keys(targetClassDef.input_types.optional));
          }
          targetHandle = allInputNames[targetSlot] || String(targetSlot);
        } else {
          const inputKeys = Object.keys(targetNode.data.inputs);
          targetHandle = inputKeys[targetSlot] || String(targetSlot);
        }

        newEdges.push({
          id: `e-${newSourceId}-${sourceHandle}-${newTargetId}-${targetHandle}`,
          source: newSourceId,
          sourceHandle,
          target: newTargetId,
          targetHandle,
        });

        if (targetNode.data.inputs.hasOwnProperty(targetHandle)) {
          const sourceOutputIdx = sourceNode.data.outputs.findIndex(
            (o) => o.name === sourceHandle
          );
          if (sourceOutputIdx >= 0) {
            targetNode.data.inputs[targetHandle] = [newSourceId, sourceOutputIdx];
          }
        }
      }
    }

    console.log('[loadWorkflowFromJson] Created', newNodes.length, 'nodes and', newEdges.length, 'edges');
    console.log('[loadWorkflowFromJson] nodeIdMap:', nodeIdMap);
    if (newNodes.length > 0) {
      console.log('[loadWorkflowFromJson] First node:', JSON.stringify(newNodes[0].data).substring(0, 200));
    }
    if (newEdges.length > 0) {
      console.log('[loadWorkflowFromJson] First edge:', newEdges[0]);
    }

    set({ nodes: newNodes, edges: newEdges, selectedNodeId: null });
  },

  getWorkflowAsJson: () => {
    const { nodes, edges, objectInfo } = get();
    const links: unknown[] = [];
    let linkId = 0;

    const edgeLinkMap: Record<string, number> = {};

    for (const edge of edges) {
      linkId++;
      const sourceNode = nodes.find((n) => n.id === edge.source);
      const targetNode = nodes.find((n) => n.id === edge.target);
      if (!sourceNode || !targetNode) continue;

      const originSlot = sourceNode.data.outputs.findIndex(
        (o) => o.name === edge.sourceHandle
      );
      if (originSlot < 0) continue;

      const sourceOutputType = sourceNode.data.outputs[originSlot]?.type || '*';

      const targetClassDef = objectInfo[targetNode.data.classType];
      let targetSlot = -1;
      if (targetClassDef) {
        const allInputNames: string[] = [];
        if (targetClassDef.input_types.required) {
          allInputNames.push(...Object.keys(targetClassDef.input_types.required));
        }
        if (targetClassDef.input_types.optional) {
          allInputNames.push(...Object.keys(targetClassDef.input_types.optional));
        }
        targetSlot = allInputNames.indexOf(edge.targetHandle || '');
      }
      if (targetSlot < 0) {
        targetSlot = Object.keys(targetNode.data.inputs).indexOf(edge.targetHandle || '');
      }

      links.push([
        linkId,
        Number(edge.source),
        originSlot,
        Number(edge.target),
        Math.max(0, targetSlot),
        sourceOutputType,
      ]);

      edgeLinkMap[`${edge.source}-${originSlot}`] = linkId;
    }

    const workflowNodes = nodes.map((n) => {
      const classDef = objectInfo[n.data.classType];

      const allInputNames: string[] = [];
      const allInputTypes: string[] = [];
      if (classDef?.input_types.required) {
        for (const [key, spec] of Object.entries(classDef.input_types.required)) {
          allInputNames.push(key);
          allInputTypes.push(spec.type_name);
        }
      }
      if (classDef?.input_types.optional) {
        for (const [key, spec] of Object.entries(classDef.input_types.optional)) {
          allInputNames.push(key);
          allInputTypes.push(spec.type_name);
        }
      }

      const isPrimitive = (typeName: string) =>
        ['INT', 'FLOAT', 'STRING', 'BOOLEAN', 'COMBO'].includes(typeName);

      const nodeInputs: Array<{ name: string; type: string; link: number | null }> = [];
      const widgetsValues: unknown[] = [];
      let widgetCount = 0;

      for (let i = 0; i < allInputNames.length; i++) {
        const inputName = allInputNames[i];
        const inputType = allInputTypes[i];
        const value = n.data.inputs[inputName];
        const primitive = isPrimitive(inputType);

        const connectedEdge = edges.find(
          (e) => e.target === n.id && e.targetHandle === inputName
        );

        if (connectedEdge) {
          const sourceNode = nodes.find((sn) => sn.id === connectedEdge.source);
          const originSlot = sourceNode
            ? sourceNode.data.outputs.findIndex((o) => o.name === connectedEdge.sourceHandle)
            : -1;
          const linkIdx = originSlot >= 0
            ? edgeLinkMap[`${connectedEdge.source}-${originSlot}`]
            : undefined;

          if (!primitive) {
            nodeInputs.push({
              name: inputName,
              type: inputType,
              link: linkIdx ?? null,
            });
          } else {
            if (value !== undefined && !Array.isArray(value)) {
              widgetsValues.push(value);
            } else {
              widgetsValues.push(null);
            }
            widgetCount++;
          }
        } else {
          if (!primitive) {
            nodeInputs.push({
              name: inputName,
              type: inputType,
              link: null,
            });
          } else {
            if (value !== undefined && !Array.isArray(value)) {
              widgetsValues.push(value);
            } else {
              widgetsValues.push(null);
            }
            widgetCount++;
          }
        }
      }

      const nodeOutputs = n.data.outputs.map((o, i) => {
        const connectedLinks = links
          .filter((l) => {
            const linkArr = l as unknown[];
            return linkArr[1] === Number(n.id) && linkArr[2] === i;
          })
          .map((l) => (l as unknown[])[0] as number);

        return {
          name: o.name,
          type: o.type,
          links: connectedLinks.length > 0 ? connectedLinks : null,
          slot_index: i,
        };
      });

      return {
        id: Number(n.id),
        type: n.data.classType,
        pos: [n.position.x, n.position.y],
        size: { '0': 220, '1': 100 },
        flags: { collapsed: false },
        order: 0,
        mode: 0,
        inputs: nodeInputs,
        outputs: nodeOutputs,
        properties: {},
        widgets_values: widgetsValues,
        title: n.data.title !== n.data.classType ? n.data.title : undefined,
      };
    });

    return {
      last_node_id: Math.max(0, ...nodes.map((n) => Number(n.id))),
      last_link_id: linkId,
      nodes: workflowNodes,
      links,
      groups: [],
      config: {},
      extra: {},
      version: 0.4,
    };
  },

  validateWorkflow: () => {
    const { nodes, edges, objectInfo } = get();
    const graphNodes = nodes.map((n) => ({
      id: n.id,
      classType: n.data.classType,
      inputs: n.data.inputs,
    }));
    const graphEdges = edges.map((e) => ({
      source: e.source,
      sourceHandle: e.sourceHandle || '',
      target: e.target,
      targetHandle: e.targetHandle || '',
    }));
    const result = validatePrompt(graphNodes, graphEdges, objectInfo);
    set({ validationErrors: result.errors });
    return result;
  },

  getExecutionOrderForWorkflow: () => {
    const { nodes, edges, objectInfo } = get();
    const graphNodes = nodes.map((n) => ({
      id: n.id,
      classType: n.data.classType,
      inputs: n.data.inputs,
    }));
    const graphEdges = edges.map((e) => ({
      source: e.source,
      sourceHandle: e.sourceHandle || '',
      target: e.target,
      targetHandle: e.targetHandle || '',
    }));
    return getExecutionOrder(graphNodes, graphEdges, objectInfo);
  },

  validateConnection: (source, sourceHandle, target, targetHandle) => {
    const { objectInfo, nodes } = get();
    const sourceNode = nodes.find((n) => n.id === source);
    if (!sourceNode) return { type: 'node_not_found', message: 'Source node not found', details: source };

    const sourceDef = objectInfo[sourceNode.data.classType];
    console.log('[validateConnection]', {
      source,
      sourceHandle,
      target,
      targetHandle,
      sourceClassType: sourceNode.data.classType,
      targetClassType: nodes.find((n) => n.id === target)?.data.classType,
      sourceOutputNames: sourceDef?.output_names,
    });
    if (!sourceDef) return { type: 'missing_node_type', message: 'Source node type not found', details: sourceNode.data.classType };

    const sourceOutputIndex = sourceDef.output_names.indexOf(sourceHandle);
    if (sourceOutputIndex < 0) return { type: 'invalid_output', message: 'Invalid output handle', details: `${sourceHandle} not in [${sourceDef.output_names.join(', ')}]` };

    const targetNode = nodes.find((n) => n.id === target);
    if (!targetNode) return { type: 'node_not_found', message: 'Target node not found', details: target };

    return validateConnection(
      sourceNode.data.classType,
      sourceOutputIndex,
      targetNode.data.classType,
      targetHandle,
      objectInfo
    );
  },

  setValidationErrors: (errors) => set({ validationErrors: errors }),
  clearValidationErrors: () => set({ validationErrors: [] }),
}));

export type { ComfyNodeData, ComfyNode };
