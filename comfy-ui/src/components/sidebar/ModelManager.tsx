import { useState, useEffect, useRef, type FC } from 'react';
import { Search, ChevronRight, ChevronDown, Trash2, Upload, RefreshCw, FolderOpen, HardDrive } from 'lucide-react';
import { useModelManagerStore, MODEL_TYPES } from '@/store/models';
import type { ModelFileInfo } from '@/types/api';

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

  useEffect(() => {
    loadModels();
  }, [loadModels]);

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
