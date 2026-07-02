import { useState, useEffect, useMemo, useRef, type FC } from 'react';
import { api } from '@/api/client';
import type {
  AssetRecord,
  CustomFolder,
  AssetSource,
  AssetType,
} from '@/types/api';
import {
  Search,
  RefreshCw,
  Upload,
  Trash2,
  FolderPlus,
  ScanLine,
  ChevronRight,
  ChevronDown,
  Image as ImageIcon,
  Film,
  Music,
  Box,
  Folder,
  HardDrive,
  X,
  Download,
  Eye,
} from 'lucide-react';

type FilterMode =
  | { kind: 'all' }
  | { kind: 'source'; source: AssetSource }
  | { kind: 'type'; source: AssetSource; assetType: AssetType }
  | { kind: 'folder'; folderId: number };

const TYPE_ICONS: Record<string, typeof ImageIcon> = {
  image: ImageIcon,
  video: Film,
  audio: Music,
  '3d': Box,
};

function formatFileSize(bytes: number): string {
  if (bytes < 1024) return `${bytes} B`;
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
  if (bytes < 1024 * 1024 * 1024) return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
  return `${(bytes / (1024 * 1024 * 1024)).toFixed(2)} GB`;
}

const AssetManager: FC = () => {
  const [assets, setAssets] = useState<AssetRecord[]>([]);
  const [folders, setFolders] = useState<CustomFolder[]>([]);
  const [loading, setLoading] = useState(false);
  const [search, setSearch] = useState('');
  const [filterMode, setFilterMode] = useState<FilterMode>({ kind: 'all' });
  const [collapsed, setCollapsed] = useState<Record<string, boolean>>({});
  const [selectedIds, setSelectedIds] = useState<Set<number>>(new Set());
  const [selectedAsset, setSelectedAsset] = useState<AssetRecord | null>(null);
  const [previewAsset, setPreviewAsset] = useState<AssetRecord | null>(null);
  const [uploading, setUploading] = useState(false);
  const [showNewFolder, setShowNewFolder] = useState(false);
  const [newFolderName, setNewFolderName] = useState('');
  const fileInputRef = useRef<HTMLInputElement>(null);

  const loadData = async () => {
    setLoading(true);
    try {
      const [assetResult, folderResult] = await Promise.all([
        api.listAssets({ limit: 1000 }),
        api.listAssetFolders(),
      ]);
      setAssets(assetResult.assets);
      setFolders(folderResult.folders);
    } catch (err) {
      console.error('Failed to load assets:', err);
    } finally {
      setLoading(false);
    }
  };

  useEffect(() => {
    loadData();
  }, []);

  const filteredAssets = useMemo(() => {
    let result = assets;
    if (filterMode.kind === 'source') {
      result = result.filter((a) => a.source === filterMode.source);
    } else if (filterMode.kind === 'type') {
      result = result.filter(
        (a) => a.source === filterMode.source && a.asset_type === filterMode.assetType
      );
    } else if (filterMode.kind === 'folder') {
      result = result.filter((a) => a.custom_folder_id === filterMode.folderId);
    }
    if (search) {
      const q = search.toLowerCase();
      result = result.filter(
        (a) =>
          a.name.toLowerCase().includes(q) ||
          a.tags.some((t) => t.toLowerCase().includes(q))
      );
    }
    return result;
  }, [assets, filterMode, search]);

  // Compute counts per category
  const counts = useMemo(() => {
    const c = {
      all: assets.length,
      uploaded: 0,
      generated: 0,
      uploaded_image: 0,
      uploaded_video: 0,
      uploaded_audio: 0,
      uploaded_3d: 0,
      generated_image: 0,
      generated_video: 0,
      generated_audio: 0,
      generated_3d: 0,
    };
    for (const a of assets) {
      if (a.source === 'uploaded') {
        c.uploaded++;
        (c as any)[`uploaded_${a.asset_type}`]++;
      } else {
        c.generated++;
        (c as any)[`generated_${a.asset_type}`]++;
      }
    }
    return c;
  }, [assets]);

  const handleUpload = async (e: React.ChangeEvent<HTMLInputElement>) => {
    const files = e.target.files;
    if (!files || files.length === 0) return;
    setUploading(true);
    try {
      for (const file of Array.from(files)) {
        await api.uploadAsset(file);
      }
      await loadData();
    } catch (err) {
      console.error('Upload failed:', err);
    } finally {
      setUploading(false);
      if (fileInputRef.current) fileInputRef.current.value = '';
    }
  };

  const handleDelete = async (id: number) => {
    try {
      await api.deleteAsset(id);
      setAssets((prev) => prev.filter((a) => a.id !== id));
      if (selectedAsset?.id === id) setSelectedAsset(null);
      setSelectedIds((prev) => {
        const next = new Set(prev);
        next.delete(id);
        return next;
      });
    } catch (err) {
      console.error('Delete failed:', err);
    }
  };

  const handleScan = async () => {
    try {
      const result = await api.scanAssets();
      console.log(`Scan complete: ${result.new_assets} new assets`);
      await loadData();
    } catch (err) {
      console.error('Scan failed:', err);
    }
  };

  const handleCreateFolder = async () => {
    if (!newFolderName.trim()) return;
    try {
      await api.createAssetFolder({ name: newFolderName.trim() });
      setNewFolderName('');
      setShowNewFolder(false);
      await loadData();
    } catch (err) {
      console.error('Create folder failed:', err);
    }
  };

  const handleDeleteFolder = async (id: number) => {
    try {
      await api.deleteAssetFolder(id);
      await loadData();
      if (filterMode.kind === 'folder' && filterMode.folderId === id) {
        setFilterMode({ kind: 'all' });
      }
    } catch (err) {
      console.error('Delete folder failed:', err);
    }
  };

  const handleMoveToFolder = async (assetId: number, folderId: number | null) => {
    try {
      await api.updateAsset(assetId, { custom_folder_id: folderId });
      await loadData();
    } catch (err) {
      console.error('Move to folder failed:', err);
    }
  };

  const toggleSelect = (id: number) => {
    setSelectedIds((prev) => {
      const next = new Set(prev);
      if (next.has(id)) next.delete(id);
      else next.add(id);
      return next;
    });
  };

  const toggleCategory = (key: string) => {
    setCollapsed((prev) => ({ ...prev, [key]: !prev[key] }));
  };

  const sidebarBtn = (label: string, active: boolean, icon: React.ReactNode, count: number, onClick: () => void) => (
    <div
      onClick={onClick}
      style={{
        padding: '4px 10px',
        fontSize: 11,
        cursor: 'pointer',
        display: 'flex',
        alignItems: 'center',
        gap: 6,
        background: active ? '#2a2a3e' : 'transparent',
        borderRadius: 3,
        margin: '1px 6px',
        transition: 'background 0.1s',
      }}
      onMouseEnter={(e) => { if (!active) (e.currentTarget as HTMLElement).style.background = '#222233'; }}
      onMouseLeave={(e) => { if (!active) (e.currentTarget as HTMLElement).style.background = 'transparent'; }}
    >
      {icon}
      <span style={{ flex: 1 }}>{label}</span>
      {count > 0 && <span style={{ fontSize: 9, color: '#555' }}>{count}</span>}
    </div>
  );

  const typeIcon = (type: string) => {
    const Icon = TYPE_ICONS[type] || ImageIcon;
    return <Icon size={11} style={{ color: '#555', flexShrink: 0 }} />;
  };

  const renderCategory = (
    key: string,
    label: string,
    icon: React.ReactNode,
    count: number,
    children?: React.ReactNode
  ) => {
    const isCollapsed = collapsed[key];
    return (
      <div key={key}>
        <div
          onClick={() => toggleCategory(key)}
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
          {icon}
          <span style={{ flex: 1 }}>{label}</span>
          {count > 0 && <span style={{ fontSize: 9, color: '#555' }}>{count}</span>}
        </div>
        {!isCollapsed && children}
      </div>
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
        onChange={handleUpload}
        multiple
        accept="image/*,video/*,audio/*,.ply,.splat"
      />

      {/* Search bar */}
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
            placeholder="Search assets..."
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
            onClick={() => loadData()}
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
        <div style={{ display: 'flex', gap: 4, marginTop: 6 }}>
          <button
            onClick={() => fileInputRef.current?.click()}
            disabled={uploading}
            style={{
              flex: 1,
              background: '#2a4a2a',
              border: '1px solid #3a6a3a',
              borderRadius: 4,
              color: '#e2e8f0',
              padding: '4px 8px',
              fontSize: 11,
              cursor: uploading ? 'wait' : 'pointer',
              display: 'flex',
              alignItems: 'center',
              justifyContent: 'center',
              gap: 4,
            }}
          >
            <Upload size={12} />
            {uploading ? 'Uploading...' : 'Upload'}
          </button>
          <button
            onClick={handleScan}
            style={{
              background: '#2a2a4e',
              border: '1px solid #3a3a6e',
              borderRadius: 4,
              color: '#e2e8f0',
              padding: '4px 8px',
              fontSize: 11,
              cursor: 'pointer',
              display: 'flex',
              alignItems: 'center',
              gap: 4,
            }}
            title="Scan files & sync"
          >
            <ScanLine size={12} />
          </button>
          <button
            onClick={() => setShowNewFolder(!showNewFolder)}
            style={{
              background: '#3a2a4e',
              border: '1px solid #5a3a6e',
              borderRadius: 4,
              color: '#e2e8f0',
              padding: '4px 8px',
              fontSize: 11,
              cursor: 'pointer',
              display: 'flex',
              alignItems: 'center',
            }}
            title="New folder"
          >
            <FolderPlus size={12} />
          </button>
        </div>
        {showNewFolder && (
          <div style={{ display: 'flex', gap: 4, marginTop: 6 }}>
            <input
              type="text"
              placeholder="Folder name..."
              value={newFolderName}
              onChange={(e) => setNewFolderName(e.target.value)}
              onKeyDown={(e) => e.key === 'Enter' && handleCreateFolder()}
              style={{
                flex: 1,
                background: '#2a2a3e',
                border: '1px solid #444',
                borderRadius: 4,
                color: '#e2e8f0',
                padding: '4px 8px',
                fontSize: 11,
                outline: 'none',
              }}
            />
            <button
              onClick={handleCreateFolder}
              style={{
                background: '#3a6a3a',
                border: 'none',
                borderRadius: 4,
                color: '#e2e8f0',
                padding: '4px 8px',
                fontSize: 11,
                cursor: 'pointer',
              }}
            >
              OK
            </button>
          </div>
        )}
      </div>

      {/* Sidebar tree */}
      <div style={{ flex: 1, overflowY: 'auto', padding: '4px 0' }}>
        {sidebarBtn(
          'All Assets',
          filterMode.kind === 'all',
          <HardDrive size={11} style={{ color: '#555' }} />,
          counts.all,
          () => setFilterMode({ kind: 'all' })
        )}

        {renderCategory(
          'uploaded',
          'Uploaded',
          <Upload size={11} style={{ color: '#555' }} />,
          counts.uploaded,
          <>
            {sidebarBtn('Images', filterMode.kind === 'type' && filterMode.source === 'uploaded' && filterMode.assetType === 'image', typeIcon('image'), counts.uploaded_image, () => setFilterMode({ kind: 'type', source: 'uploaded', assetType: 'image' }))}
            {sidebarBtn('Videos', filterMode.kind === 'type' && filterMode.source === 'uploaded' && filterMode.assetType === 'video', typeIcon('video'), counts.uploaded_video, () => setFilterMode({ kind: 'type', source: 'uploaded', assetType: 'video' }))}
            {sidebarBtn('Audios', filterMode.kind === 'type' && filterMode.source === 'uploaded' && filterMode.assetType === 'audio', typeIcon('audio'), counts.uploaded_audio, () => setFilterMode({ kind: 'type', source: 'uploaded', assetType: 'audio' }))}
            {sidebarBtn('3D Models', filterMode.kind === 'type' && filterMode.source === 'uploaded' && filterMode.assetType === '3d', typeIcon('3d'), counts.uploaded_3d, () => setFilterMode({ kind: 'type', source: 'uploaded', assetType: '3d' }))}
          </>
        )}

        {renderCategory(
          'generated',
          'AI Generated',
          <RefreshCw size={11} style={{ color: '#555' }} />,
          counts.generated,
          <>
            {sidebarBtn('Images', filterMode.kind === 'type' && filterMode.source === 'generated' && filterMode.assetType === 'image', typeIcon('image'), counts.generated_image, () => setFilterMode({ kind: 'type', source: 'generated', assetType: 'image' }))}
            {sidebarBtn('Videos', filterMode.kind === 'type' && filterMode.source === 'generated' && filterMode.assetType === 'video', typeIcon('video'), counts.generated_video, () => setFilterMode({ kind: 'type', source: 'generated', assetType: 'video' }))}
            {sidebarBtn('Audios', filterMode.kind === 'type' && filterMode.source === 'generated' && filterMode.assetType === 'audio', typeIcon('audio'), counts.generated_audio, () => setFilterMode({ kind: 'type', source: 'generated', assetType: 'audio' }))}
            {sidebarBtn('3D Models', filterMode.kind === 'type' && filterMode.source === 'generated' && filterMode.assetType === '3d', typeIcon('3d'), counts.generated_3d, () => setFilterMode({ kind: 'type', source: 'generated', assetType: '3d' }))}
          </>
        )}

        {folders.length > 0 && renderCategory(
          'folders',
          'Custom Folders',
          <Folder size={11} style={{ color: '#555' }} />,
          folders.length,
          folders.map((f) => (
            <div
              key={f.id}
              style={{ display: 'flex', alignItems: 'center' }}
            >
              <div style={{ flex: 1 }}>
                {sidebarBtn(
                  f.name,
                  filterMode.kind === 'folder' && filterMode.folderId === f.id,
                  <Folder size={11} style={{ color: f.color || '#555' }} />,
                  assets.filter((a) => a.custom_folder_id === f.id).length,
                  () => setFilterMode({ kind: 'folder', folderId: f.id })
                )}
              </div>
              <button
                onClick={(e) => { e.stopPropagation(); handleDeleteFolder(f.id); }}
                style={{
                  background: 'transparent',
                  border: 'none',
                  cursor: 'pointer',
                  color: '#555',
                  padding: '2px 6px',
                  fontSize: 10,
                  flexShrink: 0,
                }}
                title="Delete folder"
              >
                x
              </button>
            </div>
          ))
        )}
      </div>

      {/* Asset grid */}
      <div style={{ flex: 1, overflowY: 'auto', borderTop: '1px solid #333' }}>
        {filteredAssets.length === 0 && !loading && (
          <div style={{ padding: '20px', textAlign: 'center', color: '#555', fontSize: 12 }}>
            No assets found. Upload files or run a workflow to generate assets.
          </div>
        )}
        <div
          style={{
            display: 'grid',
            gridTemplateColumns: 'repeat(3, 1fr)',
            gap: '4px',
            padding: '6px',
          }}
        >
          {filteredAssets.map((asset) => {
            const isSelected = selectedIds.has(asset.id);
            const url = api.getAssetUrl(asset.relative_path);
            const Icon = TYPE_ICONS[asset.asset_type] || ImageIcon;
            return (
              <div
                key={asset.id}
                onClick={() => { toggleSelect(asset.id); setSelectedAsset(asset); }}
                onDoubleClick={() => setPreviewAsset(asset)}
                onContextMenu={(e) => { e.preventDefault(); setSelectedAsset(asset); }}
                style={{
                  cursor: 'pointer',
                  border: isSelected ? '2px solid #4a9eff' : '1px solid #333',
                  borderRadius: 4,
                  overflow: 'hidden',
                  position: 'relative',
                  aspectRatio: '1',
                  background: '#0f1117',
                }}
              >
                {asset.asset_type === 'image' || asset.asset_type === 'video' ? (
                  asset.asset_type === 'image' ? (
                    <img
                      src={url}
                      alt={asset.name}
                      loading="lazy"
                      style={{ width: '100%', height: '100%', objectFit: 'cover', display: 'block' }}
                    />
                  ) : (
                    <video
                      src={url}
                      muted
                      style={{ width: '100%', height: '100%', objectFit: 'cover', display: 'block' }}
                    />
                  )
                ) : (
                  <div style={{
                    display: 'flex',
                    alignItems: 'center',
                    justifyContent: 'center',
                    height: '100%',
                    flexDirection: 'column',
                    gap: 4,
                  }}>
                    <Icon size={28} style={{ color: '#4a5568' }} />
                    <span style={{ fontSize: 8, color: '#555', textTransform: 'uppercase' }}>
                      {asset.asset_type}
                    </span>
                  </div>
                )}
                <div style={{
                  position: 'absolute',
                  bottom: 0,
                  left: 0,
                  right: 0,
                  background: 'rgba(0,0,0,0.7)',
                  color: '#aaa',
                  fontSize: 8,
                  padding: '2px 4px',
                  whiteSpace: 'nowrap',
                  overflow: 'hidden',
                  textOverflow: 'ellipsis',
                }}>
                  {asset.name}
                </div>
                <div style={{
                  position: 'absolute',
                  top: 2,
                  right: 2,
                  background: asset.source === 'uploaded' ? 'rgba(72,187,120,0.8)' : 'rgba(160,120,200,0.8)',
                  borderRadius: 2,
                  padding: '0 3px',
                  fontSize: 7,
                  color: '#fff',
                }}>
                  {asset.source === 'uploaded' ? 'UP' : 'AI'}
                </div>
              </div>
            );
          })}
        </div>
      </div>

      {/* Detail panel */}
      {selectedAsset && (
        <div style={{
          borderTop: '1px solid #333',
          padding: '8px',
          background: '#16161e',
          maxHeight: '200px',
          overflowY: 'auto',
        }}>
          <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', marginBottom: 6 }}>
            <span style={{ fontSize: 11, fontWeight: 600, color: '#e2e8f0' }}>
              {selectedAsset.name}
            </span>
            <button
              onClick={() => setPreviewAsset(selectedAsset)}
              style={{
                background: '#2a3a4a',
                border: '1px solid #3a5a6a',
                borderRadius: 3,
                color: '#63b3ed',
                padding: '2px 6px',
                fontSize: 10,
                cursor: 'pointer',
                display: 'flex',
                alignItems: 'center',
                gap: 3,
              }}
            >
              <Eye size={10} />
              Preview
            </button>
            <button
              onClick={() => handleDelete(selectedAsset.id)}
              style={{
                background: '#4a2020',
                border: '1px solid #6a3030',
                borderRadius: 3,
                color: '#fc8181',
                padding: '2px 6px',
                fontSize: 10,
                cursor: 'pointer',
                display: 'flex',
                alignItems: 'center',
                gap: 3,
              }}
            >
              <Trash2 size={10} />
              Delete
            </button>
          </div>
          <div style={{ fontSize: 10, color: '#718096', lineHeight: 1.6 }}>
            <div>Type: <span style={{ color: '#a0aec0' }}>{selectedAsset.asset_type}</span></div>
            <div>Source: <span style={{ color: '#a0aec0' }}>{selectedAsset.source}</span></div>
            <div>Size: <span style={{ color: '#a0aec0' }}>{formatFileSize(selectedAsset.file_size)}</span></div>
            <div>Path: <span style={{ color: '#a0aec0', wordBreak: 'break-all' }}>{selectedAsset.relative_path}</span></div>
            <div>Created: <span style={{ color: '#a0aec0' }}>{selectedAsset.created_at}</span></div>
            {selectedAsset.tags.length > 0 && (
              <div>Tags: {selectedAsset.tags.map((t) => (
                <span key={t} style={{
                  display: 'inline-block',
                  background: '#2a2a3e',
                  borderRadius: 2,
                  padding: '1px 4px',
                  margin: '1px',
                  fontSize: 9,
                  color: '#a0aec0',
                }}>{t}</span>
              ))}</div>
            )}
          </div>
          {folders.length > 0 && (
            <div style={{ marginTop: 6, display: 'flex', alignItems: 'center', gap: 4 }}>
              <span style={{ fontSize: 10, color: '#718096' }}>Move to:</span>
              <select
                value={selectedAsset.custom_folder_id ?? ''}
                onChange={(e) => {
                  const val = e.target.value;
                  handleMoveToFolder(selectedAsset.id, val ? parseInt(val, 10) : null);
                }}
                style={{
                  background: '#2a2a3e',
                  border: '1px solid #444',
                  borderRadius: 3,
                  color: '#e2e8f0',
                  padding: '2px 4px',
                  fontSize: 10,
                  outline: 'none',
                  flex: 1,
                }}
              >
                <option value="">No folder</option>
                {folders.map((f) => (
                  <option key={f.id} value={f.id}>{f.name}</option>
                ))}
              </select>
            </div>
          )}
        </div>
      )}

      {/* Preview Modal */}
      {previewAsset && (
        <div
          onClick={() => setPreviewAsset(null)}
          style={{
            position: 'fixed',
            top: 0,
            left: 0,
            right: 0,
            bottom: 0,
            background: 'rgba(0, 0, 0, 0.85)',
            zIndex: 10000,
            display: 'flex',
            alignItems: 'center',
            justifyContent: 'center',
            flexDirection: 'column',
            gap: 12,
          }}
        >
          <div
            onClick={(e) => e.stopPropagation()}
            style={{
              position: 'relative',
              maxWidth: '90vw',
              maxHeight: '85vh',
              display: 'flex',
              flexDirection: 'column',
              alignItems: 'center',
              gap: 12,
            }}
          >
            {/* Close button */}
            <button
              onClick={() => setPreviewAsset(null)}
              style={{
                position: 'absolute',
                top: -40,
                right: 0,
                background: 'transparent',
                border: '1px solid #444',
                borderRadius: 4,
                color: '#e2e8f0',
                padding: '4px 8px',
                cursor: 'pointer',
                display: 'flex',
                alignItems: 'center',
                gap: 4,
                fontSize: 12,
              }}
            >
              <X size={14} />
              Close
            </button>

            {/* Preview content */}
            {previewAsset.asset_type === 'image' && (
              <img
                src={api.getAssetUrl(previewAsset.relative_path)}
                alt={previewAsset.name}
                style={{
                  maxWidth: '90vw',
                  maxHeight: '75vh',
                  objectFit: 'contain',
                  borderRadius: 4,
                }}
              />
            )}
            {previewAsset.asset_type === 'video' && (
              <video
                src={api.getAssetUrl(previewAsset.relative_path)}
                controls
                autoPlay
                style={{
                  maxWidth: '90vw',
                  maxHeight: '75vh',
                  borderRadius: 4,
                }}
              />
            )}
            {previewAsset.asset_type === 'audio' && (
              <div style={{
                display: 'flex',
                flexDirection: 'column',
                alignItems: 'center',
                gap: 16,
                padding: '40px',
              }}>
                <Music size={64} style={{ color: '#4a5568' }} />
                <audio
                  src={api.getAssetUrl(previewAsset.relative_path)}
                  controls
                  autoPlay
                  style={{ width: '400px' }}
                />
              </div>
            )}
            {previewAsset.asset_type === '3d' && (
              <div style={{
                display: 'flex',
                flexDirection: 'column',
                alignItems: 'center',
                gap: 16,
                padding: '40px',
                color: '#a0aec0',
              }}>
                <Box size={64} style={{ color: '#4a5568' }} />
                <p style={{ fontSize: 13 }}>3D model preview is not available</p>
              </div>
            )}

            {/* Info bar */}
            <div style={{
              display: 'flex',
              alignItems: 'center',
              gap: 16,
              color: '#a0aec0',
              fontSize: 12,
            }}>
              <span>{previewAsset.name}</span>
              <span style={{ color: '#555' }}>•</span>
              <span>{previewAsset.asset_type}</span>
              <span style={{ color: '#555' }}>•</span>
              <span>{formatFileSize(previewAsset.file_size)}</span>
              <a
                href={api.getAssetUrl(previewAsset.relative_path)}
                download={previewAsset.name}
                style={{
                  color: '#63b3ed',
                  textDecoration: 'none',
                  display: 'flex',
                  alignItems: 'center',
                  gap: 4,
                }}
              >
                <Download size={12} />
                Download
              </a>
            </div>
          </div>
        </div>
      )}
    </div>
  );
};

export default AssetManager;
