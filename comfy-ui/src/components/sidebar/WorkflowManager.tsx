import { useState, useEffect } from 'react';
import { api } from '@/api/client';
import type { WorkflowListItem } from '@/types/api';

interface WorkflowManagerProps {
  onLoadWorkflow: (workflow: unknown) => void;
  getCurrentWorkflow: () => unknown;
}

export default function WorkflowManager({ onLoadWorkflow, getCurrentWorkflow }: WorkflowManagerProps) {
  const [workflows, setWorkflows] = useState<WorkflowListItem[]>([]);
  const [saveName, setSaveName] = useState('');
  const [loading, setLoading] = useState(false);

  useEffect(() => {
    loadWorkflows();
  }, []);

  async function loadWorkflows() {
    setLoading(true);
    try {
      const result = await api.listWorkflows();
      setWorkflows(result.workflows);
    } catch (err) {
      console.error('Failed to load workflows:', err);
    } finally {
      setLoading(false);
    }
  }

  async function handleSave() {
    if (!saveName.trim()) return;
    try {
      const workflow = getCurrentWorkflow();
      await api.saveWorkflow({ name: saveName.trim(), workflow });
      setSaveName('');
      await loadWorkflows();
    } catch (err) {
      console.error('Failed to save workflow:', err);
    }
  }

  async function handleLoad(name: string) {
    try {
      const workflow = await api.loadWorkflow(name);
      onLoadWorkflow(workflow);
    } catch (err) {
      console.error('Failed to load workflow:', err);
    }
  }

  return (
    <div style={{ padding: '8px' }}>
      <h3 style={{ margin: 0, fontSize: '14px', marginBottom: '8px' }}>Workflows</h3>

      <div style={{ display: 'flex', gap: '4px', marginBottom: '8px' }}>
        <input
          type="text"
          value={saveName}
          onChange={(e) => setSaveName(e.target.value)}
          placeholder="Workflow name"
          style={{ flex: 1, fontSize: '12px', padding: '4px', background: '#1a1a1a', border: '1px solid #333', borderRadius: '4px', color: '#fff' }}
          onKeyDown={(e) => e.key === 'Enter' && handleSave()}
        />
        <button
          onClick={handleSave}
          disabled={!saveName.trim()}
          style={{ fontSize: '12px', padding: '4px 8px', whiteSpace: 'nowrap' }}
        >
          Save
        </button>
      </div>

      {loading && <div style={{ fontSize: '12px', color: '#888' }}>Loading...</div>}

      <div style={{ maxHeight: '200px', overflowY: 'auto' }}>
        {workflows.map((wf) => (
          <div
            key={wf.name}
            style={{
              display: 'flex',
              justifyContent: 'space-between',
              alignItems: 'center',
              padding: '4px 8px',
              borderBottom: '1px solid #222',
              cursor: 'pointer',
            }}
            onClick={() => handleLoad(wf.name)}
          >
            <span style={{ fontSize: '12px' }}>{wf.name}</span>
            <span style={{ fontSize: '10px', color: '#666' }}>
              {wf.modified ? new Date(wf.modified * 1000).toLocaleDateString() : ''}
            </span>
          </div>
        ))}
      </div>

      {workflows.length === 0 && !loading && (
        <div style={{ fontSize: '12px', color: '#666', textAlign: 'center', padding: '16px' }}>
          No saved workflows
        </div>
      )}
    </div>
  );
}
