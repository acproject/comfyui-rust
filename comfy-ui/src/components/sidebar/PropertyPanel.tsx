import { type FC } from 'react';
import { useWorkflowStore } from '@/store/workflow';

const PropertyPanel: FC = () => {
  const selectedNodeId = useWorkflowStore((s) => s.selectedNodeId);
  const nodes = useWorkflowStore((s) => s.nodes);
  const objectInfo = useWorkflowStore((s) => s.objectInfo);
  const updateNodeInput = useWorkflowStore((s) => s.updateNodeInput);

  const node = nodes.find((n) => n.id === selectedNodeId);
  if (!node) {
    return (
      <div style={panelStyle}>
        <div style={headerStyle}>Properties</div>
        <div style={{ padding: 12, color: '#718096', fontSize: 12 }}>
          Select a node to view properties
        </div>
      </div>
    );
  }

  const classDef = objectInfo[node.data.classType];
  const requiredInputs = classDef?.input_types?.required || {};
  const optionalInputs = classDef?.input_types?.optional || {};

  return (
    <div style={panelStyle}>
      <div style={headerStyle}>
        {node.data.title}
        <span style={{ fontSize: 10, color: '#718096', marginLeft: 8 }}>
          #{node.id}
        </span>
      </div>

      <div style={{ padding: '8px 0' }}>
        {Object.entries(requiredInputs).map(([name, spec]) => (
          <PropertyField
            key={name}
            name={name}
            spec={spec as { type_name: string; extra: Record<string, unknown> }}
            value={node.data.inputs[name]}
            onChange={(v) => updateNodeInput(node.id, name, v)}
          />
        ))}

        {Object.keys(optionalInputs).length > 0 && (
          <div style={{ padding: '4px 12px', fontSize: 10, color: '#718096', textTransform: 'uppercase' }}>
            Optional
          </div>
        )}

        {Object.entries(optionalInputs).map(([name, spec]) => (
          <PropertyField
            key={name}
            name={name}
            spec={spec as { type_name: string; extra: Record<string, unknown> }}
            value={node.data.inputs[name]}
            onChange={(v) => updateNodeInput(node.id, name, v)}
          />
        ))}
      </div>
    </div>
  );
};

interface PropertyFieldProps {
  name: string;
  spec: { type_name: string; extra: Record<string, unknown> };
  value: unknown;
  onChange: (value: unknown) => void;
}

const PropertyField: FC<PropertyFieldProps> = ({ name, spec, value, onChange }) => {
  const { type_name, extra } = spec;
  const choices = extra.choices as string[] | undefined;

  if (choices && choices.length > 0) {
    return (
      <div style={fieldStyle}>
        <label style={labelStyle}>{name}</label>
        <select
          value={String(value || '')}
          onChange={(e) => onChange(e.target.value)}
          style={selectStyle}
        >
          {choices.map((c) => (
            <option key={c} value={c}>
              {c}
            </option>
          ))}
        </select>
      </div>
    );
  }

  if (type_name === 'BOOLEAN') {
    return (
      <div style={fieldStyle}>
        <label style={labelStyle}>{name}</label>
        <input
          type="checkbox"
          checked={!!value}
          onChange={(e) => onChange(e.target.checked)}
          style={{ cursor: 'pointer' }}
        />
      </div>
    );
  }

  if (type_name === 'INT') {
    const min = (extra.min as number) ?? 0;
    const max = (extra.max as number) ?? 10000;
    const step = (extra.step as number) ?? 1;
    return (
      <div style={fieldStyle}>
        <label style={labelStyle}>{name}</label>
        <div style={{ display: 'flex', alignItems: 'center', gap: 6 }}>
          <input
            type="range"
            value={Number(value) || 0}
            min={min}
            max={max}
            step={step}
            onChange={(e) => onChange(parseInt(e.target.value, 10) || 0)}
            style={{ flex: 1, accentColor: '#5a6abf' }}
          />
          <input
            type="number"
            value={Number(value) || 0}
            min={min}
            max={max}
            step={step}
            onChange={(e) => onChange(parseInt(e.target.value, 10) || 0)}
            style={{ ...inputStyle, width: 60 }}
          />
        </div>
      </div>
    );
  }

  if (type_name === 'FLOAT') {
    const min = (extra.min as number) ?? 0;
    const max = (extra.max as number) ?? 1;
    const step = (extra.step as number) ?? 0.01;
    return (
      <div style={fieldStyle}>
        <label style={labelStyle}>{name}</label>
        <div style={{ display: 'flex', alignItems: 'center', gap: 6 }}>
          <input
            type="range"
            value={Number(value) || 0}
            min={min}
            max={max}
            step={step}
            onChange={(e) => onChange(parseFloat(e.target.value) || 0)}
            style={{ flex: 1, accentColor: '#5a6abf' }}
          />
          <input
            type="number"
            value={Number(value) || 0}
            min={min}
            max={max}
            step={step}
            onChange={(e) => onChange(parseFloat(e.target.value) || 0)}
            style={{ ...inputStyle, width: 60 }}
          />
        </div>
      </div>
    );
  }

  if (type_name === 'STRING') {
    const multiline = extra.multiline as boolean;
    if (multiline) {
      return (
        <div style={fieldStyle}>
          <label style={labelStyle}>{name}</label>
          <textarea
            value={String(value || '')}
            onChange={(e) => onChange(e.target.value)}
            style={{ ...inputStyle, minHeight: 80, resize: 'vertical' }}
          />
        </div>
      );
    }
    return (
      <div style={fieldStyle}>
        <label style={labelStyle}>{name}</label>
        <input
          type="text"
          value={String(value || '')}
          onChange={(e) => onChange(e.target.value)}
          style={inputStyle}
        />
      </div>
    );
  }

  return (
    <div style={fieldStyle}>
      <label style={labelStyle}>{name}</label>
      <span style={{ color: '#718096', fontSize: 11 }}>{type_name}</span>
    </div>
  );
};

const panelStyle: React.CSSProperties = {
  background: '#1e1e2e',
  display: 'flex',
  flexDirection: 'column',
  height: '100%',
  color: '#e2e8f0',
  overflowY: 'auto',
};

const headerStyle: React.CSSProperties = {
  padding: '8px 12px',
  borderBottom: '1px solid #333',
  fontWeight: 600,
  fontSize: 13,
  display: 'flex',
  alignItems: 'center',
};

const fieldStyle: React.CSSProperties = {
  padding: '4px 12px',
  display: 'flex',
  flexDirection: 'column',
  gap: 2,
};

const labelStyle: React.CSSProperties = {
  fontSize: 11,
  color: '#a0aec0',
};

const inputStyle: React.CSSProperties = {
  background: '#2a2a3e',
  border: '1px solid #444',
  borderRadius: 4,
  color: '#e2e8f0',
  padding: '4px 8px',
  fontSize: 12,
  outline: 'none',
};

const selectStyle: React.CSSProperties = {
  ...inputStyle,
  cursor: 'pointer',
};

export { PropertyPanel };
