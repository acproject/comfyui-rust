import { useState, useCallback, useRef, useEffect, useMemo, type FC } from 'react';
import { createPortal } from 'react-dom';
import { Plus, Trash2, ChevronDown, ChevronRight } from 'lucide-react';
import type { CustomNodeDef, CustomNodeInputDef, CustomNodeOutputDef } from '@/types/customNode';
import { PRIMITIVE_TYPES, COMPLEX_TYPES, collectTypesFromObjectInfo, generateClassType } from '@/types/customNode';
import type { IoType } from '@/types/api';
import { useWorkflowStore } from '@/store/workflow';

interface CustomNodeEditorProps {
  initialNode?: CustomNodeDef | null;
  onSave: (node: CustomNodeDef) => void;
  onCancel: () => void;
}

const CustomNodeEditor: FC<CustomNodeEditorProps> = ({ initialNode, onSave, onCancel }) => {
  const isEditing = !!initialNode;
  const objectInfo = useWorkflowStore((s) => s.objectInfo);

  const allTypes = useMemo(() => {
    const dynamicTypes = collectTypesFromObjectInfo(objectInfo);
    const staticTypes = [...PRIMITIVE_TYPES, ...COMPLEX_TYPES, '*'];
    const merged = new Set<IoType>([...staticTypes, ...dynamicTypes]);
    return Array.from(merged).sort((a, b) => {
      const aIdx = staticTypes.indexOf(a);
      const bIdx = staticTypes.indexOf(b);
      if (aIdx !== -1 && bIdx !== -1) return aIdx - bIdx;
      if (aIdx !== -1) return -1;
      if (bIdx !== -1) return 1;
      return a.localeCompare(b);
    });
  }, [objectInfo]);

  const [displayName, setDisplayName] = useState(initialNode?.displayName || '');
  const [classType, setClassType] = useState(initialNode?.classType || '');
  const [category, setCategory] = useState(initialNode?.category || 'custom');
  const [description, setDescription] = useState(initialNode?.description || '');
  const [isOutputNode, setIsOutputNode] = useState(initialNode?.isOutputNode || false);
  const [inputs, setInputs] = useState<CustomNodeInputDef[]>(initialNode?.inputs || []);
  const [outputs, setOutputs] = useState<CustomNodeOutputDef[]>(initialNode?.outputs || []);
  const [executeCode, setExecuteCode] = useState(initialNode?.executeCode || '');
  const [expandedInputs, setExpandedInputs] = useState<Record<string, boolean>>({});
  const [errors, setErrors] = useState<string[]>([]);

  const handleDisplayNameChange = useCallback((value: string) => {
    setDisplayName(value);
    if (!isEditing && !classType) {
      setClassType(generateClassType(value));
    }
  }, [isEditing, classType]);

  const addInput = useCallback(() => {
    const newInput: CustomNodeInputDef = {
      name: `input_${inputs.length + 1}`,
      type: 'STRING',
      required: true,
      default: '',
    };
    setInputs([...inputs, newInput]);
  }, [inputs]);

  const updateInput = useCallback((index: number, updates: Partial<CustomNodeInputDef>) => {
    setInputs(inputs.map((inp, i) => (i === index ? { ...inp, ...updates } : inp)));
  }, [inputs]);

  const removeInput = useCallback((index: number) => {
    setInputs(inputs.filter((_, i) => i !== index));
  }, [inputs]);

  const addOutput = useCallback(() => {
    const newOutput: CustomNodeOutputDef = {
      name: `output_${outputs.length + 1}`,
      type: 'STRING',
    };
    setOutputs([...outputs, newOutput]);
  }, [outputs]);

  const updateOutput = useCallback((index: number, updates: Partial<CustomNodeOutputDef>) => {
    setOutputs(outputs.map((out, i) => (i === index ? { ...out, ...updates } : out)));
  }, [outputs]);

  const removeOutput = useCallback((index: number) => {
    setOutputs(outputs.filter((_, i) => i !== index));
  }, [outputs]);

  const toggleInputExpand = useCallback((name: string) => {
    setExpandedInputs((prev) => ({ ...prev, [name]: !prev[name] }));
  }, []);

  const validate = useCallback((): boolean => {
    const errs: string[] = [];
    if (!displayName.trim()) errs.push('Display name is required');
    if (!classType.trim()) errs.push('Class type is required');
    if (!category.trim()) errs.push('Category is required');

    const inputNames = new Set<string>();
    for (const inp of inputs) {
      if (!inp.name.trim()) errs.push('All inputs must have a name');
      if (inputNames.has(inp.name)) errs.push(`Duplicate input name: ${inp.name}`);
      inputNames.add(inp.name);
    }

    const outputNames = new Set<string>();
    for (const out of outputs) {
      if (!out.name.trim()) errs.push('All outputs must have a name');
      if (outputNames.has(out.name)) errs.push(`Duplicate output name: ${out.name}`);
      outputNames.add(out.name);
    }

    setErrors(errs);
    return errs.length === 0;
  }, [displayName, classType, category, inputs, outputs]);

  const handleSave = useCallback(() => {
    if (!validate()) return;

    const now = Date.now();
    const node: CustomNodeDef = {
      id: initialNode?.id || crypto.randomUUID(),
      classType: classType.trim(),
      displayName: displayName.trim(),
      category: category.trim(),
      description: description.trim(),
      inputs,
      outputs,
      isOutputNode,
      executeCode: executeCode.trim() || undefined,
      createdAt: initialNode?.createdAt || now,
      updatedAt: now,
    };

    onSave(node);
  }, [validate, initialNode, classType, displayName, category, description, inputs, outputs, isOutputNode, executeCode, onSave]);

  return (
    <div style={{
      position: 'fixed',
      top: 0,
      left: 0,
      right: 0,
      bottom: 0,
      background: 'rgba(0,0,0,0.6)',
      display: 'flex',
      alignItems: 'center',
      justifyContent: 'center',
      zIndex: 2000,
    }}>
      <div style={{
        background: '#1e1e2e',
        border: '1px solid #444',
        borderRadius: 8,
        width: 640,
        maxHeight: '90vh',
        display: 'flex',
        flexDirection: 'column',
        boxShadow: '0 8px 32px rgba(0,0,0,0.5)',
      }}>
        <div style={{
          padding: '12px 16px',
          borderBottom: '1px solid #333',
          display: 'flex',
          justifyContent: 'space-between',
          alignItems: 'center',
        }}>
          <span style={{ fontWeight: 600, fontSize: 14, color: '#e2e8f0' }}>
            {isEditing ? 'Edit Custom Node' : 'Create Custom Node'}
          </span>
          <button onClick={onCancel} style={closeBtnStyle}>✕</button>
        </div>

        <div style={{ flex: 1, overflowY: 'auto', padding: '12px 16px' }}>
          {errors.length > 0 && (
            <div style={{
              background: '#3a1a1a',
              border: '1px solid #5a2a2a',
              borderRadius: 4,
              padding: '8px 12px',
              marginBottom: 12,
            }}>
              {errors.map((err, i) => (
                <div key={i} style={{ color: '#fc8181', fontSize: 12 }}>{err}</div>
              ))}
            </div>
          )}

          <Section title="Basic Info">
            <FieldRow label="Display Name">
              <input
                type="text"
                value={displayName}
                onChange={(e) => handleDisplayNameChange(e.target.value)}
                placeholder="My Custom Node"
                style={inputStyle}
              />
            </FieldRow>
            <FieldRow label="Class Type">
              <input
                type="text"
                value={classType}
                onChange={(e) => setClassType(e.target.value)}
                placeholder="Custom_MyNode"
                style={inputStyle}
              />
            </FieldRow>
            <FieldRow label="Category">
              <input
                type="text"
                value={category}
                onChange={(e) => setCategory(e.target.value)}
                placeholder="custom"
                style={inputStyle}
              />
            </FieldRow>
            <FieldRow label="Description">
              <textarea
                value={description}
                onChange={(e) => setDescription(e.target.value)}
                placeholder="Node description..."
                style={{ ...inputStyle, minHeight: 60, resize: 'vertical' }}
              />
            </FieldRow>
            <FieldRow label="Output Node">
              <input
                type="checkbox"
                checked={isOutputNode}
                onChange={(e) => setIsOutputNode(e.target.checked)}
                style={{ cursor: 'pointer' }}
              />
              <span style={{ fontSize: 11, color: '#718096', marginLeft: 6 }}>
                Mark as output node (e.g., SaveImage)
              </span>
            </FieldRow>
          </Section>

          <Section title="Inputs">
            {inputs.map((inp, i) => (
              <InputEditor
                key={i}
                input={inp}
                expanded={expandedInputs[inp.name] || false}
                onToggle={() => toggleInputExpand(inp.name)}
                onUpdate={(updates) => updateInput(i, updates)}
                onRemove={() => removeInput(i)}
                allTypes={allTypes}
              />
            ))}
            <button onClick={addInput} style={addBtnStyle}>
              <Plus size={14} /> Add Input
            </button>
          </Section>

          <Section title="Outputs">
            {outputs.map((out, i) => (
              <div key={i} style={{
                display: 'flex',
                alignItems: 'center',
                gap: 8,
                padding: '4px 0',
              }}>
                <input
                  type="text"
                  value={out.name}
                  onChange={(e) => updateOutput(i, { name: e.target.value })}
                  placeholder="Output name"
                  style={{ ...inputStyle, flex: 1 }}
                />
                <TypeCombobox
                  value={out.type}
                  onChange={(t) => updateOutput(i, { type: t })}
                  allTypes={allTypes}
                  style={{ width: 140 }}
                />
                <button onClick={() => removeOutput(i)} style={removeBtnStyle}>
                  <Trash2 size={14} />
                </button>
              </div>
            ))}
            <button onClick={addOutput} style={addBtnStyle}>
              <Plus size={14} /> Add Output
            </button>
          </Section>

          <Section title="Execute Function (Optional)">
            <div style={{ fontSize: 11, color: '#718096', marginBottom: 6 }}>
              JavaScript function body. Receives inputs as params, must return an array matching output types.
            </div>
            <textarea
              value={executeCode}
              onChange={(e) => setExecuteCode(e.target.value)}
              placeholder={`// Example: return [inputs.input_1 + 1];\n// Available: inputs (object with input values)`}
              style={{
                ...inputStyle,
                minHeight: 100,
                resize: 'vertical',
                fontFamily: 'monospace',
                fontSize: 12,
                lineHeight: 1.5,
              }}
            />
          </Section>
        </div>

        <div style={{
          padding: '12px 16px',
          borderTop: '1px solid #333',
          display: 'flex',
          justifyContent: 'flex-end',
          gap: 8,
        }}>
          <button onClick={onCancel} style={cancelBtnStyle}>Cancel</button>
          <button onClick={handleSave} style={saveBtnStyle}>
            {isEditing ? 'Update' : 'Create'}
          </button>
        </div>
      </div>
    </div>
  );
};

