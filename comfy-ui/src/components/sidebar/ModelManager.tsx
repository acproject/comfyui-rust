import { useState, useEffect, useRef, useCallback, type FC } from 'react';
import { Search, ChevronRight, ChevronDown, Trash2, Upload, RefreshCw, FolderOpen, HardDrive, Settings, Save, CheckCircle, AlertCircle, Download, ExternalLink, Filter, X } from 'lucide-react';
import { useModelManagerStore, MODEL_TYPES } from '@/store/models';
import { api } from '@/api/client';
import type { ModelFileInfo, ServerConfig, ModelDownloadEntry, DownloadProgress } from '@/types/api';

const MODEL_TYPE_ICONS: Record<string, string> = {
  checkpoints: '🏛️',
  loras: '🔧',
  vae: '🎨',
  text_encoders: '📝',
  diffusion_models: '🌀',
  clip_vision: '👁️',
  style_models: '🎭',
  embeddings: '📌',
  diffusers: '💨',
  vae_approx: '🖼️',
  controlnet: '🎮',
  gligen: '🎯',
  upscale_models: '🔍',
  latent_upscale_models: '⬆️',
  hypernetworks: '🧠',
  photomarker: '📷',
  classifiers: '🏷️',
  model_patches: '🩹',
  audio_encoders: '🔊',
};

function formatFileSize(bytes: number): string {
  if (bytes === 0) return '0 B';
  const units = ['B', 'KB', 'MB', 'GB', 'TB'];
  const i = Math.floor(Math.log(bytes) / Math.log(1024));
  return `${(bytes / Math.pow(1024, i)).toFixed(1)} ${units[i]}`;
}

function formatDate(timestamp: number | null): string {
  if (!timestamp) return '';
  return new Date(timestamp * 1000).toLocaleDateString();
}

