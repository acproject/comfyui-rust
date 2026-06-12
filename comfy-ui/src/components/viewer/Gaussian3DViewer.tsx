import React, { useRef, useEffect, useState } from 'react';
import * as GaussianSplats3D from '@mkkellogg/gaussian-splats-3d';

interface Gaussian3DViewerProps {
  filePath: string;
  format?: 'ply' | 'splat';
}

/**
 * 3D Gaussian Splat Viewer - Real-time Gaussian splatting renderer
 * Uses @mkkellogg/gaussian-splats-3d for production-quality rendering
 * Similar to SuperSplat / Super GLB Viewer experience
 */
const Gaussian3DViewer: React.FC<Gaussian3DViewerProps> = ({
  filePath,
  format = 'ply',
}) => {
  const containerRef = useRef<HTMLDivElement>(null);
  const viewerRef = useRef<GaussianSplats3D.Viewer | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [splatCount, setSplatCount] = useState(0);
  const [fps, setFps] = useState(0);

  // Initialize viewer
  useEffect(() => {
    if (!containerRef.current) return;

    const viewer = new GaussianSplats3D.Viewer({
      rootElement: containerRef.current,
      cameraUp: [0, 1, 0],
      initialCameraPosition: [0, 1.5, 3],
      initialCameraLookAt: [0, 0, 0],
      sharedMemoryForWorkers: false,
      integerBasedSort: true,
      halfPrecisionCovariancesOnGPU: true,
      dynamicScene: false,
      gaussianSphereColors: {
        0: [0.5, 0.5, 0.5],
        1: [0.5, 0.5, 0.5],
        2: [0.5, 0.5, 0.5],
        3: [0.5, 0.5, 0.5],
      },
    });

    viewerRef.current = viewer;

    return () => {
      viewer.dispose();
      viewerRef.current = null;
    };
  }, []);

  // Load scene when file path changes
  useEffect(() => {
    if (!viewerRef.current || !filePath) return;

    setLoading(true);
    setError(null);

    const loadScene = async () => {
      try {
        const viewer = viewerRef.current;
        if (!viewer) return;

        // Determine format from file extension
        const actualFormat = format === 'splat' ? GaussianSplats3D.SceneFormat.Splat : GaussianSplats3D.SceneFormat.PLY;

        await viewer.addSplatScene(filePath, {
          splatAlphaRemovalThreshold: 5,
          showLoadingUI: false,
          position: [0, 0, 0],
          rotation: [0, 0, 0],
          scale: [1, 1, 1],
          format: actualFormat,
        });

        viewer.start();

        // Get splat count
        const count = viewer.getSplatCount();
        setSplatCount(count);
        setLoading(false);
      } catch (err) {
        console.error('Error loading 3D Gaussians:', err);
        setError(err instanceof Error ? err.message : 'Unknown error');
        setLoading(false);
      }
    };

    loadScene();
  }, [filePath, format]);

  // FPS counter
  useEffect(() => {
    let frameCount = 0;
    let lastTime = performance.now();
    let animId: number;

    const updateFps = () => {
      frameCount++;
      const now = performance.now();
      if (now - lastTime >= 1000) {
        setFps(frameCount);
        frameCount = 0;
        lastTime = now;
      }
      animId = requestAnimationFrame(updateFps);
    };

    animId = requestAnimationFrame(updateFps);
    return () => cancelAnimationFrame(animId);
  }, []);

  return (
    <div
      style={{
        width: '100%',
        height: '100%',
        position: 'relative',
        background: '#1a1a2e',
      }}
    >
      {/* Viewer container */}
      <div
        ref={containerRef}
        style={{
          width: '100%',
          height: '100%',
        }}
      />

      {/* Loading overlay */}
      {loading && (
        <div
          style={{
            position: 'absolute',
            top: 0,
            left: 0,
            right: 0,
            bottom: 0,
            display: 'flex',
            alignItems: 'center',
            justifyContent: 'center',
            background: 'rgba(0, 0, 0, 0.6)',
            zIndex: 10,
          }}
        >
          <div style={{ textAlign: 'center', color: 'white' }}>
            <div
              style={{
                width: 40,
                height: 40,
                border: '3px solid rgba(255,255,255,0.2)',
                borderTopColor: '#4a9eff',
                borderRadius: '50%',
                animation: 'spin 1s linear infinite',
                margin: '0 auto 12px',
              }}
            />
            <div style={{ fontSize: '14px' }}>Loading 3D Gaussians...</div>
            <div style={{ fontSize: '11px', opacity: 0.6, marginTop: '4px' }}>
              Parsing and sorting splat data
            </div>
          </div>
        </div>
      )}

      {/* Error overlay */}
      {error && (
        <div
          style={{
            position: 'absolute',
            top: 0,
            left: 0,
            right: 0,
            bottom: 0,
            display: 'flex',
            alignItems: 'center',
            justifyContent: 'center',
            background: 'rgba(0, 0, 0, 0.7)',
            zIndex: 10,
            padding: '20px',
          }}
        >
          <div style={{ color: '#ff6b6b', fontSize: '14px', textAlign: 'center' }}>
            <div style={{ fontWeight: 'bold', marginBottom: '8px' }}>Failed to load 3D Gaussians</div>
            <div style={{ fontSize: '12px', opacity: 0.8 }}>{error}</div>
          </div>
        </div>
      )}

      {/* Info overlay - top left */}
      {!loading && splatCount > 0 && (
        <div
          style={{
            position: 'absolute',
            top: '10px',
            left: '10px',
            color: 'white',
            fontSize: '12px',
            background: 'rgba(0, 0, 0, 0.6)',
            padding: '8px 12px',
            borderRadius: '6px',
            zIndex: 5,
            pointerEvents: 'none',
            backdropFilter: 'blur(4px)',
          }}
        >
          <div style={{ fontWeight: 600, marginBottom: '4px' }}>3D Gaussian Splat</div>
          <div>{splatCount.toLocaleString()} splats</div>
          <div style={{ opacity: 0.7, marginTop: '2px' }}>{fps} FPS</div>
        </div>
      )}

      {/* Controls hint - bottom */}
      {!loading && splatCount > 0 && (
        <div
          style={{
            position: 'absolute',
            bottom: '10px',
            left: '50%',
            transform: 'translateX(-50%)',
            color: 'rgba(255,255,255,0.5)',
            fontSize: '11px',
            background: 'rgba(0, 0, 0, 0.4)',
            padding: '6px 14px',
            borderRadius: '12px',
            zIndex: 5,
            pointerEvents: 'none',
            backdropFilter: 'blur(4px)',
            whiteSpace: 'nowrap',
          }}
        >
          Left drag: Orbit &nbsp;|&nbsp; Right drag: Pan &nbsp;|&nbsp; Scroll: Zoom &nbsp;|&nbsp; Click: Focus
        </div>
      )}

      {/* Spinner animation */}
      <style>{`
        @keyframes spin {
          from { transform: rotate(0deg); }
          to { transform: rotate(360deg); }
        }
      `}</style>
    </div>
  );
};

export default Gaussian3DViewer;
