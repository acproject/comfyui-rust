import { useState, useRef, useEffect, useCallback, type FC } from 'react';
import { Send, Bot, User, Settings, Trash2, CheckCircle, XCircle, Loader2, Workflow, Sparkles, BookOpen } from 'lucide-react';
import { useWorkflowStore } from '@/store/workflow';
import { api } from '@/api/client';
import type {
  AgentConfig,
  AgentChatMessage,
  AgentAction,
  AgentChatResponse,
  AgentChatContext,
  AgentModelInfo,
  WorkflowTemplate,
  ModelKnowledgeEntry,
} from '@/types/api';

interface DisplayMessage {
  id: string;
  role: 'user' | 'assistant' | 'system';
  content: string;
  timestamp: number;
  actions?: AgentAction[];
  actionResults?: Array<{ action: AgentAction; success: boolean; message: string }>;
}

const AIAgent: FC = () => {
  const [messages, setMessages] = useState<DisplayMessage[]>([]);
  const [input, setInput] = useState('');
  const [isProcessing, setIsProcessing] = useState(false);
  const [showSettings, setShowSettings] = useState(false);
  const [agentConfig, setAgentConfig] = useState<AgentConfig | null>(null);
  const [configForm, setConfigForm] = useState<AgentConfig>({
    enabled: false,
    provider: 'openai',
    api_url: 'https://api.openai.com/v1',
    api_key: null,
    model: 'gpt-4o-mini',
    max_tokens: 2048,
    temperature: 0.7,
    system_prompt: '',
  });
  const [availableModels, setAvailableModels] = useState<AgentModelInfo[]>([]);
  const [isLoadingModels, setIsLoadingModels] = useState(false);
  const [modelInputMode, setModelInputMode] = useState<'select' | 'manual'>('select');
  const [workflowTemplates, setWorkflowTemplates] = useState<WorkflowTemplate[]>([]);
  const [modelRecommendations, setModelRecommendations] = useState<ModelKnowledgeEntry[]>([]);
  const [showTemplates, setShowTemplates] = useState(false);
  const [showRecommendations, setShowRecommendations] = useState(false);
  const messagesEndRef = useRef<HTMLDivElement>(null);

  const addNode = useWorkflowStore((s) => s.addNode);
  const connectNodes = useWorkflowStore((s) => s.connectNodes);
  const updateNodeInput = useWorkflowStore((s) => s.updateNodeInput);
  const getPrompt = useWorkflowStore((s) => s.getPrompt);
  const clientId = useWorkflowStore((s) => s.clientId);
  const nodes = useWorkflowStore((s) => s.nodes);
  const edges = useWorkflowStore((s) => s.edges);
  const objectInfo = useWorkflowStore((s) => s.objectInfo);
  const clearWorkflow = useWorkflowStore((s) => s.clearWorkflow);
  const validateWorkflow = useWorkflowStore((s) => s.validateWorkflow);
  const loadWorkflowFromJson = useWorkflowStore((s) => s.loadWorkflowFromJson);

  useEffect(() => {
    messagesEndRef.current?.scrollIntoView({ behavior: 'smooth' });
  }, [messages]);

  useEffect(() => {
    api.getAgentConfig().then((cfg) => {
      setAgentConfig(cfg);
      setConfigForm(cfg);
    }).catch(() => {});
  }, []);

  useEffect(() => {
    api.getWorkflowTemplates().then((res) => {
      setWorkflowTemplates(res.templates);
    }).catch(() => {});
    api.getModelKnowledge().then((res) => {
      setModelRecommendations(res.models);
    }).catch(() => {});
  }, []);

  const buildContext = useCallback((): AgentChatContext => {
    const availableNodes = Object.keys(objectInfo);
    const currentWorkflowNodes = nodes.map((n) => ({
      id: n.id,
      class_type: n.data.classType,
      title: n.data.title,
      inputs: n.data.inputs as Record<string, unknown>,
      outputs: n.data.outputs.map((o) => ({ name: o.name, type_name: o.type })),
    }));
    const currentWorkflowEdges = edges.map((e) => ({
      source: e.source,
      source_handle: e.sourceHandle || '',
      target: e.target,
      target_handle: e.targetHandle || '',
    }));
    return { available_nodes: availableNodes, current_workflow_nodes: currentWorkflowNodes, current_workflow_edges: currentWorkflowEdges };
  }, [objectInfo, nodes, edges]);

  const executeAction = useCallback(
    (action: AgentAction): { success: boolean; message: string } => {
      switch (action.type) {
        case 'add_node': {
          const p = action.payload as { classType: string; x?: number; y?: number };
          if (!p.classType) return { success: false, message: 'Missing classType' };
          if (!objectInfo[p.classType]) return { success: false, message: `Unknown node type: ${p.classType}` };
          addNode(p.classType, {
            x: p.x ?? 300 + Math.random() * 200,
            y: p.y ?? 200 + Math.random() * 200,
          });
          return { success: true, message: `Added ${p.classType}` };
        }
        case 'connect': {
          const p = action.payload as { sourceId: string; sourceHandle: string; targetId: string; targetHandle: string };
          const err = connectNodes(p.sourceId, p.sourceHandle, p.targetId, p.targetHandle);
          if (err) return { success: false, message: `Connection failed: ${err.message}` };
          return { success: true, message: `Connected ${p.sourceId}.${p.sourceHandle} -> ${p.targetId}.${p.targetHandle}` };
        }
        case 'set_param': {
          const p = action.payload as { nodeId: string; inputName: string; value: unknown };
          const node = nodes.find((n) => n.id === p.nodeId);
          if (!node) return { success: false, message: `Node ${p.nodeId} not found` };
          updateNodeInput(p.nodeId, p.inputName, p.value);
          return { success: true, message: `Set ${p.nodeId}.${p.inputName} = ${JSON.stringify(p.value)}` };
        }
        case 'load_workflow_template': {
          const p = action.payload as { templateId: string };
          if (!p.templateId) return { success: false, message: 'Missing templateId' };
          const template = workflowTemplates.find((t) => t.id === p.templateId);
          if (!template) {
            api.getWorkflowTemplate(p.templateId).then((t) => {
              const workflow = templateToWorkflow(t);
              loadWorkflowFromJson(workflow);
            }).catch(() => {});
            return { success: true, message: `Loading template: ${p.templateId}` };
          }
          const workflow = templateToWorkflow(template);
          loadWorkflowFromJson(workflow);
          return { success: true, message: `Loaded template: ${template.name}` };
        }
        case 'recommend_model': {
          const p = action.payload as { modelName: string; reason?: string };
          return { success: true, message: `Recommended model: ${p.modelName}${p.reason ? ` — ${p.reason}` : ''}` };
        }
        case 'run_workflow': {
          const prompt = getPrompt();
          api.submitPrompt({
            prompt: prompt as Record<string, import('@/types/api').NodeDefinition>,
            client_id: clientId,
          });
          return { success: true, message: 'Workflow submitted to queue' };
        }
        case 'validate_workflow': {
          const result = validateWorkflow();
          if (result.valid) return { success: true, message: 'Workflow is valid!' };
          return { success: false, message: `Validation errors: ${result.errors.map((e) => e.message).join('; ')}` };
        }
        case 'clear_workflow': {
          clearWorkflow();
          return { success: true, message: 'Workflow cleared' };
        }
        default:
          return { success: false, message: `Unknown action: ${(action as AgentAction & { type: string }).type}` };
      }
    },
    [addNode, connectNodes, updateNodeInput, getPrompt, clientId, nodes, objectInfo, validateWorkflow, clearWorkflow, loadWorkflowFromJson, workflowTemplates],
  );

  const stripActionBlocks = (text: string): string => {
    return text.replace(/```action\n[\s\S]*?```/g, '').trim();
  };

  const handleSend = async () => {
    const text = input.trim();
    if (!text || isProcessing) return;

    setInput('');
    const userMsg: DisplayMessage = {
      id: crypto.randomUUID(),
      role: 'user',
      content: text,
      timestamp: Date.now(),
    };
    setMessages((prev) => [...prev, userMsg]);
    setIsProcessing(true);

    try {
      if (agentConfig?.enabled) {
        const chatMessages: AgentChatMessage[] = messages
          .filter((m) => m.role !== 'system')
          .map((m) => ({ role: m.role as 'user' | 'assistant', content: m.content }));
        chatMessages.push({ role: 'user', content: text });

        const context = buildContext();
        const response: AgentChatResponse = await api.agentChat({
          messages: chatMessages,
          context,
        });

        const displayContent = stripActionBlocks(response.message.content);
        const actionResults = response.actions.map((action) => {
          const result = executeAction(action);
          return { action, ...result };
        });

        const assistantMsg: DisplayMessage = {
          id: crypto.randomUUID(),
          role: 'assistant',
          content: displayContent || (actionResults.length > 0 ? 'Actions executed.' : 'No response.'),
          timestamp: Date.now(),
          actions: response.actions,
          actionResults,
        };
        setMessages((prev) => [...prev, assistantMsg]);
      } else {
        const response = await processLocalMessage(text);
        const actions = parseActions(response);
        const actionResults = actions.map((action) => {
          const result = executeAction(action);
          return { action, ...result };
        });
        const displayContent = stripActionBlocks(response);
        const assistantMsg: DisplayMessage = {
          id: crypto.randomUUID(),
          role: 'assistant',
          content: displayContent,
          timestamp: Date.now(),
          actions,
          actionResults,
        };
        setMessages((prev) => [...prev, assistantMsg]);
      }
    } catch (err) {
      const assistantMsg: DisplayMessage = {
        id: crypto.randomUUID(),
        role: 'assistant',
        content: `Error: ${err instanceof Error ? err.message : 'Unknown error'}`,
        timestamp: Date.now(),
      };
      setMessages((prev) => [...prev, assistantMsg]);
    } finally {
      setIsProcessing(false);
    }
  };

  const handleKeyDown = (e: React.KeyboardEvent) => {
    if (e.key === 'Enter' && !e.shiftKey) {
      e.preventDefault();
      handleSend();
    }
  };

  const handleSaveConfig = async () => {
    try {
      const configToSave = { ...configForm };
      if (configToSave.api_key && configToSave.api_key !== '********' && !configToSave.enabled) {
        configToSave.enabled = true;
      }
      const updated = await api.setAgentConfig(configToSave);
      setAgentConfig(updated);
      setConfigForm({ ...updated, api_key: configToSave.api_key });
      setShowSettings(false);
    } catch (err) {
      console.error('Failed to save agent config:', err);
    }
  };

  const fetchModels = async () => {
    setIsLoadingModels(true);
    try {
      const response = await api.getAgentModels();
      setAvailableModels(response.models);
      if (response.models.length > 0) {
        setModelInputMode('select');
        if (!configForm.enabled && configForm.api_key) {
          setConfigForm((prev) => ({ ...prev, enabled: true }));
        }
      }
    } catch {
      setAvailableModels([]);
    } finally {
      setIsLoadingModels(false);
    }
  };

  const handleClearChat = () => {
    setMessages([]);
  };

  const renderMessageContent = (content: string) => {
    const parts = content.split(/(\*\*.*?\*\*|\n)/g);
    return parts.map((part, i) => {
      if (part.startsWith('**') && part.endsWith('**')) {
        return <strong key={i}>{part.slice(2, -2)}</strong>;
      }
      if (part === '\n') {
        return <br key={i} />;
      }
      return <span key={i}>{part}</span>;
    });
  };

  if (showSettings) {
    return (
      <div
        style={{
          width: 340,
          background: '#1a202c',
          borderLeft: '1px solid #2d3748',
          display: 'flex',
          flexDirection: 'column',
          height: '100%',
          color: '#e2e8f0',
        }}
      >
        <div
          style={{
            padding: '8px 12px',
            borderBottom: '1px solid #2d3748',
            fontWeight: 600,
            fontSize: 13,
            display: 'flex',
            alignItems: 'center',
            gap: 6,
          }}
        >
          <Settings size={16} style={{ color: '#5a6abf' }} />
          Agent Settings
        </div>

        <div style={{ flex: 1, overflowY: 'auto', padding: 12, fontSize: 12 }}>
          <label style={{ display: 'flex', alignItems: 'center', gap: 8, marginBottom: 12 }}>
            <input
              type="checkbox"
              checked={configForm.enabled}
              onChange={(e) => setConfigForm({ ...configForm, enabled: e.target.checked })}
            />
            Enable AI Agent (LLM)
          </label>

          <div style={{ marginBottom: 10 }}>
            <div style={{ marginBottom: 4, color: '#a0aec0' }}>Provider</div>
            <select
              value={configForm.provider}
              onChange={(e) => {
                const provider = e.target.value;
                let apiUrl = configForm.api_url;
                let model = configForm.model;
                if (provider === 'openai') {
                  apiUrl = 'https://api.openai.com/v1';
                  model = 'gpt-4o-mini';
                } else if (provider === 'ollama') {
                  apiUrl = 'http://localhost:11434/v1';
                  model = 'llama3';
                } else if (provider === 'deepseek') {
                  apiUrl = 'https://api.deepseek.com/v1';
                  model = 'deepseek-chat';
                }
                setConfigForm({ ...configForm, provider, api_url: apiUrl, model });
              }}
              style={{
                width: '100%',
                background: '#2d3748',
                border: '1px solid #4a5568',
                borderRadius: 4,
                color: '#e2e8f0',
                padding: '4px 8px',
                fontSize: 12,
              }}
            >
              <option value="openai">OpenAI</option>
              <option value="ollama">Ollama (Local)</option>
              <option value="deepseek">DeepSeek</option>
              <option value="custom">Custom (OpenAI-compatible)</option>
            </select>
          </div>

          <div style={{ marginBottom: 10 }}>
            <div style={{ marginBottom: 4, color: '#a0aec0' }}>API URL</div>
            <input
              type="text"
              value={configForm.api_url}
              onChange={(e) => setConfigForm({ ...configForm, api_url: e.target.value })}
              style={{
                width: '100%',
                background: '#2d3748',
                border: '1px solid #4a5568',
                borderRadius: 4,
                color: '#e2e8f0',
                padding: '4px 8px',
                fontSize: 12,
                boxSizing: 'border-box',
              }}
            />
          </div>

          <div style={{ marginBottom: 10 }}>
            <div style={{ marginBottom: 4, color: '#a0aec0' }}>API Key</div>
            <input
              type="password"
              value={configForm.api_key || ''}
              onChange={(e) => setConfigForm({ ...configForm, api_key: e.target.value || null })}
              placeholder={configForm.provider === 'ollama' ? 'Not required for Ollama' : 'Enter API key'}
              style={{
                width: '100%',
                background: '#2d3748',
                border: '1px solid #4a5568',
                borderRadius: 4,
                color: '#e2e8f0',
                padding: '4px 8px',
                fontSize: 12,
                boxSizing: 'border-box',
              }}
            />
          </div>

          <div style={{ marginBottom: 10 }}>
            <div style={{ marginBottom: 4, color: '#a0aec0', display: 'flex', justifyContent: 'space-between', alignItems: 'center' }}>
              <span>Model</span>
              <div style={{ display: 'flex', gap: 4, alignItems: 'center' }}>
                <button
                  type="button"
                  onClick={fetchModels}
                  disabled={isLoadingModels}
                  style={{
                    background: '#4a5568',
                    border: 'none',
                    borderRadius: 3,
                    color: '#e2e8f0',
                    padding: '1px 6px',
                    fontSize: 10,
                    cursor: isLoadingModels ? 'wait' : 'pointer',
                  }}
                >
                  {isLoadingModels ? '...' : '↻'}
                </button>
                <button
                  type="button"
                  onClick={() => setModelInputMode(modelInputMode === 'select' ? 'manual' : 'select')}
                  style={{
                    background: '#4a5568',
                    border: 'none',
                    borderRadius: 3,
                    color: '#e2e8f0',
                    padding: '1px 6px',
                    fontSize: 10,
                    cursor: 'pointer',
                  }}
                >
                  {modelInputMode === 'select' ? '✎' : '☰'}
                </button>
              </div>
            </div>
            {modelInputMode === 'select' && availableModels.length > 0 ? (
              <select
                value={configForm.model}
                onChange={(e) => setConfigForm({ ...configForm, model: e.target.value })}
                style={{
                  width: '100%',
                  background: '#2d3748',
                  border: '1px solid #4a5568',
                  borderRadius: 4,
                  color: '#e2e8f0',
                  padding: '4px 8px',
                  fontSize: 12,
                }}
              >
                {availableModels.map((m) => (
                  <option key={m.id} value={m.id}>
                    {m.id}{m.owned_by ? ` (${m.owned_by})` : ''}
                  </option>
                ))}
              </select>
            ) : (
              <input
                type="text"
                value={configForm.model}
                onChange={(e) => setConfigForm({ ...configForm, model: e.target.value })}
                placeholder="e.g. gpt-4o-mini"
                style={{
                  width: '100%',
                  background: '#2d3748',
                  border: '1px solid #4a5568',
                  borderRadius: 4,
                  color: '#e2e8f0',
                  padding: '4px 8px',
                  fontSize: 12,
                  boxSizing: 'border-box',
                }}
              />
            )}
          </div>

          <div style={{ marginBottom: 10 }}>
            <div style={{ marginBottom: 4, color: '#a0aec0' }}>Max Tokens</div>
            <input
              type="number"
              value={configForm.max_tokens}
              onChange={(e) => setConfigForm({ ...configForm, max_tokens: Number(e.target.value) || 2048 })}
              style={{
                width: '100%',
                background: '#2d3748',
                border: '1px solid #4a5568',
                borderRadius: 4,
                color: '#e2e8f0',
                padding: '4px 8px',
                fontSize: 12,
                boxSizing: 'border-box',
              }}
            />
          </div>

          <div style={{ marginBottom: 10 }}>
            <div style={{ marginBottom: 4, color: '#a0aec0' }}>Temperature</div>
            <input
              type="number"
              step="0.1"
              min="0"
              max="2"
              value={configForm.temperature}
              onChange={(e) => setConfigForm({ ...configForm, temperature: Number(e.target.value) || 0.7 })}
              style={{
                width: '100%',
                background: '#2d3748',
                border: '1px solid #4a5568',
                borderRadius: 4,
                color: '#e2e8f0',
                padding: '4px 8px',
                fontSize: 12,
                boxSizing: 'border-box',
              }}
            />
          </div>

          <div style={{ marginBottom: 10 }}>
            <div style={{ marginBottom: 4, color: '#a0aec0' }}>System Prompt</div>
            <textarea
              value={configForm.system_prompt}
              onChange={(e) => setConfigForm({ ...configForm, system_prompt: e.target.value })}
              rows={6}
              placeholder="Custom system prompt (leave empty for default)"
              style={{
                width: '100%',
                background: '#2d3748',
                border: '1px solid #4a5568',
                borderRadius: 4,
                color: '#e2e8f0',
                padding: '4px 8px',
                fontSize: 11,
                resize: 'vertical',
                boxSizing: 'border-box',
              }}
            />
          </div>
        </div>

        <div style={{ padding: 8, borderTop: '1px solid #2d3748', display: 'flex', gap: 8 }}>
          <button
            onClick={handleSaveConfig}
            style={{
              flex: 1,
              background: '#5a6abf',
              border: 'none',
              borderRadius: 6,
              color: '#e2e8f0',
              padding: '6px 10px',
              cursor: 'pointer',
              fontSize: 12,
            }}
          >
            Save
          </button>
          <button
            onClick={() => setShowSettings(false)}
            style={{
              flex: 1,
              background: '#4a5568',
              border: 'none',
              borderRadius: 6,
              color: '#e2e8f0',
              padding: '6px 10px',
              cursor: 'pointer',
              fontSize: 12,
            }}
          >
            Cancel
          </button>
        </div>
      </div>
    );
  }

  return (
    <div
      style={{
        width: 340,
        background: '#1a202c',
        borderLeft: '1px solid #2d3748',
        display: 'flex',
        flexDirection: 'column',
        height: '100%',
        color: '#e2e8f0',
      }}
    >
      <div
        style={{
          padding: '8px 12px',
          borderBottom: '1px solid #2d3748',
          fontWeight: 600,
          fontSize: 13,
          display: 'flex',
          alignItems: 'center',
          justifyContent: 'space-between',
        }}
      >
        <div style={{ display: 'flex', alignItems: 'center', gap: 6 }}>
          <Bot size={16} style={{ color: '#5a6abf' }} />
          AI Agent
          {agentConfig?.enabled && (
            <span
              style={{
                fontSize: 9,
                background: '#38a169',
                color: '#fff',
                padding: '1px 5px',
                borderRadius: 8,
                fontWeight: 500,
              }}
            >
              LLM
            </span>
          )}
        </div>
        <div style={{ display: 'flex', gap: 4 }}>
          <button
            onClick={() => setShowTemplates(!showTemplates)}
            title="Workflow templates"
            style={{
              background: showTemplates ? '#5a6abf' : 'transparent',
              border: 'none',
              color: showTemplates ? '#fff' : '#718096',
              cursor: 'pointer',
              padding: 2,
              display: 'flex',
              alignItems: 'center',
            }}
          >
            <Workflow size={14} />
          </button>
          <button
            onClick={() => setShowRecommendations(!showRecommendations)}
            title="Model recommendations"
            style={{
              background: showRecommendations ? '#5a6abf' : 'transparent',
              border: 'none',
              color: showRecommendations ? '#fff' : '#718096',
              cursor: 'pointer',
              padding: 2,
              display: 'flex',
              alignItems: 'center',
            }}
          >
            <Sparkles size={14} />
          </button>
          <button
            onClick={handleClearChat}
            title="Clear chat"
            style={{
              background: 'transparent',
              border: 'none',
              color: '#718096',
              cursor: 'pointer',
              padding: 2,
              display: 'flex',
              alignItems: 'center',
            }}
          >
            <Trash2 size={14} />
          </button>
          <button
            onClick={() => { setShowSettings(true); fetchModels(); }}
            title="Agent settings"
            style={{
              background: 'transparent',
              border: 'none',
              color: '#718096',
              cursor: 'pointer',
              padding: 2,
              display: 'flex',
              alignItems: 'center',
            }}
          >
            <Settings size={14} />
          </button>
        </div>
      </div>

      <div style={{ flex: 1, overflowY: 'auto', padding: 8 }}>
        {messages.length === 0 && (
          <div style={{ padding: 20, textAlign: 'center', color: '#4a5568', fontSize: 12 }}>
            <Bot size={32} style={{ color: '#2d3748', marginBottom: 8 }} />
            <div style={{ marginBottom: 8, color: '#718096' }}>
              {agentConfig?.enabled
                ? 'AI Agent with LLM is active. Ask me anything!'
                : 'AI Agent (local mode). Configure LLM in settings for smarter responses.'}
            </div>
            <div style={{ textAlign: 'left', fontSize: 11, color: '#4a5568' }}>
              <div style={{ marginBottom: 4, color: '#718096', fontWeight: 600 }}>Examples:</div>
              <div style={{ padding: '2px 0' }}>&#8226; &quot;Create a txt2img workflow&quot;</div>
              <div style={{ padding: '2px 0' }}>&#8226; &quot;Add a KSampler node&quot;</div>
              <div style={{ padding: '2px 0' }}>&#8226; &quot;Recommend an anime model&quot;</div>
              <div style={{ padding: '2px 0' }}>&#8226; &quot;Run the current workflow&quot;</div>
              <div style={{ padding: '2px 0' }}>&#8226; &quot;Validate my workflow&quot;</div>
            </div>

            {workflowTemplates.length > 0 && (
              <div style={{ marginTop: 12, textAlign: 'left' }}>
                <div style={{ marginBottom: 6, color: '#718096', fontWeight: 600, display: 'flex', alignItems: 'center', gap: 4 }}>
                  <Workflow size={12} /> Quick Templates
                </div>
                <div style={{ display: 'flex', flexWrap: 'wrap', gap: 4 }}>
                  {workflowTemplates.map((t) => (
                    <button
                      key={t.id}
                      onClick={() => {
                        const workflow = templateToWorkflow(t);
                        loadWorkflowFromJson(workflow);
                      }}
                      style={{
                        background: '#2d3748',
                        border: '1px solid #4a5568',
                        borderRadius: 4,
                        color: '#a0aec0',
                        padding: '3px 8px',
                        fontSize: 10,
                        cursor: 'pointer',
                      }}
                    >
                      {t.name}
                    </button>
                  ))}
                </div>
              </div>
            )}

            {modelRecommendations.length > 0 && (
              <div style={{ marginTop: 12, textAlign: 'left' }}>
                <div style={{ marginBottom: 6, color: '#718096', fontWeight: 600, display: 'flex', alignItems: 'center', gap: 4 }}>
                  <Sparkles size={12} /> Popular Models
                </div>
                <div style={{ display: 'flex', flexDirection: 'column', gap: 3 }}>
                  {modelRecommendations.slice(0, 5).map((m) => (
                    <div
                      key={m.name}
                      style={{
                        background: '#2d3748',
                        border: '1px solid #4a5568',
                        borderRadius: 4,
                        padding: '4px 8px',
                        fontSize: 10,
                        color: '#a0aec0',
                      }}
                    >
                      <div style={{ fontWeight: 600, color: '#e2e8f0' }}>{m.name}</div>
                      <div style={{ fontSize: 9, color: '#718096' }}>{m.description}</div>
                    </div>
                  ))}
                </div>
              </div>
            )}
          </div>
        )}

        {messages.map((msg) => (
          <div
            key={msg.id}
            style={{
              display: 'flex',
              gap: 8,
              marginBottom: 8,
              alignItems: 'flex-start',
            }}
          >
            <div
              style={{
                width: 24,
                height: 24,
                borderRadius: 12,
                background: msg.role === 'user' ? '#4a5568' : '#5a6abf',
                display: 'flex',
                alignItems: 'center',
                justifyContent: 'center',
                flexShrink: 0,
              }}
            >
              {msg.role === 'user' ? <User size={12} /> : <Bot size={12} />}
            </div>
            <div style={{ flex: 1, minWidth: 0 }}>
              <div
                style={{
                  background: '#2d3748',
                  borderRadius: 8,
                  padding: '6px 10px',
                  fontSize: 12,
                  lineHeight: 1.5,
                  whiteSpace: 'pre-wrap',
                  wordBreak: 'break-word',
                }}
              >
                {renderMessageContent(msg.content)}
              </div>

              {msg.actionResults && msg.actionResults.length > 0 && (
                <div style={{ marginTop: 4 }}>
                  {msg.actionResults.map((result, i) => (
                    <div
                      key={i}
                      style={{
                        display: 'flex',
                        alignItems: 'center',
                        gap: 4,
                        fontSize: 10,
                        padding: '2px 6px',
                        marginBottom: 2,
                        borderRadius: 4,
                        background: result.success ? 'rgba(56,161,105,0.15)' : 'rgba(229,62,62,0.15)',
                        color: result.success ? '#68d391' : '#fc8181',
                      }}
                    >
                      {result.success ? <CheckCircle size={10} /> : <XCircle size={10} />}
                      <span>{result.message}</span>
                    </div>
                  ))}
                </div>
              )}
            </div>
          </div>
        ))}

        {isProcessing && (
          <div style={{ display: 'flex', gap: 8, alignItems: 'center', color: '#718096', fontSize: 12, padding: '4px 0' }}>
            <Loader2 size={14} style={{ animation: 'spin 1s linear infinite' }} />
            Thinking...
          </div>
        )}

        <div ref={messagesEndRef} />
      </div>

      {showTemplates && workflowTemplates.length > 0 && (
        <div style={{
          maxHeight: 200,
          overflowY: 'auto',
          padding: 8,
          borderTop: '1px solid #2d3748',
          borderBottom: '1px solid #2d3748',
          background: '#1e2a3a',
        }}>
          <div style={{ fontSize: 11, fontWeight: 600, color: '#718096', marginBottom: 6, display: 'flex', alignItems: 'center', gap: 4 }}>
            <BookOpen size={11} /> Workflow Templates
          </div>
          <div style={{ display: 'flex', flexDirection: 'column', gap: 4 }}>
            {workflowTemplates.map((t) => (
              <button
                key={t.id}
                onClick={() => {
                  const workflow = templateToWorkflow(t);
                  loadWorkflowFromJson(workflow);
                  setShowTemplates(false);
                }}
                style={{
                  background: '#2d3748',
                  border: '1px solid #4a5568',
                  borderRadius: 4,
                  color: '#e2e8f0',
                  padding: '6px 10px',
                  fontSize: 11,
                  cursor: 'pointer',
                  textAlign: 'left',
                  width: '100%',
                }}
              >
                <div style={{ fontWeight: 600 }}>{t.name}</div>
                <div style={{ fontSize: 9, color: '#718096', marginTop: 2 }}>{t.description}</div>
              </button>
            ))}
          </div>
        </div>
      )}

      {showRecommendations && modelRecommendations.length > 0 && (
        <div style={{
          maxHeight: 200,
          overflowY: 'auto',
          padding: 8,
          borderTop: '1px solid #2d3748',
          borderBottom: '1px solid #2d3748',
          background: '#1e2a3a',
        }}>
          <div style={{ fontSize: 11, fontWeight: 600, color: '#718096', marginBottom: 6, display: 'flex', alignItems: 'center', gap: 4 }}>
            <Sparkles size={11} /> Model Recommendations
          </div>
          <div style={{ display: 'flex', flexDirection: 'column', gap: 4 }}>
            {modelRecommendations.map((m) => (
              <div
                key={m.name}
                style={{
                  background: '#2d3748',
                  border: '1px solid #4a5568',
                  borderRadius: 4,
                  padding: '6px 10px',
                  fontSize: 11,
                }}
              >
                <div style={{ fontWeight: 600, color: '#e2e8f0' }}>{m.name}</div>
                <div style={{ fontSize: 9, color: '#718096', marginTop: 2 }}>{m.description}</div>
                <div style={{ fontSize: 9, color: '#5a6abf', marginTop: 2 }}>
                  {m.base_model} · steps={m.recommended_settings.steps} · cfg={m.recommended_settings.cfg} · {m.recommended_settings.resolution}
                </div>
                {m.trigger_tokens.length > 0 && (
                  <div style={{ fontSize: 9, color: '#d69e2e', marginTop: 1 }}>
                    Triggers: {m.trigger_tokens.join(', ')}
                  </div>
                )}
              </div>
            ))}
          </div>
        </div>
      )}

      <div style={{ padding: 8, borderTop: '1px solid #2d3748' }}>
        <div
          style={{
            display: 'flex',
            gap: 6,
            alignItems: 'flex-end',
          }}
        >
          <textarea
            value={input}
            onChange={(e) => setInput(e.target.value)}
            onKeyDown={handleKeyDown}
            placeholder={agentConfig?.enabled ? 'Ask AI Agent...' : 'Type a command (or enable LLM in settings)...'}
            rows={2}
            style={{
              flex: 1,
              background: '#2d3748',
              border: '1px solid #4a5568',
              borderRadius: 6,
              color: '#e2e8f0',
              padding: '6px 8px',
              fontSize: 12,
              resize: 'none',
              outline: 'none',
            }}
          />
          <button
            onClick={handleSend}
            disabled={isProcessing || !input.trim()}
            style={{
              background: isProcessing || !input.trim() ? '#4a5568' : '#5a6abf',
              border: 'none',
              borderRadius: 6,
              color: '#e2e8f0',
              padding: '6px 10px',
              cursor: isProcessing || !input.trim() ? 'not-allowed' : 'pointer',
              display: 'flex',
              alignItems: 'center',
            }}
          >
            <Send size={14} />
          </button>
        </div>
      </div>

      <style>{`
        @keyframes spin {
          from { transform: rotate(0deg); }
          to { transform: rotate(360deg); }
        }
      `}</style>
    </div>
  );
};

