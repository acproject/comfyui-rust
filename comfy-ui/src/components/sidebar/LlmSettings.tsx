import { useState, useEffect, type FC } from 'react';
import { Settings, Save, Loader2 } from 'lucide-react';
import { api } from '@/api/client';
import type { LlmConfig } from '@/types/api';

const LlmSettings: FC = () => {
  const [config, setConfig] = useState<LlmConfig | null>(null);
  const [loading, setLoading] = useState(true);
  const [saving, setSaving] = useState(false);
  const [message, setMessage] = useState<{ type: 'success' | 'error'; text: string } | null>(null);

  useEffect(() => {
    loadConfig();
  }, []);

  const loadConfig = async () => {
    try {
      setLoading(true);
      const c = await api.getLlmConfig();
      setConfig(c);
    } catch (e) {
      setMessage({ type: 'error', text: `Failed to load LLM config: ${e}` });
    } finally {
      setLoading(false);
    }
  };

  const handleSave = async () => {
    if (!config) return;
    try {
      setSaving(true);
      setMessage(null);
      const updated = await api.setLlmConfig(config);
      setConfig(updated);
      setMessage({ type: 'success', text: 'LLM config saved' });
    } catch (e) {
      setMessage({ type: 'error', text: `Failed to save: ${e}` });
    } finally {
      setSaving(false);
    }
  };

  const updateConfig = (partial: Partial<LlmConfig>) => {
    if (!config) return;
    setConfig({ ...config, ...partial });
  };

  if (loading) {
    return (
      <div style={{ padding: 12, color: '#a0aec0', fontSize: 12, textAlign: 'center' }}>
        <Loader2 size={16} style={{ animation: 'spin 1s linear infinite', display: 'inline-block' }} />
        <span style={{ marginLeft: 6 }}>Loading LLM config...</span>
      </div>
    );
  }

  if (!config) {
    return (
      <div style={{ padding: 12, color: '#fc8181', fontSize: 12 }}>
        Failed to load LLM configuration
      </div>
    );
  }

  const inputStyle: React.CSSProperties = {
    width: '100%',
    background: '#2d3748',
    border: '1px solid #4a5568',
    borderRadius: 4,
    color: '#e2e8f0',
    padding: '4px 8px',
    fontSize: 12,
    boxSizing: 'border-box',
  };

  const labelStyle: React.CSSProperties = {
    marginBottom: 4,
    color: '#a0aec0',
    fontSize: 11,
  };

  const sectionStyle: React.CSSProperties = {
    marginBottom: 10,
  };

  return (
    <div style={{ padding: 8, fontSize: 12, color: '#e2e8f0' }}>
      <div style={{ display: 'flex', alignItems: 'center', gap: 6, marginBottom: 12, fontWeight: 600 }}>
        <Settings size={14} style={{ color: '#5a6abf' }} />
        LLM Inference Settings
      </div>

      <div style={sectionStyle}>
        <div style={labelStyle}>Mode</div>
        <select
          value={config.mode}
          onChange={(e) => {
            updateConfig({ mode: e.target.value });
          }}
          style={inputStyle}
        >
          <option value="local">Local (llama-cli)</option>
          <option value="remote">Remote API (OpenAI-compatible)</option>
        </select>
      </div>

      {config.mode === 'local' ? (
        <>
          <div style={sectionStyle}>
            <div style={labelStyle}>CLI Path</div>
            <input
              type="text"
              value={config.cli_path}
              onChange={(e) => updateConfig({ cli_path: e.target.value })}
              style={inputStyle}
              placeholder="/path/to/llama-cli"
            />
          </div>
          <div style={sectionStyle}>
            <div style={labelStyle}>Extra Args</div>
            <input
              type="text"
              value={config.extra_args}
              onChange={(e) => updateConfig({ extra_args: e.target.value })}
              style={inputStyle}
              placeholder="--ctx-size 4096 --threads 4"
            />
          </div>
        </>
      ) : (
        <>
          <div style={sectionStyle}>
            <div style={labelStyle}>API URL</div>
            <input
              type="text"
              value={config.api_url}
              onChange={(e) => updateConfig({ api_url: e.target.value })}
              style={inputStyle}
              placeholder="http://127.0.0.1:8080"
            />
          </div>
          <div style={sectionStyle}>
            <div style={labelStyle}>API Key</div>
            <input
              type="password"
              value={config.api_key || ''}
              onChange={(e) => updateConfig({ api_key: e.target.value || null })}
              style={inputStyle}
              placeholder="sk-..."
            />
          </div>
          <div style={sectionStyle}>
            <div style={labelStyle}>Model Name</div>
            <input
              type="text"
              value={config.model}
              onChange={(e) => updateConfig({ model: e.target.value })}
              style={inputStyle}
              placeholder="default"
            />
          </div>
        </>
      )}

      <div style={sectionStyle}>
        <div style={labelStyle}>Max Tokens</div>
        <input
          type="number"
          value={config.max_tokens}
          onChange={(e) => updateConfig({ max_tokens: parseInt(e.target.value) || 512 })}
          style={inputStyle}
          min={1}
          max={32768}
        />
      </div>

      <div style={sectionStyle}>
        <div style={labelStyle}>Temperature</div>
        <input
          type="number"
          value={config.temperature}
          onChange={(e) => updateConfig({ temperature: parseFloat(e.target.value) || 0.7 })}
          style={inputStyle}
          min={0}
          max={2}
          step={0.1}
        />
      </div>

      <div style={sectionStyle}>
        <div style={labelStyle}>Top P</div>
        <input
          type="number"
          value={config.top_p}
          onChange={(e) => updateConfig({ top_p: parseFloat(e.target.value) || 0.9 })}
          style={inputStyle}
          min={0}
          max={1}
          step={0.05}
        />
      </div>

      <div style={sectionStyle}>
        <div style={labelStyle}>System Prompt</div>
        <textarea
          value={config.system_prompt}
          onChange={(e) => updateConfig({ system_prompt: e.target.value })}
          style={{ ...inputStyle, minHeight: 60, resize: 'vertical' }}
          placeholder="Optional system prompt for LLM..."
        />
      </div>

      {message && (
        <div style={{
          padding: '4px 8px',
          marginBottom: 8,
          borderRadius: 4,
          fontSize: 11,
          background: message.type === 'success' ? '#1a4731' : '#4a2020',
          color: message.type === 'success' ? '#68d391' : '#fc8181',
        }}>
          {message.text}
        </div>
      )}

      <button
        onClick={handleSave}
        disabled={saving}
        style={{
          width: '100%',
          padding: '6px 12px',
          background: '#4a9eff',
          border: 'none',
          borderRadius: 4,
          color: '#fff',
          fontSize: 12,
          cursor: saving ? 'wait' : 'pointer',
          display: 'flex',
          alignItems: 'center',
          justifyContent: 'center',
          gap: 6,
          opacity: saving ? 0.7 : 1,
        }}
      >
        {saving ? <Loader2 size={14} style={{ animation: 'spin 1s linear infinite' }} /> : <Save size={14} />}
        {saving ? 'Saving...' : 'Save LLM Config'}
      </button>
    </div>
  );
};

export { LlmSettings };