interface TypeComboboxProps {
  value: IoType;
  onChange: (type: IoType) => void;
  allTypes: IoType[];
  style?: React.CSSProperties;
}

const TypeCombobox: FC<TypeComboboxProps> = ({ value, onChange, allTypes, style }) => {
  const [open, setOpen] = useState(false);
  const [filter, setFilter] = useState('');
  const triggerRef = useRef<HTMLDivElement>(null);
  const dropdownRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    if (!open) return;
    const handler = (e: MouseEvent) => {
      if (
        triggerRef.current && triggerRef.current.contains(e.target as Node)
      ) return;
      if (
        dropdownRef.current && dropdownRef.current.contains(e.target as Node)
      ) return;
      setOpen(false);
      setFilter('');
    };
    document.addEventListener('mousedown', handler);
    return () => document.removeEventListener('mousedown', handler);
  }, [open]);

  const filtered = filter
    ? allTypes.filter((t) => t.toLowerCase().includes(filter.toLowerCase()))
    : allTypes;

  const isCustom = value && !allTypes.includes(value);

  const [dropdownPos, setDropdownPos] = useState({ top: 0, left: 0, width: 0 });

  useEffect(() => {
    if (open && triggerRef.current) {
      const rect = triggerRef.current.getBoundingClientRect();
      setDropdownPos({
        top: rect.bottom + 2,
        left: rect.left,
        width: rect.width,
      });
    }
  }, [open]);

  const selectType = (t: IoType) => {
    onChange(t);
    setOpen(false);
    setFilter('');
  };

  return (
    <div ref={triggerRef} style={{ position: 'relative', ...style }}>
      <div
        style={{
          display: 'flex',
          alignItems: 'center',
          background: '#2a2a3e',
          border: open ? '1px solid #6a8abf' : '1px solid #444',
          borderRadius: 4,
          color: '#e2e8f0',
          fontSize: 12,
        }}
      >
        <input
          type="text"
          value={open ? filter : value}
          onChange={(e) => {
            setFilter(e.target.value);
            onChange(e.target.value as IoType);
          }}
          onFocus={() => {
            setOpen(true);
            setFilter('');
          }}
          placeholder="Type or select..."
          style={{
            background: 'transparent',
            border: 'none',
            color: '#e2e8f0',
            fontSize: 12,
            outline: 'none',
            padding: '4px 6px',
            width: '100%',
            boxSizing: 'border-box',
          }}
        />
        <div
          onClick={() => {
            if (open) {
              setOpen(false);
              setFilter('');
            } else {
              setOpen(true);
              setFilter('');
            }
          }}
          style={{ padding: '0 4px', cursor: 'pointer', display: 'flex', alignItems: 'center' }}
        >
          <ChevronDown size={12} style={{ opacity: 0.5 }} />
        </div>
      </div>

      {open && createPortal(
        <div
          ref={dropdownRef}
          style={{
            position: 'fixed',
            top: dropdownPos.top,
            left: dropdownPos.left,
            width: Math.max(dropdownPos.width, 160),
            zIndex: 9999,
            background: '#252538',
            border: '1px solid #444',
            borderRadius: 4,
            maxHeight: 200,
            overflowY: 'auto',
            boxShadow: '0 4px 12px rgba(0,0,0,0.5)',
          }}
        >
          {isCustom && value && !filter && (
            <div
              onMouseDown={(e) => {
                e.preventDefault();
                selectType(value);
              }}
              style={{
                padding: '5px 8px',
                fontSize: 11,
                cursor: 'pointer',
                color: '#8ab4f8',
                background: '#1a2a3e',
              }}
            >
              {value} (custom)
            </div>
          )}
          {filtered.map((t) => (
            <div
              key={t}
              onMouseDown={(e) => {
                e.preventDefault();
                selectType(t);
              }}
              style={{
                padding: '5px 8px',
                fontSize: 11,
                cursor: 'pointer',
                color: t === value ? '#8ab4f8' : '#e2e8f0',
                background: t === value ? '#1a2a3e' : 'transparent',
              }}
              onMouseEnter={(e) => { (e.target as HTMLElement).style.background = '#1a2a3e'; }}
              onMouseLeave={(e) => { (e.target as HTMLElement).style.background = t === value ? '#1a2a3e' : 'transparent'; }}
            >
              {t}
            </div>
          ))}
          {filtered.length === 0 && (
            <div style={{ padding: '5px 8px', fontSize: 11, color: '#718096' }}>
              No matches — type custom type name
            </div>
          )}
        </div>,
        document.body,
      )}
    </div>
  );
};

