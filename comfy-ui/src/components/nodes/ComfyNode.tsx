import { memo, type FC, useCallback, useRef, useState } from 'react';
import { Handle, Position } from '@xyflow/react';
import type { ComfyNodeData } from '@/store/workflow';
import { useWorkflowStore } from '@/store/workflow';
import { api } from '@/api/client';
import { getTypeColor, getCategoryColor, isCustomNode } from '@/components/nodes/nodeColors';

interface ComfyNodeProps {
  id: string;
  data: ComfyNodeData;
  selected: boolean;
}

const HEADER_H = 28;
const ROW_H = 24;
const SEP_H = 2;

const ComfyNodeComponent: FC<ComfyNodeProps> = memo(({ id, data, selected }) => {
  const { title, outputs, isOutputNode, category, classType } = data;
  const headerColor = getCategoryColor(category);
  const [collapsed, setCollapsed] = useState(false);

  const objectInfo = useWorkflowStore((s) => s.objectInfo[data.classType]);
  const executingNodeId = useWorkflowStore((s) => s.executingNodeId);
  const executedNodeIds = useWorkflowStore((s) => s.executedNodeIds);
  const cachedNodeIds = useWorkflowStore((s) => s.cachedNodeIds);
  const isExecuting = executingNodeId === id;
  const isExecuted = executedNodeIds.includes(id);
  const isCached = cachedNodeIds.includes(id);
  const isCompleted = isExecuted || isCached;

  const allInputSpecs: Array<{
    name: string;
    typeName: string;
    optional: boolean;
    choices?: string[];
    multiline?: boolean;
  }> = [];

  if (objectInfo?.input_types?.required) {
    for (const [k, v] of Object.entries(objectInfo.input_types.required)) {
      const spec = v as { type_name: string; extra?: Record<string, unknown> };
      allInputSpecs.push({
        name: k,
        typeName: spec.type_name,
        optional: false,
        choices: (spec.extra?.choices as string[]) || undefined,
        multiline: spec.extra?.multiline === true || (k === 'text' && spec.type_name === 'STRING'),
      });
    }
  }
  if (objectInfo?.input_types?.optional) {
    for (const [k, v] of Object.entries(objectInfo.input_types.optional)) {
      const spec = v as { type_name: string; extra?: Record<string, unknown> };
      allInputSpecs.push({
        name: k,
        typeName: spec.type_name,
        optional: true,
        choices: (spec.extra?.choices as string[]) || undefined,
        multiline: spec.extra?.multiline === true || (k === 'text' && spec.type_name === 'STRING'),
      });
    }
  }

  const isImageNode = classType === 'LoadImage';
  const isSaveImageNode = classType === 'SaveImage';

  const isPrimitive = (typeName: string) =>
    ['INT', 'FLOAT', 'STRING', 'BOOLEAN', 'COMBO'].includes(typeName);

  const nonPrimitiveInputs = allInputSpecs.filter((s) => !isPrimitive(s.typeName));

  let y = HEADER_H;
  if (!collapsed) {
    if (isImageNode) y += 80;
    if (isSaveImageNode) y += 80;
  }

  const inputHandleY: Record<string, number> = {};
  for (const spec of allInputSpecs) {
    const rowH = (!collapsed && spec.multiline) ? ROW_H * 3 : ROW_H;
    inputHandleY[spec.name] = y + rowH / 2;
    y += rowH;
  }

  if (!collapsed && outputs.length > 0) {
    y += SEP_H;
  }

  const outputHandleY: Record<string, number> = {};
  for (const output of outputs) {
    outputHandleY[output.name] = y + ROW_H / 2;
    y += ROW_H;
  }

  return (
    <div
      style={{
        background: '#1e1e2e',
        borderRadius: 6,
        border: isExecuting
          ? '2px solid #f59e0b'
          : isCompleted
            ? '2px solid #22c55e'
            : selected
              ? '2px solid #fff'
              : '1px solid #333',
        minWidth: 220,
        maxWidth: 280,
        fontSize: 12,
        color: '#e2e8f0',
        boxShadow: isExecuting
          ? '0 0 16px rgba(245, 158, 11, 0.6), 0 0 4px rgba(245, 158, 11, 0.3)'
          : isCompleted
            ? '0 0 12px rgba(34, 197, 94, 0.4)'
            : selected
              ? '0 0 12px rgba(100, 150, 255, 0.4)'
              : '0 2px 8px rgba(0,0,0,0.3)',
        transition: 'border-color 0.3s, box-shadow 0.3s',
      }}
    >
      {nonPrimitiveInputs.map((spec) => (
        <Handle
          key={`in-${spec.name}`}
          type="target"
          position={Position.Left}
          id={spec.name}
          style={{
            background: getTypeColor(spec.typeName),
            width: 12,
            height: 12,
            border: '2px solid #1e1e2e',
            top: inputHandleY[spec.name],
          }}
        />
      ))}

      {outputs.map((output) => (
        <Handle
          key={`out-${output.name}`}
          type="source"
          position={Position.Right}
          id={output.name}
          style={{
            background: getTypeColor(output.type),
            width: 12,
            height: 12,
            border: '2px solid #1e1e2e',
            top: outputHandleY[output.name],
          }}
        />
      ))}

      <div
        style={{
          background: headerColor,
          padding: '5px 8px',
          fontWeight: 600,
          fontSize: 11,
          display: 'flex',
          justifyContent: 'space-between',
          alignItems: 'center',
          borderRadius: '4px 4px 0 0',
          cursor: 'pointer',
          userSelect: 'none',
          height: HEADER_H,
          boxSizing: 'border-box',
        }}
        onDoubleClick={() => setCollapsed(!collapsed)}
      >
        <span style={{ overflow: 'hidden', textOverflow: 'ellipsis', whiteSpace: 'nowrap', flex: 1 }}>
          {title}
        </span>
        <div style={{ display: 'flex', alignItems: 'center', gap: 4 }}>
          {isExecuting && (
            <span style={{
              fontSize: 8,
              background: 'rgba(245, 158, 11, 0.3)',
              color: '#f59e0b',
              padding: '1px 4px',
              borderRadius: 3,
              animation: 'pulse 1.5s ease-in-out infinite',
            }}>
              ●
            </span>
          )}
          {isCompleted && !isExecuting && (
            <span style={{
              fontSize: 8,
              background: 'rgba(34, 197, 94, 0.3)',
              color: '#22c55e',
              padding: '1px 4px',
              borderRadius: 3,
            }}>
              ✓
            </span>
          )}
          {isCustomNode(classType) && (
            <span style={{
              fontSize: 8,
              background: 'rgba(139,107,191,0.3)',
              color: '#b39ddb',
              padding: '1px 4px',
              borderRadius: 3,
            }}>
              custom
            </span>
          )}
          <span style={{ fontSize: 9, opacity: 0.6 }}>#{id}</span>
          {isOutputNode && (
            <span style={{
              fontSize: 8,
              background: 'rgba(255,255,255,0.2)',
              padding: '1px 4px',
              borderRadius: 3,
            }}>
              SAVE
            </span>
          )}
          <span style={{ fontSize: 9, opacity: 0.5 }}>{collapsed ? '▸' : '▾'}</span>
        </div>
      </div>

      {!collapsed && (
        <>
          {(isImageNode) && (
            <ImagePreview filename={data.inputs['image'] ? String(data.inputs['image']) : ''} type="input" />
          )}
          {(isSaveImageNode) && (
            <OutputPreview nodeId={id} />
          )}

          <div style={{ padding: '2px 0' }}>
            {allInputSpecs.map((spec) => {
              const { name, typeName, choices, multiline } = spec;
              const value = data.inputs[name];
              const typeColor = getTypeColor(typeName);
              const showHandle = !isPrimitive(typeName);

              return (
                <div
                  key={name}
                  style={{
                    padding: '2px 8px',
                    display: 'flex',
                    alignItems: multiline ? 'flex-start' : 'center',
                    gap: 4,
                    height: multiline ? ROW_H * 3 : ROW_H,
                    boxSizing: 'border-box',
                    paddingLeft: showHandle ? 18 : 8,
                  }}
                >
                  {showHandle && (
                    <span style={{
                      width: 8,
                      height: 8,
                      borderRadius: '50%',
                      background: typeColor,
                      border: '1.5px solid #1e1e2e',
                      flexShrink: 0,
                      marginTop: multiline ? 7 : 0,
                    }} />
                  )}
                  <span style={{
                    flex: '0 0 auto',
                    fontSize: 10,
                    color: '#a0aec0',
                    overflow: 'hidden',
                    textOverflow: 'ellipsis',
                    whiteSpace: 'nowrap',
                    lineHeight: multiline ? undefined : '20px',
                    paddingTop: multiline ? 3 : 0,
                  }}>
                    {name}
                  </span>
                  {isPrimitive(typeName) && (
                    <NodeInputField
                      nodeId={id}
                      name={name}
                      value={value}
                      typeName={typeName}
                      choices={choices}
                      classType={classType}
                      multiline={multiline}
                    />
                  )}
                </div>
              );
            })}
          </div>

          {outputs.length > 0 && (
            <>
              <div style={{ height: SEP_H, background: '#333' }} />
              <div style={{ padding: '2px 0' }}>
                {outputs.map((output) => {
                  const typeColor = getTypeColor(output.type);
                  return (
                    <div
                      key={output.name}
                      style={{
                        padding: '2px 8px',
                        textAlign: 'right',
                        fontSize: 10,
                        color: '#a0aec0',
                        height: ROW_H,
                        display: 'flex',
                        alignItems: 'center',
                        justifyContent: 'flex-end',
                        gap: 4,
                        boxSizing: 'border-box',
                        paddingRight: 18,
                      }}
                    >
                      <span style={{ overflow: 'hidden', textOverflow: 'ellipsis', whiteSpace: 'nowrap' }}>
                        {output.name}
                      </span>
                      <span style={{
                        fontSize: 8,
                        color: typeColor,
                        background: `${typeColor}22`,
                        padding: '0px 3px',
                        borderRadius: 2,
                      }}>
                        {output.type}
                      </span>
                      <span style={{
                        width: 8,
                        height: 8,
                        borderRadius: '50%',
                        background: typeColor,
                        border: '1.5px solid #1e1e2e',
                        flexShrink: 0,
                      }} />
                    </div>
                  );
                })}
              </div>
            </>
          )}
        </>
      )}
    </div>
  );
});

