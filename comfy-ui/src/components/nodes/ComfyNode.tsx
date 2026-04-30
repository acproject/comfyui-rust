import { memo, type FC, useCallback, useRef, useState } from 'react';
import { Handle, Position } from '@xyflow/react';
import type { ComfyNodeData } from '@/store/workflow';
import { useWorkflowStore } from '@/store/workflow';
import { api } from '@/api/client';
import { getTypeColor, getCategoryColor } from '@/components/nodes/nodeColors';

interface ComfyNodeProps {
  id: string;
  data: ComfyNodeData;
  selected: boolean;
}

const ComfyNodeComponent: FC<ComfyNodeProps> = memo(({ id, data, selected }) => {
  const { title, outputs, isOutputNode, category } = data;
  const headerColor = getCategoryColor(category);
  const [collapsed, setCollapsed] = useState(false);

  const objectInfo = useWorkflowStore((s) => s.objectInfo[data.classType]);

  const inputTypeMap: Record<string, string> = {};
  if (objectInfo?.input_types?.required) {
    for (const [k, v] of Object.entries(objectInfo.input_types.required)) {
      inputTypeMap[k] = (v as { type_name: string }).type_name;
    }
  }
  if (objectInfo?.input_types?.optional) {
    for (const [k, v] of Object.entries(objectInfo.input_types.optional)) {
      inputTypeMap[k] = (v as { type_name: string }).type_name;
    }
  }

  const inputEntries = Object.entries(data.inputs);
  const isImageNode = data.classType === 'LoadImage';
  const isSaveImageNode = data.classType === 'SaveImage';

  const isPrimitive = (typeName: string) =>
    ['INT', 'FLOAT', 'STRING', 'BOOLEAN', 'COMBO'].includes(typeName);

  return (
    <div
      style={{
        background: '#1e1e2e',
        borderRadius: 6,
        border: selected ? '2px solid #fff' : '1px solid #333',
        minWidth: 220,
        maxWidth: 280,
        fontSize: 12,
        color: '#e2e8f0',
        overflow: 'visible',
        boxShadow: selected
          ? '0 0 12px rgba(100, 150, 255, 0.4)'
          : '0 2px 8px rgba(0,0,0,0.3)',
      }}
    >
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
        }}
        onDoubleClick={() => setCollapsed(!collapsed)}
      >
        <span style={{ overflow: 'hidden', textOverflow: 'ellipsis', whiteSpace: 'nowrap', flex: 1 }}>
          {title}
        </span>
        <div style={{ display: 'flex', alignItems: 'center', gap: 4 }}>
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
          {(isImageNode && data.inputs['image']) && (
            <ImagePreview filename={String(data.inputs['image'])} type="input" />
          )}
          {(isSaveImageNode && data.inputs['images']) && (
            <OutputPreview nodeId={id} />
          )}

          <div style={{ padding: '2px 0' }}>
            {inputEntries.map(([name, value]) => {
              const typeName = inputTypeMap[name] || '';
              const showHandle = !isPrimitive(typeName);
              const typeColor = getTypeColor(typeName);

              const inputSpec = objectInfo?.input_types?.required?.[name] || objectInfo?.input_types?.optional?.[name];
              const choices = (inputSpec as { extra?: { choices?: string[] } })?.extra?.choices;

              return (
                <div
                  key={name}
                  style={{
                    position: 'relative',
                    padding: '2px 8px',
                    display: 'flex',
                    alignItems: 'center',
                    gap: 4,
                    minHeight: 22,
                  }}
                >
                  {showHandle && (
                    <Handle
                      type="target"
                      position={Position.Left}
                      id={name}
                      style={{
                        background: typeColor,
                        width: 12,
                        height: 12,
                        border: '2px solid #1e1e2e',
                        top: 'auto',
                        position: 'relative',
                        transform: 'none',
                        left: -16,
                      }}
                    />
                  )}
                  <span style={{
                    flex: 1,
                    fontSize: 10,
                    color: '#a0aec0',
                    overflow: 'hidden',
                    textOverflow: 'ellipsis',
                    whiteSpace: 'nowrap',
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
                      classType={data.classType}
                    />
                  )}
                </div>
              );
            })}
          </div>

          {outputs.length > 0 && (
            <div style={{ padding: '2px 0', borderTop: '1px solid #333' }}>
              {outputs.map((output) => {
                const typeColor = getTypeColor(output.type);
                return (
                  <div
                    key={output.name}
                    style={{
                      position: 'relative',
                      padding: '2px 8px',
                      textAlign: 'right',
                      fontSize: 10,
                      color: '#a0aec0',
                      minHeight: 22,
                      display: 'flex',
                      alignItems: 'center',
                      justifyContent: 'flex-end',
                      gap: 4,
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
                    <Handle
                      type="source"
                      position={Position.Right}
                      id={output.name}
                      style={{
                        background: typeColor,
                        width: 12,
                        height: 12,
                        border: '2px solid #1e1e2e',
                        top: 'auto',
                        position: 'relative',
                        transform: 'none',
                        right: -16,
                      }}
                    />
                  </div>
                );
              })}
            </div>
          )}
        </>
      )}
    </div>
  );
});