interface InputEditorProps {
  input: CustomNodeInputDef;
  expanded: boolean;
  onToggle: () => void;
  onUpdate: (updates: Partial<CustomNodeInputDef>) => void;
  onRemove: () => void;
  allTypes: IoType[];
}

const InputEditor: FC<InputEditorProps> = ({ input, expanded, onToggle, onUpdate, onRemove, allTypes }) => {
  const isCombo = input.type === 'COMBO' || (input.extra?.choices && input.extra.choices.length > 0);
  const [choicesText, setChoicesText] = useState(
    input.extra?.choices?.join(', ') || ''
  );

  const handleChoicesChange = (text: string) => {
    setChoicesText(text);
    const choices = text.split(',').map((s) => s.trim()).filter(Boolean);
    onUpdate({
      extra: { ...input.extra, choices },
    });
  };

  return (
    <div style={{
      border: '1px solid #333',
      borderRadius: 4,
      marginBottom: 6,
    }}>
      <div
        style={{
          display: 'flex',
          alignItems: 'center',
          gap: 6,
          padding: '6px 8px',
          background: '#252538',
          cursor: 'pointer',
        }}
        onClick={onToggle}
      >
        {expanded ? <ChevronDown size={14} /> : <ChevronRight size={14} />}
        <input
          type="text"
          value={input.name}
          onChange={(e) => onUpdate({ name: e.target.value })}
          onClick={(e) => e.stopPropagation()}
          style={{ ...inputStyle, flex: 1, background: 'transparent', border: 'none', padding: 0 }}
          placeholder="Input name"
        />
        <div onClick={(e) => e.stopPropagation()}>
          <TypeCombobox
            value={isCombo ? 'COMBO' : input.type}
            onChange={(t) => {
              if (t !== 'COMBO') {
                setChoicesText('');
                onUpdate({ type: t, extra: { ...input.extra, choices: undefined } });
              } else {
                onUpdate({ type: t });
              }
            }}
            allTypes={allTypes}
            style={{ width: 120 }}
          />
        </div>
        <label
          style={{ display: 'flex', alignItems: 'center', gap: 3, fontSize: 10, color: '#a0aec0' }}
          onClick={(e) => e.stopPropagation()}
        >
          <input
            type="checkbox"
            checked={input.required}
            onChange={(e) => onUpdate({ required: e.target.checked })}
          />
          Req
        </label>
        <button onClick={(e) => { e.stopPropagation(); onRemove(); }} style={removeBtnStyle}>
          <Trash2 size={12} />
        </button>
      </div>

      {expanded && (
        <div style={{ padding: '8px', display: 'flex', flexDirection: 'column', gap: 6 }}>
          {isCombo && (
            <FieldRow label="Choices (comma-separated)">
              <input
                type="text"
                value={choicesText}
                onChange={(e) => handleChoicesChange(e.target.value)}
                placeholder="option1, option2, option3"
                style={inputStyle}
              />
            </FieldRow>
          )}

          {(input.type === 'INT' || input.type === 'FLOAT') && (
            <>
              <FieldRow label="Default">
                <input
                  type="number"
                  value={input.default !== undefined ? Number(input.default) : (input.type === 'INT' ? 0 : 0.0)}
                  onChange={(e) => onUpdate({ default: input.type === 'INT' ? parseInt(e.target.value) || 0 : parseFloat(e.target.value) || 0 })}
                  style={{ ...inputStyle, width: 80 }}
                />
              </FieldRow>
              <div style={{ display: 'flex', gap: 8 }}>
                <FieldRow label="Min">
                  <input
                    type="number"
                    value={input.extra?.min ?? ''}
                    onChange={(e) => onUpdate({ extra: { ...input.extra, min: parseFloat(e.target.value) } })}
                    style={{ ...inputStyle, width: 70 }}
                  />
                </FieldRow>
                <FieldRow label="Max">
                  <input
                    type="number"
                    value={input.extra?.max ?? ''}
                    onChange={(e) => onUpdate({ extra: { ...input.extra, max: parseFloat(e.target.value) } })}
                    style={{ ...inputStyle, width: 70 }}
                  />
                </FieldRow>
                <FieldRow label="Step">
                  <input
                    type="number"
                    value={input.extra?.step ?? ''}
                    onChange={(e) => onUpdate({ extra: { ...input.extra, step: parseFloat(e.target.value) } })}
                    style={{ ...inputStyle, width: 70 }}
                  />
                </FieldRow>
              </div>
            </>
          )}

          {input.type === 'STRING' && (
            <>
              <FieldRow label="Default">
                <input
                  type="text"
                  value={String(input.default || '')}
                  onChange={(e) => onUpdate({ default: e.target.value })}
                  style={inputStyle}
                />
              </FieldRow>
              <FieldRow label="Multiline">
                <input
                  type="checkbox"
                  checked={input.extra?.multiline || false}
                  onChange={(e) => onUpdate({ extra: { ...input.extra, multiline: e.target.checked } })}
                  style={{ cursor: 'pointer' }}
                />
              </FieldRow>
            </>
          )}

          {input.type === 'BOOLEAN' && (
            <FieldRow label="Default">
              <input
                type="checkbox"
                checked={!!input.default}
                onChange={(e) => onUpdate({ default: e.target.checked })}
                style={{ cursor: 'pointer' }}
              />
            </FieldRow>
          )}

          <FieldRow label="Tooltip">
            <input
              type="text"
              value={input.extra?.tooltip || ''}
              onChange={(e) => onUpdate({ extra: { ...input.extra, tooltip: e.target.value } })}
              placeholder="Input tooltip"
              style={inputStyle}
            />
          </FieldRow>
        </div>
      )}
    </div>
  );
};