ComfyNodeComponent.displayName = 'ComfyNode';

const ImagePreview: FC<{ filename: string; type: string }> = memo(({ filename, type }) => {
  if (!filename) {
    return (
      <div style={{
        padding: '4px 6px',
        borderBottom: '1px solid #333',
        height: 72,
        display: 'flex',
        alignItems: 'center',
        justifyContent: 'center',
        background: '#111',
        borderRadius: 3,
        margin: '4px 6px',
      }}>
        <span style={{ fontSize: 10, color: '#666' }}>Preview</span>
      </div>
    );
  }
  const url = type === 'input'
    ? api.getInputImageUrl(filename)
    : `${window.location.origin}/view?filename=${encodeURIComponent(filename)}`;

  return (
    <div style={{
      padding: '4px 6px',
      borderBottom: '1px solid #333',
      height: 80,
      overflow: 'hidden',
    }}>
      <img
        src={url}
        alt={filename}
        style={{
          width: '100%',
          height: '100%',
          objectFit: 'contain',
          borderRadius: 3,
          background: '#111',
          display: 'block',
        }}
        onError={(e) => {
          (e.target as HTMLImageElement).style.display = 'none';
        }}
      />
    </div>
  );
});

ImagePreview.displayName = 'ImagePreview';

const OutputPreview: FC<{ nodeId: string }> = memo(({ nodeId }) => {
  const outputImages = useWorkflowStore((s) => s.outputImages);
  const images = outputImages[nodeId];

  if (!images || images.length === 0) {
    return (
      <div style={{
        padding: '4px 6px',
        borderBottom: '1px solid #333',
        height: 72,
        display: 'flex',
        alignItems: 'center',
        justifyContent: 'center',
        background: '#111',
        borderRadius: 3,
        margin: '4px 6px',
      }}>
        <span style={{ fontSize: 10, color: '#666' }}>Preview</span>
      </div>
    );
  }

  return (
    <div style={{
      padding: '4px 6px',
      borderBottom: '1px solid #333',
      height: 80,
      overflow: 'hidden',
    }}>
      {images.map((img, i) => (
        <img
          key={i}
          src={`${window.location.origin}/view?filename=${encodeURIComponent(img.filename)}&subfolder=${encodeURIComponent(img.subfolder || '')}`}
          alt={img.filename}
          style={{
            width: '100%',
            height: '100%',
            objectFit: 'contain',
            borderRadius: 3,
            background: '#111',
            display: 'block',
          }}
          onError={(e) => {
            (e.target as HTMLImageElement).style.display = 'none';
          }}
        />
      ))}
    </div>
  );
});

