import { type FC } from 'react';
import { Play, Square, Trash2, Save, FolderOpen, PanelLeftClose, PanelLeftOpen, MessageSquare, MessageSquareOff } from 'lucide-react';
import { useWorkflowStore } from '@/store/workflow';
import { api } from '@/api/client';

interface ToolbarProps {
  showSidebar: boolean;
  showAgent: boolean;
  onToggleSidebar: () => void;
  onToggleAgent: () => void;
}

const Toolbar: FC<ToolbarProps> = ({ showSidebar, showAgent, onToggleSidebar, onToggleAgent }) => {
  const getPrompt = useWorkflowStore((s) => s.getPrompt);
  const clientId = useWorkflowStore((s) => s.clientId);
  const clearWorkflow = useWorkflowStore((s) => s.clearWorkflow);
  const executingPromptId = useWorkflowStore((s) => s.executingPromptId);
  const queueInfo = useWorkflowStore((s) => s.queueInfo);
  const loadWorkflowFromJson = useWorkflowStore((s) => s.loadWorkflowFromJson);
  const validateWorkflow = useWorkflowStore((s) => s.validateWorkflow);
  const validationErrors = useWorkflowStore((s) => s.validationErrors);

  const handleQueuePrompt = async () => {
    try {
      const result = validateWorkflow();
      if (!result.valid) {
        console.error('Workflow validation failed:', result.errors);
        return;
      }

      const prompt = getPrompt();
      const response = await api.submitPrompt({
        prompt: prompt as Record<string, import('@/types/api').NodeDefinition>,
        client_id: clientId,
      });
      console.log('Prompt queued:', response.prompt_id);
    } catch (err) {
      console.error('Failed to queue prompt:', err);
    }
  };

  const handleInterrupt = async () => {
    try {
      await api.interrupt();
    } catch (err) {
      console.error('Failed to interrupt:', err);
    }
  };

  const handleClear = () => {
    clearWorkflow();
  };

  const handleSave = () => {
    const prompt = getPrompt();
    const json = JSON.stringify(prompt, null, 2);
    const blob = new Blob([json], { type: 'application/json' });
    const url = URL.createObjectURL(blob);
    const a = document.createElement('a');
    a.href = url;
    a.download = 'workflow.json';
    a.click();
    URL.revokeObjectURL(url);
  };

  const handleLoad = () => {
    const input = document.createElement('input');
    input.type = 'file';
    input.accept = '.json';
    input.onchange = async (e) => {
      const file = (e.target as HTMLInputElement).files?.[0];
      if (!file) return;
      const text = await file.text();
      try {
        const workflow = JSON.parse(text);
        if (workflow.nodes && Array.isArray(workflow.nodes)) {
          loadWorkflowFromJson(workflow);
        }
      } catch {
        console.error('Invalid workflow JSON');
      }
    };
    input.click();
  };

  const isExecuting = !!executingPromptId;
  const runningCount = queueInfo?.queue_running?.length || 0;
  const pendingCount = queueInfo?.queue_pending?.length || 0;

  return (
    <div
      style={{
        height: 38,
        background: '#1e1e2e',
        borderBottom: '1px solid #333',
        display: 'flex',
        alignItems: 'center',
        padding: '0 12px',
        gap: 4,
        color: '#e2e8f0',
        fontSize: 12,
        position: 'relative',
      }}
    >
      <ToolbarButton
        icon={<Play size={14} />}
        label="Queue Prompt"
        onClick={handleQueuePrompt}
        disabled={isExecuting}
        accent
      />
      <ToolbarButton
        icon={<Square size={14} />}
        label="Interrupt"
        onClick={handleInterrupt}
        disabled={!isExecuting}
        danger
      />
      <div style={{ width: 1, height: 18, background: '#333', margin: '0 4px' }} />
      <ToolbarButton icon={<Save size={14} />} label="Save" onClick={handleSave} />
      <ToolbarButton icon={<FolderOpen size={14} />} label="Load" onClick={handleLoad} />
      <ToolbarButton icon={<Trash2 size={14} />} label="Clear" onClick={handleClear} />

      <div style={{ width: 1, height: 18, background: '#333', margin: '0 4px' }} />
      <ToolbarButton
        icon={showSidebar ? <PanelLeftClose size={14} /> : <PanelLeftOpen size={14} />}
        label={showSidebar ? 'Hide Sidebar' : 'Show Sidebar'}
        onClick={onToggleSidebar}
      />
      <ToolbarButton
        icon={showAgent ? <MessageSquareOff size={14} /> : <MessageSquare size={14} />}
        label={showAgent ? 'Hide Agent' : 'Show Agent'}
        onClick={onToggleAgent}
      />

      <div style={{ flex: 1 }} />

      {(runningCount > 0 || pendingCount > 0) && (
        <div style={{
          display: 'flex',
          alignItems: 'center',
          gap: 8,
          fontSize: 11,
          color: '#a0aec0',
        }}>
          {runningCount > 0 && (
            <span style={{ color: '#f6ad55' }}>
              ▶ {runningCount} running
            </span>
          )}
          {pendingCount > 0 && (
            <span style={{ color: '#718096' }}>
              ⏳ {pendingCount} pending
            </span>
          )}
        </div>
      )}

      <div style={{
        fontSize: 10,
        color: '#555',
        padding: '2px 8px',
        border: '1px solid #333',
        borderRadius: 4,
      }}>
        ComfyUI-Rust
      </div>

      {validationErrors.length > 0 && (
        <div style={{
          position: 'absolute',
          top: 38,
          left: 0,
          right: 0,
          background: '#2d1515',
          borderBottom: '1px solid #742a2a',
          padding: '6px 12px',
          fontSize: 11,
          color: '#fc8181',
          maxHeight: 120,
          overflowY: 'auto',
          zIndex: 100,
        }}>
          <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', marginBottom: 4 }}>
            <span style={{ fontWeight: 600 }}>
              Validation Errors ({validationErrors.length})
            </span>
            <button
              onClick={() => useWorkflowStore.getState().clearValidationErrors()}
              style={{
                background: 'transparent',
                border: 'none',
                color: '#fc8181',
                cursor: 'pointer',
                fontSize: 11,
                padding: '0 4px',
              }}
            >
              ✕
            </button>
          </div>
          {validationErrors.map((err, i) => (
            <div key={i} style={{ padding: '2px 0', borderBottom: i < validationErrors.length - 1 ? '1px solid #4a2020' : 'none' }}>
              <span style={{ fontWeight: 500 }}>{err.message}</span>
              {err.details && <span style={{ color: '#e0a0a0', marginLeft: 6 }}>- {err.details}</span>}
              {err.nodeId && <span style={{ color: '#c08080', marginLeft: 6 }}>(Node #{err.nodeId})</span>}
            </div>
          ))}
        </div>
      )}
    </div>
  );
};

interface ToolbarButtonProps {
  icon: React.ReactNode;
  label: string;
  onClick: () => void;
  disabled?: boolean;
  accent?: boolean;
  danger?: boolean;
}

const ToolbarButton: FC<ToolbarButtonProps> = ({ icon, label, onClick, disabled, accent, danger }) => (
  <button
    onClick={onClick}
    disabled={disabled}
    title={label}
    style={{
      display: 'flex',
      alignItems: 'center',
      gap: 4,
      padding: '4px 8px',
      background: accent ? '#5a6abf' : 'transparent',
      border: 'none',
      borderRadius: 4,
      color: disabled ? '#4a5568' : danger ? '#fc8181' : '#e2e8f0',
      cursor: disabled ? 'not-allowed' : 'pointer',
      fontSize: 11,
      transition: 'background 0.15s',
    }}
    onMouseEnter={(e) => {
      if (!disabled) (e.currentTarget as HTMLElement).style.background = accent ? '#4a5abf' : '#2a2a3e';
    }}
    onMouseLeave={(e) => {
      (e.currentTarget as HTMLElement).style.background = accent ? '#5a6abf' : 'transparent';
    }}
  >
    {icon}
    <span>{label}</span>
  </button>
);

export { Toolbar };
