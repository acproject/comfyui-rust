import type { ObjectInfoMap, IoType } from '@/types/api';

export interface ValidationError {
  type: string;
  message: string;
  details: string;
  nodeId?: string;
  inputName?: string;
  extraInfo?: Record<string, unknown>;
}

export interface ValidationResult {
  valid: boolean;
  errors: ValidationError[];
}

export interface ExecutionOrderResult {
  order: string[];
  errors: ValidationError[];
}

const ANY_TYPE = '*';
const MATCH_TYPE = 'MATCH';

function normalizeType(t: string): string {
  const upper = t.trim().toUpperCase();
  if (upper === 'BOOLEAN') return 'BOOLEAN';
  return upper;
}

function splitTypes(typeStr: string): Set<string> {
  return new Set(typeStr.split(',').map((t) => normalizeType(t)).filter(Boolean));
}

export function validateNodeInput(
  receivedType: IoType,
  inputType: IoType,
  strict: boolean = false
): boolean {
  if (typeof receivedType === 'string' && typeof inputType === 'string') {
    if (normalizeType(receivedType) === normalizeType(inputType)) return true;
  } else if (receivedType === inputType) {
    return true;
  }

  if (receivedType === ANY_TYPE || inputType === ANY_TYPE) return true;

  if (receivedType === MATCH_TYPE || inputType === MATCH_TYPE) return true;

  if (typeof receivedType !== 'string' || typeof inputType !== 'string') return false;

  const receivedTypes = splitTypes(receivedType);
  const inputTypes = splitTypes(inputType);

  if (receivedTypes.has(ANY_TYPE) || inputTypes.has(ANY_TYPE)) return true;

  if (strict) {
    for (const rt of receivedTypes) {
      if (!inputTypes.has(rt)) return false;
    }
    return true;
  }

  for (const rt of receivedTypes) {
    if (inputTypes.has(rt)) return true;
  }
  return false;
}

export function detectCycle(
  nodes: Record<string, { inputs: Record<string, unknown> }>,
  startNodeId: string
): string[] | null {
  const visited = new Set<string>();
  const recursionStack = new Set<string>();
  const path: string[] = [];

  function dfs(nodeId: string): string[] | null {
    visited.add(nodeId);
    recursionStack.add(nodeId);
    path.push(nodeId);

    const node = nodes[nodeId];
    if (!node) return null;

    for (const inputVal of Object.values(node.inputs)) {
      if (isLinkValue(inputVal)) {
        const sourceId = inputVal[0];
        if (!visited.has(sourceId)) {
          const cycle = dfs(sourceId);
          if (cycle) return cycle;
        } else if (recursionStack.has(sourceId)) {
          const cycleStart = path.indexOf(sourceId);
          return path.slice(cycleStart);
        }
      }
    }

    path.pop();
    recursionStack.delete(nodeId);
    return null;
  }

  return dfs(startNodeId);
}

export function detectCycleInGraph(
  nodes: Record<string, { inputs: Record<string, unknown> }>
): string[] | null {
  const visited = new Set<string>();

  for (const nodeId of Object.keys(nodes)) {
    if (!visited.has(nodeId)) {
      const cycle = detectCycle(nodes, nodeId);
      if (cycle) return cycle;
      visited.add(nodeId);
    }
  }
  return null;
}