ComfyNodeComponent.displayName = 'ComfyNode';

const ImagePreview: FC<{ filename: string; type: string }> = memo(({ filename, type }) => {
  if (!filename) return null;
  const url = type === 'input'
    ? api.getInputImageUrl(filename)
    : `${window.location.origin}/view?filename=${encodeURIComponent(filename)}`;

  return (
    <div style={{
      padding: '4px 6px',
      borderBottom: '1px solid #333',
    }}>
      <img
        src={url}
        alt={filename}
        style={{
          width: '100%',
          maxHeight: 160,
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

  if (!images || images.length === 0) return null;

  return (
    <div style={{
      padding: '4px 6px',
      borderBottom: '1px solid #333',
    }}>
      {images.map((img, i) => (
        <img
          key={i}
          src={`${window.location.origin}/view?filename=${encodeURIComponent(img.filename)}&subfolder=${encodeURIComponent(img.subfolder || '')}`}
          alt={img.filename}
          style={{
            width: '100%',
            maxHeight: 160,
            objectFit: 'contain',
            borderRadius: 3,
            background: '#111',
            display: 'block',
            marginBottom: i < images.length - 1 ? 4 : 0,
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
}

const NodeInputField: FC<NodeInputFieldProps> = memo(({ nodeId, name, value, typeName, choices, classType }) => {
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

  const inputStyle: React.CSSProperties = {
    width: 80,
    background: '#2a2a3e',
    border: '1px solid #444',
    borderRadius: 3,
    color: '#e2e8f0',
    padding: '1px 4px',
    fontSize: 10,
    outline: 'none',
  };

  if (typeName === 'BOOLEAN') {
    return (
      <input
        type="checkbox"
        checked={!!value}
        onChange={(e) => handleChange(e.target.checked)}
        style={{ width: 12, height: 12, cursor: 'pointer' }}
      />
    );
  }

  if (typeName === 'INT') {
    return (
      <input
        type="number"
        value={Number(value) || 0}
        onChange={(e) => handleChange(parseInt(e.target.value, 10) || 0)}
        style={inputStyle}
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
        style={inputStyle}
      />
    );
  }

  if (typeName === 'STRING') {
    return (
      <input
        type="text"
        value={String(value || '')}
        onChange={(e) => handleChange(e.target.value)}
        style={{ ...inputStyle, width: 90 }}
      />
    );
  }

  if (typeName === 'COMBO') {
    const isImageField = classType === 'LoadImage' && name === 'image';
    return (
      <div style={{ display: 'flex', alignItems: 'center', gap: 2 }}>
        <select
          value={String(value || '')}
          onChange={(e) => handleChange(e.target.value)}
          style={{
            ...inputStyle,
            width: isImageField ? 70 : 90,
            cursor: 'pointer',
          }}
        >
          {(choices || []).map((c) => (
            <option key={c} value={c}>
              {c.length > 16 ? '…' + c.slice(-14) : c}
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
