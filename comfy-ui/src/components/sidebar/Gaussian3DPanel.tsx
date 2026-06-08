import React, { useState } from 'react';
import Gaussian3DViewer from '../viewer/Gaussian3DViewer';

interface Gaussian3DPanelProps {
  filePath: string;
  format?: 'ply' | 'splat';
  onClose?: () => void;
}

/**
 * 3D Gaussian Preview Panel
 * Displays in the sidebar with the 3D viewer
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
      {/* Header */}
      <div
        style={{
          display: 'flex',
          justifyContent: 'space-between',
          alignItems: 'center',
          padding: '12px 16px',
          background: '#16213e',
          borderBottom: '1px solid #0f3460',
        }}
      >
        <h3 style={{ margin: 0, color: '#e2e2e2', fontSize: '14px', fontWeight: 600 }}>
          3D Gaussian Preview
        </h3>
        <div style={{ display: 'flex', gap: '8px' }}>
          <button
            onClick={handleRefresh}
            style={{
              background: '#0f3460',
              border: 'none',
              color: '#e2e2e2',
              padding: '4px 8px',
              borderRadius: '4px',
              cursor: 'pointer',
              fontSize: '12px',
            }}
            title="Refresh"
          >
            ↻
          </button>
          {onClose && (
            <button
              onClick={onClose}
              style={{
                background: '#0f3460',
                border: 'none',
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

      {/* Viewer */}
      <div style={{ flex: 1, minHeight: 0 }}>
        <Gaussian3DViewer
          key={viewerKey}
          filePath={filePath}
          format={format}
          width={400}
          height={500}
        />
      </div>

      {/* File info */}
      <div
        style={{
          padding: '8px 16px',
          background: '#16213e',
          borderTop: '1px solid #0f3460',
          fontSize: '11px',
          color: '#a2a2b2',
        }}
      >
        <div style={{ wordBreak: 'break-all' }}>{filePath}</div>
        <div style={{ marginTop: '4px', textTransform: 'uppercase' }}>{format}</div>
      </div>
    </div>
  );
};

export default Gaussian3DPanel;