interface SectionProps {
  title: string;
  children: React.ReactNode;
}

const Section: FC<SectionProps> = ({ title, children }) => (
  <div style={{ marginBottom: 16 }}>
    <div style={{
      fontSize: 12,
      fontWeight: 600,
      color: '#a0aec0',
      textTransform: 'uppercase',
      letterSpacing: '0.05em',
      marginBottom: 8,
      paddingBottom: 4,
      borderBottom: '1px solid #333',
    }}>
      {title}
    </div>
    {children}
  </div>
);

interface FieldRowProps {
  label: string;
  children: React.ReactNode;
}

const FieldRow: FC<FieldRowProps> = ({ label, children }) => (
  <div style={{
    display: 'flex',
    alignItems: 'center',
    gap: 8,
    marginBottom: 6,
  }}>
    <span style={{
      fontSize: 11,
      color: '#a0aec0',
      minWidth: 100,
      flexShrink: 0,
    }}>
      {label}
    </span>
    <div style={{ flex: 1, display: 'flex', alignItems: 'center', gap: 6 }}>
      {children}
    </div>
  </div>
);

const inputStyle: React.CSSProperties = {
  background: '#2a2a3e',
  border: '1px solid #444',
  borderRadius: 4,
  color: '#e2e8f0',
  padding: '4px 8px',
  fontSize: 12,
  outline: 'none',
  width: '100%',
  boxSizing: 'border-box',
};

