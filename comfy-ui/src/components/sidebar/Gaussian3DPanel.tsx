import React, { useState } from 'react';
import Gaussian3DViewer from '../viewer/Gaussian3DViewer';

interface Gaussian3DPanelProps {
  filePath: string;
  format?: 'ply' | 'splat';
  onClose?: () => void;
}

/**
 * 3D Gaussian Preview Panel
 * Full-featured panel with real-time Gaussian splat rendering
 */
const Gaussian3DPanel: React.FC<Gaussian3DPanelProps> = ({
  filePath,
  format = 'ply',
  onClose,
}) => {
  const [viewerKey, setViewerKey] = useState(0);

  const handleRefresh = () => {
    setViewerKey((k) => k + 1);
  };

  return (
    <div
      style={{
        display: 'flex',
        flexDirection: 'column',
        height: '100%',
        background: '#1a1a2e',
      }}
    >
      {/* Header bar */}
      <div
        style={{
          display: 'flex',
          justifyContent: 'space-between',
          alignItems: 'center',
          padding: '10px 14px',
          background: '#16213e',
          borderBottom: '1px solid #0f3460',
          flexShrink: 0,
        }}
      >
        <div style={{ display: 'flex', alignItems: 'center', gap: '8px' }}>
          <span style={{ fontSize: '16px' }}>🎮</span>
          <h3 style={{ margin: 0, color: '#e2e2e2', fontSize: '13px', fontWeight: 600 }}>
            3D Gaussian Viewer
          </h3>
          <span
            style={{
              fontSize: '10px',
              color: '#00e676',
              background: 'rgba(0,230,118,0.1)',
              padding: '2px 6px',
              borderRadius: '4px',
            }}
          >
            {format.toUpperCase()}
          </span>
        </div>
        <div style={{ display: 'flex', gap: '6px' }}>
          <button
            onClick={handleRefresh}
            style={{
              background: '#0f3460',
              border: '1px solid #1a4a8a',
              color: '#e2e2e2',
              padding: '4px 8px',
              borderRadius: '4px',
              cursor: 'pointer',
              fontSize: '12px',
            }}
            title="Reload viewer"
          >
            ↻
          </button>
          {onClose && (
            <button
              onClick={onClose}
              style={{
                background: '#0f3460',
                border: '1px solid #1a4a8a',
                color: '#e2e2e2',
                padding: '4px 8px',
                borderRadius: '4px',
                cursor: 'pointer',
                fontSize: '12px',
              }}
              title="Close"
            >
              ✕
            </button>
          )}
        </div>
      </div>

      {/* 3D Viewer - fills remaining space */}
      <div style={{ flex: 1, minHeight: 0, position: 'relative' }}>
        <Gaussian3DViewer
          key={viewerKey}
          filePath={filePath}
          format={format}
        />
      </div>

      {/* File info footer */}
      <div
        style={{
          padding: '6px 14px',
          background: '#16213e',
          borderTop: '1px solid #0f3460',
          fontSize: '11px',
          color: '#a2a2b2',
          flexShrink: 0,
        }}
      >
        <div
          style={{
            overflow: 'hidden',
            textOverflow: 'ellipsis',
            whiteSpace: 'nowrap',
          }}
          title={filePath}
        >
          {filePath}
        </div>
      </div>
    </div>
  );
};

export default Gaussian3DPanel;
