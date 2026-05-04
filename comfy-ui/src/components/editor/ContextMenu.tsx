import { memo, type FC, useState, useRef, useEffect, useCallback } from 'react';
import { useWorkflowStore } from '@/store/workflow';
import { isCustomNode } from '@/components/nodes/nodeColors';

interface ContextMenuState {
  x: number;
  y: number;
  type: 'canvas' | 'node';
  nodeId?: string;
}

interface ContextMenuProps {
  menu: ContextMenuState;
  onClose: () => void;
}

const ContextMenu: FC<ContextMenuProps> = memo(({ menu, onClose }) => {
  const { objectInfo, addNode, addNoteNode, removeNode, nodes, edges, setEdges } = useWorkflowStore();
  const [search, setSearch] = useState('');
  const ref = useRef<HTMLDivElement>(null);

  useEffect(() => {
    const handleClick = (e: MouseEvent) => {
      if (ref.current && !ref.current.contains(e.target as Node)) {
        onClose();
      }
    };
    const handleKey = (e: KeyboardEvent) => {
      if (e.key === 'Escape') onClose();
    };
    document.addEventListener('mousedown', handleClick);
    document.addEventListener('keydown', handleKey);
    return () => {
      document.removeEventListener('mousedown', handleClick);
      document.removeEventListener('keydown', handleKey);
    };
  }, [onClose]);

  const handleAddNode = useCallback(
    (classType: string) => {
      addNode(classType, { x: menu.x, y: menu.y });
      onClose();
    },
    [addNode, menu.x, menu.y, onClose]
  );

  const handleDeleteNode = useCallback(() => {
    if (menu.nodeId) {
      removeNode(menu.nodeId);
    }
    onClose();
  }, [menu.nodeId, removeNode, onClose]);

  const handleDisconnect = useCallback(() => {
    if (menu.nodeId) {
      const filtered = edges.filter(
        (e) => e.source !== menu.nodeId && e.target !== menu.nodeId
      );
      setEdges(filtered);
    }
    onClose();
  }, [menu.nodeId, edges, setEdges, onClose]);

  const handleClone = useCallback(() => {
    if (menu.nodeId) {
      const node = nodes.find((n) => n.id === menu.nodeId);
      if (node) {
        addNode(node.data.classType, {
          x: node.position.x + 30,
          y: node.position.y + 30,
        });
      }
    }
    onClose();
  }, [menu.nodeId, nodes, addNode, onClose]);

  const filteredNodes = Object.entries(objectInfo).filter(([classType, def]) => {
    if (!search) return true;
    const q = search.toLowerCase();
    return (
      classType.toLowerCase().includes(q) ||
      def.display_name.toLowerCase().includes(q) ||
      def.category.toLowerCase().includes(q)
    );
  });

  const categories: Record<string, Array<[string, typeof objectInfo[string]]>> = {};
  for (const [classType, def] of filteredNodes) {
    const cat = def.category || 'uncategorized';
    if (!categories[cat]) categories[cat] = [];
    categories[cat].push([classType, def]);
  }

  if (menu.type === 'node') {
    return (
      <div
        ref={ref}
        style={{
          position: 'fixed',
          left: menu.x,
          top: menu.y,
          background: '#1e1e2e',
          border: '1px solid #444',
          borderRadius: 6,
          padding: '4px 0',
          minWidth: 160,
          zIndex: 1000,
          boxShadow: '0 4px 12px rgba(0,0,0,0.5)',
          color: '#e2e8f0',
          fontSize: 12,
        }}
      >
        <MenuItem label="Delete" onClick={handleDeleteNode} shortcut="Del" />
        <MenuItem label="Clone" onClick={handleClone} shortcut="Ctrl+D" />
        <MenuItem label="Disconnect" onClick={handleDisconnect} />
      </div>
    );
  }

  return (
    <div
      ref={ref}
      style={{
        position: 'fixed',
        left: menu.x,
        top: menu.y,
        background: '#1e1e2e',
        border: '1px solid #444',
        borderRadius: 6,
        minWidth: 220,
        maxHeight: 400,
        zIndex: 1000,
        boxShadow: '0 4px 12px rgba(0,0,0,0.5)',
        color: '#e2e8f0',
        fontSize: 12,
        display: 'flex',
        flexDirection: 'column',
        overflow: 'hidden',
      }}
    >
      <div style={{ padding: '6px 8px', borderBottom: '1px solid #333' }}>
        <input
          type="text"
          placeholder="Search nodes..."
          value={search}
          onChange={(e) => setSearch(e.target.value)}
          autoFocus
          style={{
            width: '100%',
            background: '#2a2a3e',
            border: '1px solid #444',
            borderRadius: 4,
            color: '#e2e8f0',
            padding: '4px 8px',
            fontSize: 12,
            outline: 'none',
          }}
        />
      </div>
      <div
        onClick={() => {
          addNoteNode({ x: menu.x, y: menu.y });
          onClose();
        }}
        style={{
          padding: '6px 10px',
          cursor: 'pointer',
          display: 'flex',
          alignItems: 'center',
          gap: 6,
          borderBottom: '1px solid #333',
          transition: 'background 0.1s',
        }}
        onMouseEnter={(e) => {
          (e.currentTarget as HTMLElement).style.background = '#2a2a3e';
        }}
        onMouseLeave={(e) => {
          (e.currentTarget as HTMLElement).style.background = 'transparent';
        }}
      >
        <span style={{ fontSize: 12 }}>📝</span>
        <span>Add Note</span>
      </div>
      <div style={{ overflowY: 'auto', flex: 1 }}>
        {Object.entries(categories).map(([category, items]) => (
          <div key={category}>
            <div
              style={{
                padding: '4px 10px',
                fontSize: 10,
                color: '#718096',
                textTransform: 'uppercase',
                letterSpacing: '0.05em',
                fontWeight: 600,
              }}
            >
              {category}
            </div>
            {items.map(([classType, def]) => (
              <div
                key={classType}
                onClick={() => handleAddNode(classType)}
                style={{
                  padding: '4px 10px 4px 16px',
                  cursor: 'pointer',
                  transition: 'background 0.1s',
                  display: 'flex',
                  alignItems: 'center',
                  justifyContent: 'space-between',
                }}
                onMouseEnter={(e) => {
                  (e.currentTarget as HTMLElement).style.background = '#2a2a3e';
                }}
                onMouseLeave={(e) => {
                  (e.currentTarget as HTMLElement).style.background = 'transparent';
                }}
              >
                <span style={{ overflow: 'hidden', textOverflow: 'ellipsis', whiteSpace: 'nowrap' }}>
                  {def.display_name || classType}
                </span>
                {isCustomNode(classType) && (
                  <span style={{
                    fontSize: 8,
                    background: '#8b6bbf33',
                    color: '#8b6bbf',
                    padding: '1px 4px',
                    borderRadius: 2,
                    flexShrink: 0,
                    marginLeft: 4,
                  }}>
                    custom
                  </span>
                )}
              </div>
            ))}
          </div>
        ))}
      </div>
    </div>
  );
});

const MenuItem: FC<{ label: string; onClick: () => void; shortcut?: string }> = ({
  label,
  onClick,
  shortcut,
}) => (
  <div
    onClick={onClick}
    style={{
      padding: '6px 12px',
      cursor: 'pointer',
      display: 'flex',
      justifyContent: 'space-between',
      alignItems: 'center',
      transition: 'background 0.1s',
    }}
    onMouseEnter={(e) => {
      (e.currentTarget as HTMLElement).style.background = '#2a2a3e';
    }}
    onMouseLeave={(e) => {
      (e.currentTarget as HTMLElement).style.background = 'transparent';
    }}
  >
    <span>{label}</span>
    {shortcut && (
      <span style={{ fontSize: 10, color: '#718096', marginLeft: 16 }}>{shortcut}</span>
    )}
  </div>
);

export { ContextMenu };
export type { ContextMenuState };