OutputPreview.displayName = 'OutputPreview';

interface NodeInputFieldProps {
  nodeId: string;
  name: string;
  value: unknown;
  typeName: string;
  choices?: string[];
  classType: string;
  multiline?: boolean;
}

const NodeInputField: FC<NodeInputFieldProps> = memo(({ nodeId, name, value, typeName, choices, classType, multiline }) => {
  const updateNodeInput = useWorkflowStore((s) => s.updateNodeInput);
  const fileInputRef = useRef<HTMLInputElement>(null);

  const handleChange = useCallback(
    (newValue: unknown) => {
      updateNodeInput(nodeId, name, newValue);
    },
    [nodeId, name, updateNodeInput]
  );

  const handleUpload = useCallback(
    async (e: React.ChangeEvent<HTMLInputElement>) => {
      const file = e.target.files?.[0];
      if (!file) return;
      try {
        const result = await api.uploadInputImage(file);
        handleChange(result.name);
      } catch (err) {
        console.error('Failed to upload image:', err);
      }
    },
    [handleChange]
  );

  const baseStyle: React.CSSProperties = {
    background: '#2a2a3e',
    border: '1px solid #444',
    borderRadius: 3,
    color: '#e2e8f0',
    padding: '1px 4px',
    fontSize: 10,
    outline: 'none',
    flex: 1,
    minWidth: 60,
  };

  if (typeName === 'BOOLEAN') {
    return (
      <input
        type="checkbox"
        checked={!!value}
        onChange={(e) => handleChange(e.target.checked)}
        style={{ width: 12, height: 12, cursor: 'pointer', marginTop: 4 }}
      />
    );
  }

  if (typeName === 'INT') {
    return (
      <input
        type="number"
        value={Number(value) || 0}
        onChange={(e) => handleChange(parseInt(e.target.value, 10) || 0)}
        style={{ ...baseStyle, width: 70 }}
      />
    );
  }

  if (typeName === 'FLOAT') {
    return (
      <input
        type="number"
        step="0.1"
        value={Number(value) || 0}
        onChange={(e) => handleChange(parseFloat(e.target.value) || 0)}
        style={{ ...baseStyle, width: 70 }}
      />
    );
  }

  if (typeName === 'STRING') {
    if (multiline) {
      return (
        <textarea
          value={String(value || '')}
          onChange={(e) => handleChange(e.target.value)}
          rows={3}
          style={{
            ...baseStyle,
            width: '100%',
            resize: 'vertical',
            fontFamily: 'inherit',
            lineHeight: 1.4,
          }}
        />
      );
    }
    return (
      <input
        type="text"
        value={String(value || '')}
        onChange={(e) => handleChange(e.target.value)}
        style={{ ...baseStyle, width: 90 }}
      />
    );
  }

  if (typeName === 'COMBO') {
    const isImageField = classType === 'LoadImage' && name === 'image';
    return (
      <div style={{ display: 'flex', alignItems: 'center', gap: 2, flex: 1 }}>
        <select
          value={String(value || '')}
          onChange={(e) => handleChange(e.target.value)}
          style={{
            ...baseStyle,
            width: isImageField ? 70 : 90,
            cursor: 'pointer',
          }}
        >
          {(choices || []).map((c) => (
            <option key={c} value={c}>
              {c.length > 16 ? c.slice(0, 13) + '…' : c}
            </option>
          ))}
        </select>
        {isImageField && (
          <>
            <button
              onClick={() => fileInputRef.current?.click()}
              title="Upload image"
              style={{
                background: '#4a5568',
                border: '1px solid #5a6578',
                borderRadius: 3,
                color: '#e2e8f0',
                padding: '1px 5px',
                fontSize: 10,
                cursor: 'pointer',
                lineHeight: 1,
              }}
            >
              📁
            </button>
            <input
              ref={fileInputRef}
              type="file"
              accept="image/*"
              style={{ display: 'none' }}
              onChange={handleUpload}
            />
          </>
        )}
      </div>
    );
  }

  return <span style={{ fontSize: 9, color: '#718096' }}>{String(value)}</span>;
});

NodeInputField.displayName = 'NodeInputField';

export { ComfyNodeComponent };
