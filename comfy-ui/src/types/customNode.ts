import type { NodeClassDef, IoType } from '@/types/api';

export interface CustomNodeInputDef {
  name: string;
  type: IoType;
  required: boolean;
  default?: unknown;
  extra?: {
    min?: number;
    max?: number;
    step?: number;
    multiline?: boolean;
    choices?: string[];
    tooltip?: string;
  };
}

export interface CustomNodeOutputDef {
  name: string;
  type: IoType;
}

export interface CustomNodeDef {
  id: string;
  classType: string;
  displayName: string;
  category: string;
  description: string;
  inputs: CustomNodeInputDef[];
  outputs: CustomNodeOutputDef[];
  isOutputNode: boolean;
  executeCode?: string;
  createdAt: number;
  updatedAt: number;
}

export function customNodeDefToNodeClassDef(def: CustomNodeDef): NodeClassDef {
  const required: Record<string, { type_name: string; extra: Record<string, unknown> }> = {};
  const optional: Record<string, { type_name: string; extra: Record<string, unknown> }> = {};

  for (const input of def.inputs) {
    const spec: { type_name: string; extra: Record<string, unknown> } = {
      type_name: input.type,
      extra: {
        ...(input.extra || {}),
        ...(input.default !== undefined ? { default: input.default } : {}),
      },
    };

    if (input.extra?.choices && input.extra.choices.length > 0) {
      spec.type_name = 'COMBO';
      spec.extra.choices = input.extra.choices;
    }

    if (input.required) {
      required[input.name] = spec;
    } else {
      optional[input.name] = spec;
    }
  }

  return {
    class_type: def.classType,
    display_name: def.displayName,
    category: def.category,
    input_types: {
      required,
      ...(Object.keys(optional).length > 0 ? { optional } : {}),
    },
    output_types: def.outputs.map((o) => o.type),
    output_names: def.outputs.map((o) => o.name),
    output_is_list: def.outputs.map(() => false),
    is_output_node: def.isOutputNode,
    has_intermediate_output: false,
    function_name: 'execute',
  };
}

export const PRIMITIVE_TYPES: IoType[] = [
  'STRING',
  'INT',
  'FLOAT',
  'BOOLEAN',
];

export const COMMON_TYPES: IoType[] = [
  'MODEL',
  'CLIP',
  'VAE',
  'IMAGE',
  'MASK',
  'LATENT',
  'CONDITIONING',
  'CONTROL_NET',
];

export function generateClassType(displayName: string): string {
  const sanitized = displayName
    .replace(/[^a-zA-Z0-9_]/g, '_')
    .replace(/_+/g, '_')
    .replace(/^_|_$/g, '');
  return `Custom_${sanitized}`;
}