function templateToWorkflow(template: WorkflowTemplate): Record<string, unknown> {
  const workflowNodes: Record<string, unknown> = {};
  for (const node of template.nodes) {
    workflowNodes[node.id] = {
      class_type: node.class_type,
      inputs: node.inputs,
      _meta: { title: node.title },
    };
  }
  const workflowLinks = template.connections.map((conn, idx) => [
    idx + 1,
    parseInt(conn.source),
    conn.source_handle,
    parseInt(conn.target),
    conn.target_handle,
  ]);
  return {
    last_node_id: String(template.nodes.length),
    last_link_id: String(template.connections.length),
    nodes: template.nodes.map((n) => ({
      id: parseInt(n.id),
      type: n.class_type,
      pos: [n.x, n.y],
      size: { 0: 220, 1: 100 },
      flags: {},
      order: parseInt(n.id),
      mode: 0,
      properties: {},
      widgets_values: [],
      title: n.title,
    })),
    links: workflowLinks,
    groups: [],
    config: {},
    extra: {},
    version: 0.4,
  };
}

function parseActions(text: string): AgentAction[] {
  const actions: AgentAction[] = [];
  const actionRegex = /```action\n([\s\S]*?)```/g;
  let match;
  while ((match = actionRegex.exec(text)) !== null) {
    try {
      const parsed = JSON.parse(match[1]);
      if (parsed.type && parsed.payload) {
        actions.push(parsed as AgentAction);
      }
    } catch {
      // skip invalid actions
    }
  }
  return actions;
}