export function topologicalSort(
  nodes: Record<string, { inputs: Record<string, unknown> }>
): ExecutionOrderResult {
  const inDegree: Record<string, number> = {};
  const adjacency: Record<string, string[]> = {};
  const nodeIds = Object.keys(nodes);

  for (const id of nodeIds) {
    inDegree[id] = 0;
    adjacency[id] = [];
  }

  for (const [id, node] of Object.entries(nodes)) {
    for (const inputVal of Object.values(node.inputs)) {
      if (isLinkValue(inputVal)) {
        const sourceId = inputVal[0];
        if (nodes[sourceId]) {
          adjacency[sourceId].push(id);
          inDegree[id]++;
        }
      }
    }
  }

  const queue: string[] = [];
  for (const id of nodeIds) {
    if (inDegree[id] === 0) {
      queue.push(id);
    }
  }

  const order: string[] = [];
  while (queue.length > 0) {
    const current = queue.shift()!;
    order.push(current);

    for (const neighbor of adjacency[current]) {
      inDegree[neighbor]--;
      if (inDegree[neighbor] === 0) {
        queue.push(neighbor);
      }
    }
  }

  const errors: ValidationError[] = [];
  if (order.length !== nodeIds.length) {
    const cycleNodes = nodeIds.filter((id) => !order.includes(id));
    const cyclePath = cycleNodes.join(' -> ');
    errors.push({
      type: 'dependency_cycle',
      message: 'Dependency cycle detected',
      details: cyclePath,
      extraInfo: { cycleNodes },
    });
  }

  return { order, errors };
}

export interface GraphNode {
  id: string;
  classType: string;
  inputs: Record<string, unknown>;
}

export function validateConnection(
  sourceNodeType: string,
  sourceOutputIndex: number,
  targetNodeType: string,
  targetInputName: string,
  objectInfo: ObjectInfoMap
): ValidationError | null {
  const sourceDef = objectInfo[sourceNodeType];
  const targetDef = objectInfo[targetNodeType];

  if (!sourceDef) {
    return {
      type: 'missing_node_type',
      message: 'Source node type not found',
      details: sourceNodeType,
    };
  }

  if (!targetDef) {
    return {
      type: 'missing_node_type',
      message: 'Target node type not found',
      details: targetNodeType,
    };
  }

  if (sourceOutputIndex >= sourceDef.output_types.length) {
    return {
      type: 'invalid_output_index',
      message: 'Invalid output index',
      details: `Output index ${sourceOutputIndex} out of range for ${sourceNodeType}`,
    };
  }

  const outputType = sourceDef.output_types[sourceOutputIndex];

  const allInputs: Record<string, { type_name: string; extra: Record<string, unknown> }> = {
    ...(targetDef.input_types.required || {}),
    ...(targetDef.input_types.optional || {}),
  };

  const inputSpec = allInputs[targetInputName];
  if (!inputSpec) {
    return {
      type: 'invalid_input_name',
      message: 'Target input not found',
      details: `Input '${targetInputName}' not found on ${targetNodeType}`,
    };
  }

  const inputType = inputSpec.type_name as IoType;

  if (!validateNodeInput(outputType, inputType)) {
    return {
      type: 'return_type_mismatch',
      message: 'Return type mismatch between linked nodes',
      details: `${targetInputName}: output type(${outputType}) does not match input type(${inputType})`,
      inputName: targetInputName,
      extraInfo: {
        receivedType: outputType,
        inputType,
      },
    };
  }

  return null;
}

