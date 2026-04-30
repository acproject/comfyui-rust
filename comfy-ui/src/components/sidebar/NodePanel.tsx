import { useState, useMemo, type FC } from 'react';
import { Search, ChevronRight, ChevronDown } from 'lucide-react';
import { useWorkflowStore } from '@/store/workflow';
import type { NodeClassDef } from '@/types/api';

interface CategorizedNodes {
  [category: string]: NodeClassDef[];
}

const NodePanel: FC = () => {
  const objectInfo = useWorkflowStore((s) => s.objectInfo);
  const addNode = useWorkflowStore((s) => s.addNode);
  const [search, setSearch] = useState('');
  const [collapsed, setCollapsed] = useState<Record<string, boolean>>({});

  const categorized = useMemo<CategorizedNodes>(() => {
    const result: CategorizedNodes = {};
    for (const [classType, def] of Object.entries(objectInfo)) {
      if (search) {
        const q = search.toLowerCase();
        const match =
          classType.toLowerCase().includes(q) ||
          def.display_name.toLowerCase().includes(q) ||
          def.category.toLowerCase().includes(q);
        if (!match) continue;
      }
      const cat = def.category || 'uncategorized';
      if (!result[cat]) result[cat] = [];
      result[cat].push(def);
    }
    return result;
  }, [objectInfo, search]);

  const toggleCategory = (cat: string) => {
    setCollapsed((prev) => ({ ...prev, [cat]: !prev[cat] }));
  };

  const handleDragStart = (e: React.DragEvent, classType: string) => {
    e.dataTransfer.setData('application/comfy-node', classType);
    e.dataTransfer.effectAllowed = 'move';
  };

  const handleDoubleClick = (classType: string) => {
    addNode(classType, { x: 200, y: 200 });
  };

  return (
    <div
      style={{
        background: '#1e1e2e',
        display: 'flex',
        flexDirection: 'column',
        height: '100%',
        color: '#e2e8f0',
      }}
    >
      <div style={{ padding: '8px 10px', borderBottom: '1px solid #333' }}>
        <div
          style={{
            display: 'flex',
            alignItems: 'center',
            gap: 8,
            background: '#2a2a3e',
            borderRadius: 6,
            padding: '5px 8px',
          }}
        >
          <Search size={14} style={{ color: '#718096' }} />
          <input
            type="text"
            placeholder="Search nodes..."
            value={search}
            onChange={(e) => setSearch(e.target.value)}
            style={{
              background: 'transparent',
              border: 'none',
              outline: 'none',
              color: '#e2e8f0',
              fontSize: 12,
              width: '100%',
            }}
          />
        </div>
      </div>

      <div style={{ flex: 1, overflowY: 'auto', padding: '4px 0' }}>
        {Object.entries(categorized).map(([category, nodes]) => {
          const isCollapsed = collapsed[category];
          return (
            <div key={category}>
              <div
                onClick={() => toggleCategory(category)}
                style={{
                  padding: '5px 10px',
                  fontSize: 11,
                  fontWeight: 600,
                  color: '#a0aec0',
                  textTransform: 'uppercase',
                  letterSpacing: '0.05em',
                  cursor: 'pointer',
                  display: 'flex',
                  alignItems: 'center',
                  gap: 4,
                  userSelect: 'none',
                }}
              >
                {isCollapsed ? <ChevronRight size={12} /> : <ChevronDown size={12} />}
                {category}
                <span style={{ fontSize: 9, color: '#555', marginLeft: 'auto' }}>
                  {nodes.length}
                </span>
              </div>
              {!isCollapsed &&
                nodes.map((def) => (
                  <div
                    key={def.class_type}
                    draggable
                    onDragStart={(e) => handleDragStart(e, def.class_type)}
                    onDoubleClick={() => handleDoubleClick(def.class_type)}
                    style={{
                      padding: '3px 10px 3px 24px',
                      fontSize: 11,
                      cursor: 'grab',
                      borderRadius: 3,
                      margin: '1px 6px',
                      transition: 'background 0.1s',
                    }}
                    onMouseEnter={(e) => {
                      (e.currentTarget as HTMLElement).style.background = '#2a2a3e';
                    }}
                    onMouseLeave={(e) => {
                      (e.currentTarget as HTMLElement).style.background = 'transparent';
                    }}
                  >
                    {def.display_name || def.class_type}
                  </div>
                ))}
            </div>
          );
        })}
      </div>
    </div>
  );
};

export { NodePanel };
