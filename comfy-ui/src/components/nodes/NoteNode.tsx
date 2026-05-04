import { memo, type FC, useState, useCallback, useRef, useEffect } from 'react';
import { Handle, Position } from '@xyflow/react';

interface NoteNodeProps {
  id: string;
  data: {
    text: string;
    color: string;
  };
  selected: boolean;
}

const NOTE_COLORS = [
  '#5b8c5a',
  '#c78030',
  '#5a6abf',
  '#7a5bbf',
  '#bf5b7a',
  '#8b6bbf',
  '#2d6a4f',
  '#d4a017',
  '#4a90d9',
];

const NoteNodeComponent: FC<NoteNodeProps> = memo(({ id, data, selected }) => {
  const [editing, setEditing] = useState(false);
  const [text, setText] = useState(data.text || '');
  const [color, setColor] = useState(data.color || '#5b8c5a');
  const textareaRef = useRef<HTMLTextAreaElement>(null);
  const [showColorPicker, setShowColorPicker] = useState(false);

  useEffect(() => {
    setText(data.text || '');
    setColor(data.color || '#5b8c5a');
  }, [data.text, data.color]);

  useEffect(() => {
    if (editing && textareaRef.current) {
      textareaRef.current.focus();
      textareaRef.current.select();
    }
  }, [editing]);

  const handleDoubleClick = useCallback(() => {
    setEditing(true);
  }, []);

  const handleBlur = useCallback(() => {
    setEditing(false);
    const event = new CustomEvent('note-update', {
      detail: { id, text, color },
    });
    window.dispatchEvent(event);
  }, [id, text, color]);

  const handleColorChange = useCallback((newColor: string) => {
    setColor(newColor);
    setShowColorPicker(false);
    const event = new CustomEvent('note-update', {
      detail: { id, text, color: newColor },
    });
    window.dispatchEvent(event);
  }, [id, text]);

  return (
    <div
      style={{
        background: color,
        borderRadius: 6,
        border: selected ? '2px solid #fff' : '1px solid rgba(0,0,0,0.2)',
        minWidth: 180,
        maxWidth: 320,
        minHeight: 60,
        fontSize: 12,
        color: '#fff',
        boxShadow: selected
          ? '0 0 12px rgba(100, 150, 255, 0.4)'
          : '0 2px 8px rgba(0,0,0,0.3)',
        transition: 'border-color 0.3s, box-shadow 0.3s',
        display: 'flex',
        flexDirection: 'column',
      }}
      onDoubleClick={handleDoubleClick}
    >
      <div
        style={{
          display: 'flex',
          justifyContent: 'space-between',
          alignItems: 'center',
          padding: '4px 8px',
          cursor: 'pointer',
          userSelect: 'none',
          borderBottom: '1px solid rgba(255,255,255,0.15)',
        }}
      >
        <span style={{ fontSize: 10, fontWeight: 600, opacity: 0.8 }}>📝 Note</span>
        <div style={{ display: 'flex', alignItems: 'center', gap: 4 }}>
          <span
            onClick={(e) => {
              e.stopPropagation();
              setShowColorPicker(!showColorPicker);
            }}
            style={{
              fontSize: 9,
              cursor: 'pointer',
              opacity: 0.7,
              padding: '0 2px',
            }}
          >
            🎨
          </span>
          <span style={{ fontSize: 9, opacity: 0.5 }}>#{id}</span>
        </div>
      </div>

      {showColorPicker && (
        <div
          style={{
            display: 'flex',
            gap: 3,
            padding: '4px 8px',
            flexWrap: 'wrap',
          }}
        >
          {NOTE_COLORS.map((c) => (
            <div
              key={c}
              onClick={(e) => {
                e.stopPropagation();
                handleColorChange(c);
              }}
              style={{
                width: 14,
                height: 14,
                borderRadius: 3,
                background: c,
                cursor: 'pointer',
                border: c === color ? '2px solid #fff' : '1px solid rgba(255,255,255,0.3)',
              }}
            />
          ))}
        </div>
      )}

      {editing ? (
        <textarea
          ref={textareaRef}
          value={text}
          onChange={(e) => setText(e.target.value)}
          onBlur={handleBlur}
          rows={4}
          style={{
            background: 'rgba(0,0,0,0.2)',
            border: 'none',
            borderRadius: '0 0 4px 4px',
            color: '#fff',
            padding: '6px 8px',
            fontSize: 11,
            outline: 'none',
            resize: 'both',
            fontFamily: 'inherit',
            lineHeight: 1.5,
            width: '100%',
            boxSizing: 'border-box',
          }}
        />
      ) : (
        <div
          style={{
            padding: '6px 8px',
            fontSize: 11,
            lineHeight: 1.5,
            whiteSpace: 'pre-wrap',
            wordBreak: 'break-word',
            minHeight: 30,
            color: 'rgba(255,255,255,0.9)',
          }}
        >
          {text || 'Double-click to edit...'}
        </div>
      )}
    </div>
  );
});

NoteNodeComponent.displayName = 'NoteNode';

export { NoteNodeComponent };