export function validatePrompt(
  nodes: GraphNode[],
  edges: Array<{ source: string; sourceHandle: string; target: string; targetHandle: string }>,
  objectInfo: ObjectInfoMap
): ValidationResult {
  const errors: ValidationError[] = [];

  const nodeMap: Record<string, GraphNode> = {};
  for (const node of nodes) {
    nodeMap[node.id] = node;
  }

  const outputNodes: string[] = [];
  for (const node of nodes) {
    const classDef = objectInfo[node.classType];
    if (classDef?.is_output_node) {
      outputNodes.push(node.id);
    }
  }

  if (outputNodes.length === 0) {
    errors.push({
      type: 'prompt_no_outputs',
      message: 'Prompt has no outputs',
      details: 'Add at least one output node (e.g., SaveImage)',
    });
  }

  for (const node of nodes) {
    const classDef = objectInfo[node.classType];
    if (!classDef) {
      errors.push({
        type: 'missing_node_type',
        message: `Node type '${node.classType}' not found`,
        details: `The custom node may not be installed`,
        nodeId: node.id,
      });
      continue;
    }

    const requiredInputs = classDef.input_types.required || {};
    for (const [inputName, spec] of Object.entries(requiredInputs)) {
      const inputSpec = spec as { type_name: string; extra: Record<string, unknown> };
      const isPrimitive = ['INT', 'FLOAT', 'STRING', 'BOOLEAN', 'COMBO'].includes(inputSpec.type_name);

      const hasConnection = edges.some(
        (e) => e.target === node.id && e.targetHandle === inputName
      );

      if (!hasConnection && !isPrimitive && node.inputs[inputName] === undefined) {
        errors.push({
          type: 'required_input_missing',
          message: 'Required input is missing',
          details: inputName,
          nodeId: node.id,
          inputName,
        });
      }
    }
  }

  for (const edge of edges) {
    const sourceNode = nodeMap[edge.source];
    const targetNode = nodeMap[edge.target];
    if (!sourceNode || !targetNode) continue;

    const sourceDef = objectInfo[sourceNode.classType];
    const targetDef = objectInfo[targetNode.classType];
    if (!sourceDef || !targetDef) continue;

    const sourceOutputIndex = sourceDef.output_names.indexOf(edge.sourceHandle);
    if (sourceOutputIndex < 0) continue;

    const connError = validateConnection(
      sourceNode.classType,
      sourceOutputIndex,
      targetNode.classType,
      edge.targetHandle,
      objectInfo
    );
    if (connError) {
      connError.nodeId = targetNode.id;
      errors.push(connError);
    }
  }

  const promptNodes: Record<string, { inputs: Record<string, unknown> }> = {};
  for (const node of nodes) {
    const inputs: Record<string, unknown> = {};
    for (const [key, value] of Object.entries(node.inputs)) {
      inputs[key] = value;
    }
    for (const edge of edges) {
      if (edge.target === node.id && edge.targetHandle) {
        const sourceDef = objectInfo[nodeMap[edge.source]?.classType || ''];
        if (sourceDef) {
          const outputIndex = sourceDef.output_names.indexOf(edge.sourceHandle);
          if (outputIndex >= 0) {
            inputs[edge.targetHandle] = [edge.source, outputIndex];
          }
        }
      }
    }
    promptNodes[node.id] = { inputs };
  }

  const cycle = detectCycleInGraph(promptNodes);
  if (cycle) {
    const cyclePath = cycle.join(' -> ');
    for (const nodeId of cycle) {
      errors.push({
        type: 'dependency_cycle',
        message: 'Dependency cycle detected',
        details: cyclePath,
        nodeId,
        extraInfo: { cycleNodes: cycle },
      });
    }
  }

  return {
    valid: errors.length === 0,
    errors,
  };
}

export function getExecutionOrder(
  nodes: GraphNode[],
  edges: Array<{ source: string; sourceHandle: string; target: string; targetHandle: string }>,
  objectInfo: ObjectInfoMap
): ExecutionOrderResult {
  const promptNodes: Record<string, { inputs: Record<string, unknown> }> = {};
  for (const node of nodes) {
    const inputs: Record<string, unknown> = {};
    for (const [key, value] of Object.entries(node.inputs)) {
      inputs[key] = value;
    }
    for (const edge of edges) {
      if (edge.target === node.id && edge.targetHandle) {
        const sourceDef = objectInfo[nodes.find((n) => n.id === edge.source)?.classType || ''];
        if (sourceDef) {
          const outputIndex = sourceDef.output_names.indexOf(edge.sourceHandle);
          if (outputIndex >= 0) {
            inputs[edge.targetHandle] = [edge.source, outputIndex];
          }
        }
      }
    }
    promptNodes[node.id] = { inputs };
  }

  return topologicalSort(promptNodes);
}

function isLinkValue(value: unknown): value is [string, number] {
  return Array.isArray(value) && value.length === 2 && typeof value[0] === 'string' && typeof value[1] === 'number';
}