const addBtnStyle: React.CSSProperties = {
  display: 'flex',
  alignItems: 'center',
  gap: 4,
  background: '#2a3a4e',
  border: '1px dashed #4a6a8e',
  borderRadius: 4,
  color: '#8ab4f8',
  padding: '6px 12px',
  fontSize: 12,
  cursor: 'pointer',
  width: '100%',
  justifyContent: 'center',
  marginTop: 4,
};

const removeBtnStyle: React.CSSProperties = {
  background: 'transparent',
  border: 'none',
  color: '#718096',
  cursor: 'pointer',
  padding: 2,
  display: 'flex',
  alignItems: 'center',
};

const closeBtnStyle: React.CSSProperties = {
  background: 'transparent',
  border: 'none',
  color: '#718096',
  cursor: 'pointer',
  fontSize: 16,
  padding: '2px 6px',
};

const cancelBtnStyle: React.CSSProperties = {
  background: '#2a2a3e',
  border: '1px solid #444',
  borderRadius: 4,
  color: '#a0aec0',
  padding: '6px 16px',
  fontSize: 12,
  cursor: 'pointer',
};

const saveBtnStyle: React.CSSProperties = {
  background: '#4a6abf',
  border: '1px solid #5a7acf',
  borderRadius: 4,
  color: '#fff',
  padding: '6px 16px',
  fontSize: 12,
  cursor: 'pointer',
  fontWeight: 600,
};

export { CustomNodeEditor };