async function processLocalMessage(text: string): Promise<string> {
  const lower = text.toLowerCase();

  if (lower.includes('txt2img') || lower.includes('text to image') || lower.includes('text-to-image') || lower.includes('文生图')) {
    return `I'll create a text-to-image workflow for you!

1. **CheckpointLoaderSimple** - Load the SD model
2. **CLIPTextEncode** (positive) - Encode the positive prompt
3. **CLIPTextEncode** (negative) - Encode the negative prompt
4. **EmptyLatentImage** - Create empty latent
5. **KSampler** - Sample the latent
6. **VAEDecode** - Decode latent to image
7. **SaveImage** - Save the output

\`\`\`action
{"type": "add_node", "payload": {"classType": "CheckpointLoaderSimple", "x": 50, "y": 100}}
\`\`\`
\`\`\`action
{"type": "add_node", "payload": {"classType": "CLIPTextEncode", "x": 300, "y": 50}}
\`\`\`
\`\`\`action
{"type": "add_node", "payload": {"classType": "CLIPTextEncode", "x": 300, "y": 200}}
\`\`\`
\`\`\`action
{"type": "add_node", "payload": {"classType": "EmptyLatentImage", "x": 300, "y": 350}}
\`\`\`
\`\`\`action
{"type": "add_node", "payload": {"classType": "KSampler", "x": 550, "y": 150}}
\`\`\`
\`\`\`action
{"type": "add_node", "payload": {"classType": "VAEDecode", "x": 800, "y": 150}}
\`\`\`
\`\`\`action
{"type": "add_node", "payload": {"classType": "SaveImage", "x": 1050, "y": 150}}
\`\`\`

Connect the nodes and adjust the parameters as needed!`;
  }

  if (lower.includes('img2img') || lower.includes('image to image') || lower.includes('image-to-image') || lower.includes('图生图')) {
    return `I'll create an image-to-image workflow for you!

1. **CheckpointLoaderSimple** - Load the SD model
2. **LoadImage** - Load the input image
3. **CLIPTextEncode** (positive) - Encode the positive prompt
4. **CLIPTextEncode** (negative) - Encode the negative prompt
5. **VAEEncode** - Encode image to latent
6. **KSampler** - Sample the latent
7. **VAEDecode** - Decode latent to image
8. **SaveImage** - Save the output

\`\`\`action
{"type": "add_node", "payload": {"classType": "CheckpointLoaderSimple", "x": 50, "y": 100}}
\`\`\`
\`\`\`action
{"type": "add_node", "payload": {"classType": "LoadImage", "x": 50, "y": 300}}
\`\`\`
\`\`\`action
{"type": "add_node", "payload": {"classType": "CLIPTextEncode", "x": 300, "y": 50}}
\`\`\`
\`\`\`action
{"type": "add_node", "payload": {"classType": "CLIPTextEncode", "x": 300, "y": 200}}
\`\`\`
\`\`\`action
{"type": "add_node", "payload": {"classType": "VAEEncode", "x": 300, "y": 350}}
\`\`\`
\`\`\`action
{"type": "add_node", "payload": {"classType": "KSampler", "x": 550, "y": 200}}
\`\`\`
\`\`\`action
{"type": "add_node", "payload": {"classType": "VAEDecode", "x": 800, "y": 200}}
\`\`\`
\`\`\`action
{"type": "add_node", "payload": {"classType": "SaveImage", "x": 1050, "y": 200}}
\`\`\``;
  }

  if (lower.includes('add') && (lower.includes('ksampler') || lower.includes('sampler'))) {
    return `Adding a KSampler node for you.

\`\`\`action
{"type": "add_node", "payload": {"classType": "KSampler", "x": 400, "y": 200}}
\`\`\`

The KSampler node handles the denoising process. Configure the seed, steps, CFG, and sampler method to control generation quality.`;
  }

  if (lower.includes('add') && (lower.includes('checkpoint') || lower.includes('loader') || lower.includes('model'))) {
    return `Adding a CheckpointLoaderSimple node.

\`\`\`action
{"type": "add_node", "payload": {"classType": "CheckpointLoaderSimple", "x": 50, "y": 150}}
\`\`\`

This node loads a Stable Diffusion model checkpoint and outputs MODEL, CLIP, and VAE.`;
  }

  if (lower.includes('add') && (lower.includes('save') || lower.includes('preview'))) {
    return `Adding a SaveImage node.

\`\`\`action
{"type": "add_node", "payload": {"classType": "SaveImage", "x": 900, "y": 200}}
\`\`\`

This node saves the generated image to the output directory.`;
  }

  if ((lower.includes('text to video') || lower.includes('text-to-video') || lower.includes('txt2vid') || lower.includes('文生视频') || lower.includes('生成视频')) && !lower.includes('image to video') && !lower.includes('图生视频')) {
    return `I'll create a text-to-video workflow using Wan2.1!

\`\`\`action
{"type": "load_workflow_template", "payload": {"templateId": "wan_txt2vid"}}
\`\`\`

This workflow uses:
1. **WanLoader** - Load the Wan2.1 video model
2. **CLIPTextEncode** (positive/negative) - Encode prompts
3. **WanVideoSampler** - Generate video latent (832x480, 33 frames)
4. **VideoVAEDecode** - Decode video latent to video frames (NOT VAEDecode!)
5. **SaveVideo** - Save the output video

Important: Use **VideoVAEDecode** for video workflows, not VAEDecode which only handles single images.`;
  }

  if (lower.includes('image to video') || lower.includes('image-to-video') || lower.includes('img2vid') || lower.includes('图生视频')) {
    return `I'll create an image-to-video workflow using Wan2.1!

\`\`\`action
{"type": "load_workflow_template", "payload": {"templateId": "wan_img2vid"}}
\`\`\`

This workflow uses:
1. **WanLoader** - Load the Wan2.1 video model
2. **LoadImage** - Load the reference image (first frame)
3. **CLIPTextEncode** (positive/negative) - Encode prompts
4. **WanVideoSampler** - Generate video from the init image
5. **VideoVAEDecode** - Decode video latent to video frames
6. **SaveVideo** - Save the output video

The init image guides the video generation as the first frame.`;
  }

  if (lower.includes('add') && (lower.includes('video') && lower.includes('vae') && lower.includes('decode'))) {
    return `Adding a VideoVAEDecode node for video workflows.

\`\`\`action
{"type": "add_node", "payload": {"classType": "VideoVAEDecode", "x": 800, "y": 200}}
\`\`\`

This node decodes video LATENT into VIDEO frames. Use this instead of VAEDecode for video workflows — VAEDecode only handles single images!`;
  }

  if (lower.includes('add') && (lower.includes('video') && lower.includes('vae') && lower.includes('encode'))) {
    return `Adding a VideoVAEEncode node for video workflows.

\`\`\`action
{"type": "add_node", "payload": {"classType": "VideoVAEEncode", "x": 300, "y": 200}}
\`\`\`

This node encodes VIDEO frames into LATENT. Use this instead of VAEEncode for video workflows.`;
  }

  if (lower.includes('add') && (lower.includes('save') && lower.includes('video'))) {
    return `Adding a SaveVideo node.

\`\`\`action
{"type": "add_node", "payload": {"classType": "SaveVideo", "x": 1100, "y": 200}}
\`\`\`

This node saves the generated video to the output directory. Supports mp4, gif, webm formats.`;
  }

  if (lower.includes('add') && (lower.includes('clip') || lower.includes('encode') || lower.includes('prompt'))) {
    return `Adding a CLIPTextEncode node.

\`\`\`action
{"type": "add_node", "payload": {"classType": "CLIPTextEncode", "x": 300, "y": 150}}
\`\`\`

This node encodes a text prompt into a CONDITIONING output for the KSampler.`;
  }

  if (lower.includes('add') && (lower.includes('latent') || lower.includes('empty'))) {
    return `Adding an EmptyLatentImage node.

\`\`\`action
{"type": "add_node", "payload": {"classType": "EmptyLatentImage", "x": 300, "y": 350}}
\`\`\`

This creates an empty latent image with configurable width, height, and batch size.`;
  }

  if (lower.includes('add') && (lower.includes('vae') || lower.includes('decode'))) {
    return `Adding a VAEDecode node.

\`\`\`action
{"type": "add_node", "payload": {"classType": "VAEDecode", "x": 800, "y": 200}}
\`\`\`

This decodes a LATENT into an IMAGE using the VAE.`;
  }

  if (lower.includes('run') || lower.includes('execute') || lower.includes('generate') || lower.includes('执行') || lower.includes('运行')) {
    return `Running the current workflow!

\`\`\`action
{"type": "run_workflow", "payload": {}}
\`\`\`

The workflow has been submitted to the queue. You can monitor progress in the status bar.`;
  }

  if (lower.includes('validate') || lower.includes('check') || lower.includes('验证') || lower.includes('检查')) {
    return `Validating the current workflow...

\`\`\`action
{"type": "validate_workflow", "payload": {}}
\`\`\`

This checks the workflow for errors like missing connections or invalid types.`;
  }

  if (lower.includes('clear') || lower.includes('reset') || lower.includes('清空') || lower.includes('重置')) {
    return `Clearing the workflow...

\`\`\`action
{"type": "clear_workflow", "payload": {}}
\`\`\`

All nodes and connections have been removed.`;
  }

  if (lower.includes('help') || lower.includes('what can you do') || lower.includes('帮助') || lower.includes('能做什么')) {
    return `I can help you with:

**Workflow Building:**
- "Create a txt2img workflow" - Build a text-to-image pipeline
- "Create an img2img workflow" - Build an image-to-image pipeline
- "Add a KSampler node" - Add specific nodes
- "Load template txt2img_sd15" - Load a workflow template

**Model Recommendations:**
- "Recommend an anime model" - Get model suggestions
- "What model should I use for realistic images?" - Style-based recommendations

**Workflow Execution:**
- "Run the current workflow" - Submit to queue
- "Validate my workflow" - Check for errors
- "Clear the workflow" - Reset everything

**Node Types Available:**
- CheckpointLoaderSimple, CLIPTextEncode, KSampler
- VAEDecode, VAEEncode, EmptyLatentImage
- SaveImage, LoadImage, LoraLoader, and more...

Enable LLM in settings for more intelligent responses!`;
  }

  return `I understand you want: "${text}"

In local mode, I support these commands:
- **Build workflows**: "Create a txt2img/img2img workflow"
- **Add nodes**: "Add a [node type] node"
- **Run/Validate/Clear**: "Run/Validate/Clear the workflow"
- **Video generation**: "Create a text-to-video workflow"
- **Help**: "What can you do?"

For more intelligent and flexible responses, enable LLM in the Agent Settings (gear icon).`;
}

export { AIAgent };