const ModelManager: FC = () => {
  const models = useModelManagerStore((s) => s.models);
  const loading = useModelManagerStore((s) => s.loading);
  const error = useModelManagerStore((s) => s.error);
  const loadModels = useModelManagerStore((s) => s.loadModels);
  const deleteModel = useModelManagerStore((s) => s.deleteModel);
  const uploadModel = useModelManagerStore((s) => s.uploadModel);

  const [search, setSearch] = useState('');
  const [collapsed, setCollapsed] = useState<Record<string, boolean>>({});
  const [uploadingType, setUploadingType] = useState<string | null>(null);
  const [deletingPath, setDeletingPath] = useState<string | null>(null);
  const fileInputRef = useRef<HTMLInputElement>(null);

  const [showSettings, setShowSettings] = useState(false);
  const [inferenceConfig, setInferenceConfig] = useState<{
    backend: string;
    sd_cli_path: string;
    n_threads: number;
    flash_attn: boolean;
    offload_params_to_cpu: boolean;
    enable_mmap: boolean;
    hf_token: string;
  }>({
    backend: 'local',
    sd_cli_path: '',
    n_threads: 4,
    flash_attn: false,
    offload_params_to_cpu: false,
    enable_mmap: true,
    hf_token: '',
  });
  const [configLoaded, setConfigLoaded] = useState(false);
  const [savingConfig, setSavingConfig] = useState(false);
  const [saveResult, setSaveResult] = useState<'success' | 'error' | null>(null);

  const [showDownloads, setShowDownloads] = useState(false);
  const [downloadList, setDownloadList] = useState<ModelDownloadEntry[]>([]);
  const [downloadLoading, setDownloadLoading] = useState(false);
  const [downloadLoaded, setDownloadLoaded] = useState(false);
  const [downloadCategory, setDownloadCategory] = useState<string>('all');
  const [downloadingUrls, setDownloadingUrls] = useState<Set<string>>(new Set());
  const [downloadSearch, setDownloadSearch] = useState('');
  const [activeDownloads, setActiveDownloads] = useState<DownloadProgress[]>([]);

  const loadDownloadList = () => {
    setDownloadLoading(true);
    api.getModelDownloadList().then((list) => {
      setDownloadList(list);
      setDownloadLoaded(true);
    }).catch(() => {}).finally(() => {
      setDownloadLoading(false);
    });
  };

  const pollDownloads = useCallback(async () => {
    try {
      const resp = await api.getDownloadProgress();
      setActiveDownloads(resp.downloads);
      const hasActive = resp.downloads.some(d => d.status === 'Pending' || d.status === 'Downloading');
      if (!hasActive) return false;
      return true;
    } catch {
      return false;
    }
  }, []);

  useEffect(() => {
    let timer: ReturnType<typeof setInterval> | null = null;
    let active = true;
    const startPolling = async () => {
      const hasActive = await pollDownloads();
      if (hasActive && active) {
        timer = setInterval(async () => {
          const stillActive = await pollDownloads();
          if (!stillActive && timer) {
            clearInterval(timer);
            timer = null;
            loadModels();
          }
        }, 2000);
      }
    };
    startPolling();
    return () => {
      active = false;
      if (timer) clearInterval(timer);
    };
  }, [pollDownloads, loadModels]);

  useEffect(() => {
    loadModels();
  }, [loadModels]);

  useEffect(() => {
    if (!configLoaded) {
      api.getConfig().then((config) => {
        setInferenceConfig({
          backend: config.inference.backend,
          sd_cli_path: config.inference.sd_cli_path || '',
          n_threads: config.inference.n_threads,
          flash_attn: config.inference.flash_attn,
          offload_params_to_cpu: config.inference.offload_params_to_cpu,
          enable_mmap: config.inference.enable_mmap,
          hf_token: config.inference.hf_token || '',
        });
        setConfigLoaded(true);
      }).catch(() => {});
    }
  }, [configLoaded]);

  useEffect(() => {
    if (showDownloads && !downloadLoaded) {
      loadDownloadList();
    }
  }, [showDownloads, downloadLoaded]);

  const handleDownload = async (url: string, modelType: string, filename?: string) => {
    const key = `${url}:${modelType}`;
    setDownloadingUrls((prev) => new Set(prev).add(key));
    try {
      await api.downloadModel({ url, model_type: modelType, filename });
      pollDownloads();
    } catch {
    } finally {
      setDownloadingUrls((prev) => {
        const next = new Set(prev);
        next.delete(key);
        return next;
      });
    }
  };

  const handleSaveConfig = async () => {
    setSavingConfig(true);
    setSaveResult(null);
    try {
      const currentConfig = await api.getConfig();
      const updatedConfig: ServerConfig = {
        ...currentConfig,
        inference: {
          ...currentConfig.inference,
          backend: inferenceConfig.backend,
          sd_cli_path: inferenceConfig.sd_cli_path || null,
          n_threads: inferenceConfig.n_threads,
          flash_attn: inferenceConfig.flash_attn,
          offload_params_to_cpu: inferenceConfig.offload_params_to_cpu,
          enable_mmap: inferenceConfig.enable_mmap,
          hf_token: inferenceConfig.hf_token || null,
        },
      };
      await api.updateConfig(updatedConfig);
      setSaveResult('success');
      setTimeout(() => setSaveResult(null), 3000);
    } catch {
      setSaveResult('error');
    } finally {
      setSavingConfig(false);
    }
  };

  const toggleCategory = (cat: string) => {
    setCollapsed((prev) => ({ ...prev, [cat]: !prev[cat] }));
  };

  const handleDelete = async (modelType: string, path: string) => {
    setDeletingPath(path);
    try {
      await deleteModel(modelType, path);
    } finally {
      setDeletingPath(null);
    }
  };

  const handleUploadClick = (modelType: string) => {
    setUploadingType(modelType);
    fileInputRef.current?.click();
  };

  const handleFileSelect = async (e: React.ChangeEvent<HTMLInputElement>) => {
    const file = e.target.files?.[0];
    if (!file || !uploadingType) return;

    try {
      await uploadModel(file, uploadingType);
    } catch {
    } finally {
      setUploadingType(null);
      if (fileInputRef.current) {
        fileInputRef.current.value = '';
      }
    }
  };

  const filteredTypes = MODEL_TYPES.filter((type) => {
    if (!search) return true;
    const q = search.toLowerCase();
    const files = models[type] || [];
    return (
      type.toLowerCase().includes(q) ||
      files.some((f) => f.name.toLowerCase().includes(q) || f.path.toLowerCase().includes(q))
    );
  });

  const getModelCount = (type: string): number => {
    return (models[type] || []).length;
  };

  const getTotalSize = (type: string): number => {
    return (models[type] || []).reduce((sum, f) => sum + f.size, 0);
  };

  const getFilteredFiles = (type: string): ModelFileInfo[] => {
    const files = models[type] || [];
    if (!search) return files;
    const q = search.toLowerCase();
    return files.filter(
      (f) => f.name.toLowerCase().includes(q) || f.path.toLowerCase().includes(q)
    );
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
      <input
        ref={fileInputRef}
        type="file"
        style={{ display: 'none' }}
        onChange={handleFileSelect}
        accept=".safetensors,.ckpt,.pt,.pth,.bin,.onnx,.gguf,.sft,.json"
      />

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
            placeholder="Search models..."
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
          <button
            onClick={() => loadModels()}
            disabled={loading}
            style={{
              background: 'transparent',
              border: 'none',
              cursor: loading ? 'wait' : 'pointer',
              color: '#718096',
              padding: 0,
              display: 'flex',
              alignItems: 'center',
            }}
            title="Refresh"
          >
            <RefreshCw size={13} style={{ animation: loading ? 'spin 1s linear infinite' : 'none' }} />
          </button>
        </div>
      </div>

      {error && (
        <div
          style={{
            padding: '6px 10px',
            background: '#742a2a',
            fontSize: 11,
            color: '#fed7d7',
          }}
        >
          {error}
        </div>
      )}

      <div style={{ borderBottom: '1px solid #333' }}>
        {activeDownloads.length > 0 && (
          <div style={{ padding: '4px 10px' }}>
            {activeDownloads.map((dl) => {
              const percent = dl.total_bytes > 0 ? (dl.downloaded_bytes / dl.total_bytes) * 100 : 0;
              const isFailed = dl.status === 'Failed';
              const isCompleted = dl.status === 'Completed';
              const isPending = dl.status === 'Pending';
              const barColor = isFailed ? '#e53e3e' : isCompleted ? '#38a169' : '#3b5998';
              const statusText = isPending ? '等待中...' :
                isFailed ? `失败: ${dl.error || '未知错误'}` :
                isCompleted ? '完成' :
                dl.total_bytes > 0 ? `${percent.toFixed(1)}%` : '下载中...';
              return (
                <div key={dl.id} style={{ marginBottom: 4, padding: '4px 0' }}>
                  <div style={{ display: 'flex', alignItems: 'center', gap: 4, marginBottom: 2 }}>
                    <span style={{ fontSize: 10, color: '#e2e8f0', flex: 1, overflow: 'hidden', textOverflow: 'ellipsis', whiteSpace: 'nowrap' }}>
                      {dl.filename}
                    </span>
                    <span style={{ fontSize: 8, color: isFailed ? '#fc8181' : isCompleted ? '#68d391' : '#718096', flexShrink: 0 }}>
                      {statusText}
                    </span>
                    {(isCompleted || isFailed) && (
                      <button
                        onClick={() => {
                          setActiveDownloads((prev) => prev.filter(d => d.id !== dl.id));
                          if (activeDownloads.every(d => d.id === dl.id || d.status === 'Completed' || d.status === 'Failed')) {
                            api.clearDownloadProgress().catch(() => {});
                          }
                        }}
                        style={{
                          background: 'transparent',
                          border: 'none',
                          cursor: 'pointer',
                          color: '#555',
                          padding: 0,
                          display: 'flex',
                          alignItems: 'center',
                        }}
                      >
                        <X size={9} />
                      </button>
                    )}
                  </div>
                  <div style={{
                    width: '100%',
                    height: 3,
                    background: '#2a2a3e',
                    borderRadius: 2,
                    overflow: 'hidden',
                  }}>
                    <div style={{
                      width: `${isPending ? 0 : isCompleted ? 100 : percent}%`,
                      height: '100%',
                      background: barColor,
                      borderRadius: 2,
                      transition: 'width 0.3s ease',
                    }} />
                  </div>
                  <div style={{ fontSize: 8, color: '#555', marginTop: 1, display: 'flex', justifyContent: 'space-between' }}>
                    <span>{formatFileSize(dl.downloaded_bytes)}</span>
                    {dl.total_bytes > 0 && <span>{formatFileSize(dl.total_bytes)}</span>}
                  </div>
                </div>
              );
            })}
          </div>
        )}

        <div
          style={{
            padding: '6px 10px',
            fontSize: 11,
            fontWeight: 600,
            color: '#a0aec0',
            cursor: 'pointer',
            display: 'flex',
            alignItems: 'center',
            gap: 6,
            userSelect: 'none',
          }}
          onClick={() => setShowSettings(!showSettings)}
        >
          {showSettings ? <ChevronDown size={12} /> : <ChevronRight size={12} />}
          <Settings size={12} />
          <span>推理设置</span>
          <span style={{ fontSize: 9, color: '#555', marginLeft: 'auto' }}>
            {inferenceConfig.backend === 'cli' ? 'CLI' : inferenceConfig.backend === 'local' ? 'FFI' : inferenceConfig.backend}
          </span>
        </div>

        {showSettings && (
          <div style={{ padding: '6px 10px 10px', display: 'flex', flexDirection: 'column', gap: 8 }}>
            <div>
              <label style={{ fontSize: 10, color: '#718096', display: 'block', marginBottom: 3 }}>
                推理后端
              </label>
              <select
                value={inferenceConfig.backend}
                onChange={(e) => setInferenceConfig({ ...inferenceConfig, backend: e.target.value })}
                style={{
                  width: '100%',
                  background: '#2a2a3e',
                  border: '1px solid #444',
                  borderRadius: 4,
                  color: '#e2e8f0',
                  fontSize: 11,
                  padding: '4px 6px',
                  outline: 'none',
                }}
              >
                <option value="local">Local (FFI - stable-diffusion.cpp)</option>
                <option value="cli">CLI (sd-cli 子进程)</option>
                <option value="null">Null (无推理)</option>
              </select>
            </div>

            {inferenceConfig.backend === 'cli' && (
              <div>
                <label style={{ fontSize: 10, color: '#718096', display: 'block', marginBottom: 3 }}>
                  sd-cli 路径
                </label>
                <input
                  type="text"
                  value={inferenceConfig.sd_cli_path}
                  onChange={(e) => setInferenceConfig({ ...inferenceConfig, sd_cli_path: e.target.value })}
                  placeholder="/path/to/sd-cli"
                  style={{
                    width: '100%',
                    background: '#2a2a3e',
                    border: '1px solid #444',
                    borderRadius: 4,
                    color: '#e2e8f0',
                    fontSize: 11,
                    padding: '4px 6px',
                    outline: 'none',
                    boxSizing: 'border-box',
                  }}
                />
                <div style={{ fontSize: 9, color: '#555', marginTop: 2 }}>
                  sd-cli 可执行文件的路径，留空则使用 PATH 中的 sd-cli
                </div>
              </div>
            )}

            <div>
              <label style={{ fontSize: 10, color: '#718096', display: 'block', marginBottom: 3 }}>
                HuggingFace Token
              </label>
              <input
                type="password"
                value={inferenceConfig.hf_token}
                onChange={(e) => setInferenceConfig({ ...inferenceConfig, hf_token: e.target.value })}
                placeholder="hf_xxxxxxxxxxxxxxxxxxxxxxxx"
                style={{
                  width: '100%',
                  background: '#2a2a3e',
                  border: '1px solid #444',
                  borderRadius: 4,
                  color: '#e2e8f0',
                  fontSize: 11,
                  padding: '4px 6px',
                  outline: 'none',
                  boxSizing: 'border-box',
                }}
              />
              <div style={{ fontSize: 9, color: '#555', marginTop: 2 }}>
                用于下载 HuggingFace 受限模型，在
                <a href="https://huggingface.co/settings/tokens" target="_blank" rel="noreferrer" style={{ color: '#6b8dd6', textDecoration: 'none' }}>
                  hf.co/settings/tokens
                </a>
                获取
              </div>
            </div>

            <div>
              <label style={{ fontSize: 10, color: '#718096', display: 'block', marginBottom: 3 }}>
                线程数: {inferenceConfig.n_threads}
              </label>
              <input
                type="range"
                min={1}
                max={32}
                value={inferenceConfig.n_threads}
                onChange={(e) => setInferenceConfig({ ...inferenceConfig, n_threads: parseInt(e.target.value) })}
                style={{ width: '100%' }}
              />
            </div>

            <div style={{ display: 'flex', flexDirection: 'column', gap: 4 }}>
              <label style={{ fontSize: 10, color: '#718096', display: 'flex', alignItems: 'center', gap: 6, cursor: 'pointer' }}>
                <input
                  type="checkbox"
                  checked={inferenceConfig.flash_attn}
                  onChange={(e) => setInferenceConfig({ ...inferenceConfig, flash_attn: e.target.checked })}
                />
                Flash Attention
              </label>
              <label style={{ fontSize: 10, color: '#718096', display: 'flex', alignItems: 'center', gap: 6, cursor: 'pointer' }}>
                <input
                  type="checkbox"
                  checked={inferenceConfig.offload_params_to_cpu}
                  onChange={(e) => setInferenceConfig({ ...inferenceConfig, offload_params_to_cpu: e.target.checked })}
                />
                CPU Offload
              </label>
              <label style={{ fontSize: 10, color: '#718096', display: 'flex', alignItems: 'center', gap: 6, cursor: 'pointer' }}>
                <input
                  type="checkbox"
                  checked={inferenceConfig.enable_mmap}
                  onChange={(e) => setInferenceConfig({ ...inferenceConfig, enable_mmap: e.target.checked })}
                />
                Memory Map (mmap)
              </label>
            </div>

            <button
              onClick={handleSaveConfig}
              disabled={savingConfig}
              style={{
                background: savingConfig ? '#2a2a3e' : '#3b5998',
                border: 'none',
                borderRadius: 4,
                color: '#e2e8f0',
                fontSize: 11,
                padding: '5px 10px',
                cursor: savingConfig ? 'wait' : 'pointer',
                display: 'flex',
                alignItems: 'center',
                justifyContent: 'center',
                gap: 4,
                transition: 'background 0.2s',
              }}
            >
              <Save size={12} />
              {savingConfig ? '保存中...' : '保存设置'}
              {saveResult === 'success' && <CheckCircle size={12} style={{ color: '#68d391' }} />}
              {saveResult === 'error' && <AlertCircle size={12} style={{ color: '#fc8181' }} />}
            </button>

            {saveResult === 'success' && (
              <div style={{ fontSize: 9, color: '#68d391', textAlign: 'center' }}>
                设置已保存，重启服务后生效
              </div>
            )}
            {saveResult === 'error' && (
              <div style={{ fontSize: 9, color: '#fc8181', textAlign: 'center' }}>
                保存失败，请重试
              </div>
            )}
          </div>
        )}
      </div>

      <div style={{ borderBottom: '1px solid #333' }}>
        <div
          style={{
            padding: '6px 10px',
            fontSize: 11,
            fontWeight: 600,
            color: '#a0aec0',
            cursor: 'pointer',
            display: 'flex',
            alignItems: 'center',
            gap: 6,
            userSelect: 'none',
          }}
          onClick={() => setShowDownloads(!showDownloads)}
        >
          {showDownloads ? <ChevronDown size={12} /> : <ChevronRight size={12} />}
          <Download size={12} />
          <span>模型下载</span>
          <span style={{ fontSize: 9, color: '#555', marginLeft: 'auto', display: 'flex', alignItems: 'center', gap: 4 }}>
            {downloadList.length > 0 ? `${downloadList.length} 个模型` : ''}
            {showDownloads && (
              <button
                onClick={(e) => { e.stopPropagation(); setDownloadLoaded(false); }}
                disabled={downloadLoading}
                style={{
                  background: 'transparent',
                  border: 'none',
                  cursor: downloadLoading ? 'wait' : 'pointer',
                  color: '#718096',
                  padding: 0,
                  display: 'flex',
                  alignItems: 'center',
                }}
                title="刷新下载列表"
              >
                <RefreshCw size={11} style={{ animation: downloadLoading ? 'spin 1s linear infinite' : 'none' }} />
              </button>
            )}
          </span>
        </div>

        {showDownloads && (
          <div style={{ display: 'flex', flexDirection: 'column', gap: 6 }}>
            <div style={{ padding: '4px 10px' }}>
              <div
                style={{
                  display: 'flex',
                  alignItems: 'center',
                  gap: 6,
                  background: '#2a2a3e',
                  borderRadius: 6,
                  padding: '4px 8px',
                }}
              >
                <Filter size={11} style={{ color: '#718096', flexShrink: 0 }} />
                <input
                  type="text"
                  placeholder="搜索模型..."
                  value={downloadSearch}
                  onChange={(e) => setDownloadSearch(e.target.value)}
                  style={{
                    background: 'transparent',
                    border: 'none',
                    outline: 'none',
                    color: '#e2e8f0',
                    fontSize: 11,
                    width: '100%',
                  }}
                />
              </div>
            </div>

            <div style={{ padding: '0 10px', display: 'flex', flexWrap: 'wrap', gap: 3 }}>
              {['all', ...Array.from(new Set(downloadList.map((d) => d.category)))].map((cat) => (
                <button
                  key={cat}
                  onClick={() => setDownloadCategory(cat)}
                  style={{
                    background: downloadCategory === cat ? '#3b5998' : '#2a2a3e',
                    border: '1px solid',
                    borderColor: downloadCategory === cat ? '#5a7abf' : '#444',
                    borderRadius: 3,
                    color: downloadCategory === cat ? '#e2e8f0' : '#718096',
                    fontSize: 9,
                    padding: '2px 6px',
                    cursor: 'pointer',
                    transition: 'all 0.1s',
                  }}
                >
                  {cat === 'all' ? '全部' : cat}
                </button>
              ))}
            </div>

            <div style={{ maxHeight: 300, overflowY: 'auto', padding: '2px 0' }}>
              {downloadLoading && (
                <div style={{ padding: '8px 10px', fontSize: 10, color: '#718096', textAlign: 'center' }}>
                  加载中...
                </div>
              )}

              {!downloadLoading && (() => {
                const filtered = downloadList.filter((entry) => {
                  if (downloadCategory !== 'all' && entry.category !== downloadCategory) return false;
                  if (downloadSearch) {
                    const q = downloadSearch.toLowerCase();
                    return entry.name.toLowerCase().includes(q) ||
                      entry.description.toLowerCase().includes(q) ||
                      entry.category.toLowerCase().includes(q);
                  }
                  return true;
                });

                return filtered.map((entry, idx) => (
                  <div
                    key={idx}
                    style={{
                      padding: '5px 10px',
                      borderBottom: '1px solid #2a2a3e',
                    }}
                  >
                    <div style={{ display: 'flex', alignItems: 'center', gap: 4, marginBottom: 2 }}>
                      <span style={{ fontSize: 11, fontWeight: 600, color: '#e2e8f0', flex: 1 }}>
                        {entry.name}
                      </span>
                      <span
                        style={{
                          fontSize: 8,
                          padding: '1px 4px',
                          borderRadius: 2,
                          background: '#2a2a3e',
                          color: '#718096',
                          border: '1px solid #444',
                        }}
                      >
                        {entry.model_type}
                      </span>
                    </div>
                    <div style={{ fontSize: 9, color: '#718096', marginBottom: 4 }}>
                      {entry.description}
                    </div>
                    {entry.dependencies.length > 0 && (
                      <div style={{ fontSize: 9, color: '#555', marginBottom: 3 }}>
                        依赖: {entry.dependencies.join(', ')}
                      </div>
                    )}
                    <div style={{ display: 'flex', flexWrap: 'wrap', gap: 3 }}>
                      {entry.urls.map((urlObj, urlIdx) => {
                        const dlKey = `${urlObj.url}:${entry.model_type}`;
                        const isDownloading = downloadingUrls.has(dlKey);
                        return (
                          <div
                            key={urlIdx}
                            style={{
                              display: 'flex',
                              alignItems: 'center',
                              gap: 3,
                              background: '#2a2a3e',
                              borderRadius: 3,
                              padding: '2px 5px',
                              border: '1px solid #444',
                            }}
                          >
                            <span
                              style={{
                                fontSize: 8,
                                padding: '0px 3px',
                                borderRadius: 2,
                                background: urlObj.format === 'gguf' ? '#2d4a2d' : '#2d3a4a',
                                color: urlObj.format === 'gguf' ? '#68d391' : '#63b3ed',
                              }}
                            >
                              {urlObj.format}
                            </span>
                            <span style={{ fontSize: 9, color: '#a0aec0', maxWidth: 120, overflow: 'hidden', textOverflow: 'ellipsis', whiteSpace: 'nowrap' }}>
                              {urlObj.label}
                            </span>
                            <button
                              onClick={() => handleDownload(urlObj.url, entry.model_type)}
                              disabled={isDownloading}
                              title={isDownloading ? '下载中...' : `下载到 ${entry.model_type}`}
                              style={{
                                background: 'transparent',
                                border: 'none',
                                cursor: isDownloading ? 'wait' : 'pointer',
                                color: isDownloading ? '#68d391' : '#718096',
                                padding: 0,
                                display: 'flex',
                                alignItems: 'center',
                                transition: 'color 0.1s',
                              }}
                              onMouseEnter={(e) => {
                                if (!isDownloading) (e.currentTarget as HTMLElement).style.color = '#63b3ed';
                              }}
                              onMouseLeave={(e) => {
                                if (!isDownloading) (e.currentTarget as HTMLElement).style.color = '#718096';
                              }}
                            >
                              <Download size={10} />
                            </button>
                            <a
                              href={urlObj.url}
                              target="_blank"
                              rel="noopener noreferrer"
                              style={{
                                color: '#555',
                                display: 'flex',
                                alignItems: 'center',
                                transition: 'color 0.1s',
                              }}
                              title="在浏览器中打开"
                              onMouseEnter={(e) => {
                                (e.currentTarget as HTMLElement).style.color = '#718096';
                              }}
                              onMouseLeave={(e) => {
                                (e.currentTarget as HTMLElement).style.color = '#555';
                              }}
                            >
                              <ExternalLink size={9} />
                            </a>
                          </div>
                        );
                      })}
                    </div>
                  </div>
                ));
              })()}
            </div>
          </div>
        )}
      </div>

      <div style={{ flex: 1, overflowY: 'auto', padding: '4px 0' }}>
        {filteredTypes.map((type) => {
          const isCollapsed = collapsed[type];
          const files = getFilteredFiles(type);
          const count = getModelCount(type);
          const totalSize = getTotalSize(type);
          const icon = MODEL_TYPE_ICONS[type] || '📁';

          return (
            <div key={type}>
              <div
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
                onClick={() => toggleCategory(type)}
              >
                {isCollapsed ? <ChevronRight size={12} /> : <ChevronDown size={12} />}
                <span>{icon}</span>
                <span style={{ flex: 1 }}>{type.replace(/_/g, ' ')}</span>
                <span style={{ fontSize: 9, color: '#555', display: 'flex', alignItems: 'center', gap: 4 }}>
                  {count > 0 && <span>{count}</span>}
                  {totalSize > 0 && <span>{formatFileSize(totalSize)}</span>}
                </span>
              </div>

              {!isCollapsed && (
                <>
                  {files.map((file) => (
                    <div
                      key={file.path}
                      style={{
                        padding: '4px 10px 4px 28px',
                        fontSize: 11,
                        display: 'flex',
                        alignItems: 'center',
                        gap: 6,
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
                      <HardDrive size={11} style={{ color: '#555', flexShrink: 0 }} />
                      <div style={{ flex: 1, minWidth: 0 }}>
                        <div
                          style={{
                            overflow: 'hidden',
                            textOverflow: 'ellipsis',
                            whiteSpace: 'nowrap',
                          }}
                          title={file.path}
                        >
                          {file.name}
                        </div>
                        <div style={{ fontSize: 9, color: '#555', display: 'flex', gap: 8 }}>
                          <span>{formatFileSize(file.size)}</span>
                          {file.modified && <span>{formatDate(file.modified)}</span>}
                        </div>
                      </div>
                      <button
                        onClick={(e) => {
                          e.stopPropagation();
                          if (confirm(`Delete ${file.name}?`)) {
                            handleDelete(type, file.path);
                          }
                        }}
                        disabled={deletingPath === file.path}
                        style={{
                          background: 'transparent',
                          border: 'none',
                          cursor: deletingPath === file.path ? 'wait' : 'pointer',
                          color: '#718096',
                          padding: '2px',
                          display: 'flex',
                          alignItems: 'center',
                          borderRadius: 3,
                          flexShrink: 0,
                          opacity: 0.6,
                          transition: 'opacity 0.1s, color 0.1s',
                        }}
                        onMouseEnter={(e) => {
                          (e.currentTarget as HTMLElement).style.opacity = '1';
                          (e.currentTarget as HTMLElement).style.color = '#fc8181';
                        }}
                        onMouseLeave={(e) => {
                          (e.currentTarget as HTMLElement).style.opacity = '0.6';
                          (e.currentTarget as HTMLElement).style.color = '#718096';
                        }}
                        title="Delete model"
                      >
                        <Trash2 size={12} />
                      </button>
                    </div>
                  ))}

                  {files.length === 0 && (
                    <div
                      style={{
                        padding: '8px 28px',
                        fontSize: 10,
                        color: '#555',
                        display: 'flex',
                        alignItems: 'center',
                        gap: 4,
                      }}
                    >
                      <FolderOpen size={11} />
                      No models found
                    </div>
                  )}

                  <div
                    style={{
                      padding: '2px 10px 4px 28px',
                    }}
                  >
                    <button
                      onClick={() => handleUploadClick(type)}
                      style={{
                        background: '#2a2a3e',
                        border: '1px dashed #444',
                        borderRadius: 4,
                        color: '#718096',
                        fontSize: 10,
                        padding: '3px 8px',
                        cursor: 'pointer',
                        display: 'flex',
                        alignItems: 'center',
                        gap: 4,
                        width: '100%',
                        justifyContent: 'center',
                        transition: 'border-color 0.1s, color 0.1s',
                      }}
                      onMouseEnter={(e) => {
                        (e.currentTarget as HTMLElement).style.borderColor = '#5a6abf';
                        (e.currentTarget as HTMLElement).style.color = '#a0aec0';
                      }}
                      onMouseLeave={(e) => {
                        (e.currentTarget as HTMLElement).style.borderColor = '#444';
                        (e.currentTarget as HTMLElement).style.color = '#718096';
                      }}
                    >
                      <Upload size={10} />
                      Upload
                    </button>
                  </div>
                </>
              )}
            </div>
          );
        })}
      </div>

      <div
        style={{
          padding: '6px 10px',
          borderTop: '1px solid #333',
          fontSize: 10,
          color: '#555',
          display: 'flex',
          justifyContent: 'space-between',
        }}
      >
        <span>{MODEL_TYPES.length} categories</span>
        <span>
          {Object.values(models).reduce((sum, files) => sum + (files?.length || 0), 0)} models
        </span>
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

export { ModelManager };
