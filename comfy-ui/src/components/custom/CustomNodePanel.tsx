import { useState, useRef, useCallback, type FC } from 'react';
import { Plus, Edit3, Trash2, Copy, Download, Upload } from 'lucide-react';
import { useCustomNodesStore } from '@/store/customNodes';
import { useWorkflowStore } from '@/store/workflow';
import { CustomNodeEditor } from '@/components/custom/CustomNodeEditor';
import type { CustomNodeDef } from '@/types/customNode';

const CustomNodePanel: FC = () => {
  const customNodes = useCustomNodesStore((s) => s.customNodes);
  const addCustomNode = useCustomNodesStore((s) => s.addCustomNode);
  const updateCustomNode = useCustomNodesStore((s) => s.updateCustomNode);
  const removeCustomNode = useCustomNodesStore((s) => s.removeCustomNode);
  const duplicateCustomNode = useCustomNodesStore((s) => s.duplicateCustomNode);
  const importCustomNodes = useCustomNodesStore((s) => s.importCustomNodes);
  const exportCustomNodes = useCustomNodesStore((s) => s.exportCustomNodes);
  const setObjectInfo = useWorkflowStore((s) => s.setObjectInfo);
  const objectInfo = useWorkflowStore((s) => s.objectInfo);

  const [editingNode, setEditingNode] = useState<CustomNodeDef | null>(null);
  const [showEditor, setShowEditor] = useState(false);
  const [confirmDelete, setConfirmDelete] = useState<string | null>(null);
  const fileInputRef = useRef<HTMLInputElement>(null);

  const refreshObjectInfo = useCallback(() => {
    const store = useCustomNodesStore.getState();
    const merged = store.mergeWithObjectInfo(objectInfo);
    setObjectInfo(merged);
  }, [objectInfo, setObjectInfo]);

  const handleCreate = useCallback(() => {
    setEditingNode(null);
    setShowEditor(true);
  }, []);

  const handleEdit = useCallback((node: CustomNodeDef) => {
    setEditingNode(node);
    setShowEditor(true);
  }, []);

  const handleSave = useCallback((node: CustomNodeDef) => {
    if (editingNode) {
      updateCustomNode(node.id, node);
    } else {
      addCustomNode(node);
    }
    setShowEditor(false);
    setEditingNode(null);
    setTimeout(refreshObjectInfo, 0);
  }, [editingNode, addCustomNode, updateCustomNode, refreshObjectInfo]);

  const handleDelete = useCallback((id: string) => {
    removeCustomNode(id);
    setConfirmDelete(null);
    setTimeout(refreshObjectInfo, 0);
  }, [removeCustomNode, refreshObjectInfo]);

  const handleDuplicate = useCallback((id: string) => {
    duplicateCustomNode(id);
    setTimeout(refreshObjectInfo, 0);
  }, [duplicateCustomNode, refreshObjectInfo]);

  const handleExport = useCallback(() => {
    const json = exportCustomNodes();
    const blob = new Blob([json], { type: 'application/json' });
    const url = URL.createObjectURL(blob);
    const a = document.createElement('a');
    a.href = url;
    a.download = 'custom_nodes.json';
    a.click();
    URL.revokeObjectURL(url);
  }, [exportCustomNodes]);

  const handleImport = useCallback((e: React.ChangeEvent<HTMLInputElement>) => {
    const file = e.target.files?.[0];
    if (!file) return;

    const reader = new FileReader();
    reader.onload = (ev) => {
      try {
        const nodes = JSON.parse(ev.target?.result as string) as CustomNodeDef[];
        if (Array.isArray(nodes)) {
          importCustomNodes(nodes);
          setTimeout(refreshObjectInfo, 0);
        }
      } catch {
        console.error('Failed to import custom nodes');
      }
    };
    reader.readAsText(file);
    e.target.value = '';
  }, [importCustomNodes, refreshObjectInfo]);

  const handleCancel = useCallback(() => {
    setShowEditor(false);
    setEditingNode(null);
  }, []);

  return (
    <div style={{
      background: '#1e1e2e',
      display: 'flex',
      flexDirection: 'column',
      height: '100%',
      color: '#e2e8f0',
    }}>
      <div style={{
        padding: '8px 10px',
        borderBottom: '1px solid #333',
        display: 'flex',
        justifyContent: 'space-between',
        alignItems: 'center',
      }}>
        <span style={{ fontSize: 12, fontWeight: 600, color: '#a0aec0' }}>
          Custom Nodes ({customNodes.length})
        </span>
        <div style={{ display: 'flex', gap: 4 }}>
          <button onClick={handleExport} title="Export" style={iconBtnStyle}>
            <Download size={14} />
          </button>
          <button onClick={() => fileInputRef.current?.click()} title="Import" style={iconBtnStyle}>
            <Upload size={14} />
          </button>
          <input
            ref={fileInputRef}
            type="file"
            accept=".json"
            style={{ display: 'none' }}
            onChange={handleImport}
          />
        </div>
      </div>

      <div style={{ flex: 1, overflowY: 'auto', padding: '4px 0' }}>
        {customNodes.length === 0 && (
          <div style={{
            padding: '20px 16px',
            textAlign: 'center',
            color: '#718096',
            fontSize: 12,
          }}>
            No custom nodes yet.
            <br />
            Click the button below to create one.
          </div>
        )}

        {customNodes.map((node) => (
          <div
            key={node.id}
            style={{
              padding: '6px 10px',
              borderBottom: '1px solid #2a2a3e',
              display: 'flex',
              flexDirection: 'column',
              gap: 4,
            }}
          >
            <div style={{
              display: 'flex',
              alignItems: 'center',
              justifyContent: 'space-between',
            }}>
              <div style={{ flex: 1, minWidth: 0 }}>
                <div style={{
                  fontSize: 12,
                  fontWeight: 500,
                  overflow: 'hidden',
                  textOverflow: 'ellipsis',
                  whiteSpace: 'nowrap',
                }}>
                  {node.displayName}
                </div>
                <div style={{
                  fontSize: 10,
                  color: '#718096',
                  overflow: 'hidden',
                  textOverflow: 'ellipsis',
                  whiteSpace: 'nowrap',
                }}>
                  {node.classType} · {node.category}
                </div>
              </div>
              <div style={{ display: 'flex', gap: 2, flexShrink: 0 }}>
                <button onClick={() => handleEdit(node)} title="Edit" style={iconBtnStyle}>
                  <Edit3 size={12} />
                </button>
                <button onClick={() => handleDuplicate(node.id)} title="Duplicate" style={iconBtnStyle}>
                  <Copy size={12} />
                </button>
                {confirmDelete === node.id ? (
                  <button
                    onClick={() => handleDelete(node.id)}
                    style={{ ...iconBtnStyle, color: '#fc8181' }}
                    title="Confirm delete"
                  >
                    <Trash2 size={12} />
                  </button>
                ) : (
                  <button
                    onClick={() => setConfirmDelete(node.id)}
                    style={iconBtnStyle}
                    title="Delete"
                  >
                    <Trash2 size={12} />
                  </button>
                )}
              </div>
            </div>

            <div style={{
              display: 'flex',
              gap: 6,
              fontSize: 10,
              color: '#555',
            }}>
              <span>{node.inputs.length} in</span>
              <span>·</span>
              <span>{node.outputs.length} out</span>
              {node.isOutputNode && (
                <>
                  <span>·</span>
                  <span style={{ color: '#bf5b7a' }}>output</span>
                </>
              )}
            </div>

            {node.description && (
              <div style={{
                fontSize: 10,
                color: '#555',
                overflow: 'hidden',
                textOverflow: 'ellipsis',
                whiteSpace: 'nowrap',
              }}>
                {node.description}
              </div>
            )}
          </div>
        ))}
      </div>

      <div style={{ padding: '8px 10px', borderTop: '1px solid #333' }}>
        <button onClick={handleCreate} style={createBtnStyle}>
          <Plus size={14} /> Create Custom Node
        </button>
      </div>

      {showEditor && (
        <CustomNodeEditor
          initialNode={editingNode}
          onSave={handleSave}
          onCancel={handleCancel}
        />
      )}
    </div>
  );
};

const iconBtnStyle: React.CSSProperties = {
  background: 'transparent',
  border: 'none',
  color: '#718096',
  cursor: 'pointer',
  padding: '3px',
  display: 'flex',
  alignItems: 'center',
  borderRadius: 3,
  transition: 'color 0.1s, background 0.1s',
};

const createBtnStyle: React.CSSProperties = {
  display: 'flex',
  alignItems: 'center',
  justifyContent: 'center',
  gap: 6,
  width: '100%',
  background: '#4a6abf',
  border: '1px solid #5a7acf',
  borderRadius: 6,
  color: '#fff',
  padding: '8px 16px',
  fontSize: 12,
  cursor: 'pointer',
  fontWeight: 600,
};

export { CustomNodePanel };
