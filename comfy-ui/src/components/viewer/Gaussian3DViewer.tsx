import React, { useRef, useEffect, useState, useCallback } from 'react';
import * as THREE from 'three';
import { OrbitControls } from 'three/examples/jsm/controls/OrbitControls.js';

interface Gaussian3DViewerProps {
  filePath: string;
  format?: 'ply' | 'splat';
  width?: number;
  height?: number;
}

/**
 * 3D Gaussian Viewer component
 * Renders 3D Gaussian splats (PLY or SPLAT format) using Three.js
 * Inspired by SuperSplat and SparkGLS viewers
 */
const Gaussian3DViewer: React.FC<Gaussian3DViewerProps> = ({
  filePath,
  format = 'ply',
  width = 800,
  height = 600,
}) => {
  const containerRef = useRef<HTMLDivElement>(null);
  const rendererRef = useRef<THREE.WebGLRenderer | null>(null);
  const sceneRef = useRef<THREE.Scene | null>(null);
  const cameraRef = useRef<THREE.PerspectiveCamera | null>(null);
  const controlsRef = useRef<OrbitControls | null>(null);
  const pointsRef = useRef<THREE.Points | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [gaussianCount, setGaussianCount] = useState(0);
  const [isDragging, setIsDragging] = useState(false);

  // Initialize Three.js scene
  useEffect(() => {
    if (!containerRef.current) return;

    // Scene setup
    const scene = new THREE.Scene();
    scene.background = new THREE.Color(0x1a1a2e);
    sceneRef.current = scene;

    // Camera setup
    const camera = new THREE.PerspectiveCamera(
      75,
      width / height,
      0.1,
      1000
    );
    camera.position.set(0, 0, 2);
    cameraRef.current = camera;

    // Renderer setup
    const renderer = new THREE.WebGLRenderer({
      antialias: true,
      alpha: true,
    });
    renderer.setSize(width, height);
    renderer.setPixelRatio(window.devicePixelRatio);
    containerRef.current.appendChild(renderer.domElement);
    rendererRef.current = renderer;

    // Orbit controls
    const controls = new OrbitControls(camera, renderer.domElement);
    controls.enableDamping = true;
    controls.dampingFactor = 0.05;
    controlsRef.current = controls;

    // Add some lighting
    const ambientLight = new THREE.AmbientLight(0xffffff, 0.5);
    scene.add(ambientLight);

    const directionalLight = new THREE.DirectionalLight(0xffffff, 1);
    directionalLight.position.set(5, 5, 5);
    scene.add(directionalLight);

    // Animation loop
    let animationId: number;
    const animate = () => {
      animationId = requestAnimationFrame(animate);
      controls.update();
      renderer.render(scene, camera);
    };
    animate();

    // Handle resize
    const handleResize = () => {
      if (!containerRef.current || !cameraRef.current || !rendererRef.current) return;
      const newWidth = containerRef.current.clientWidth;
      const newHeight = containerRef.current.clientHeight;
      cameraRef.current.aspect = newWidth / newHeight;
      cameraRef.current.updateProjectionMatrix();
      rendererRef.current.setSize(newWidth, newHeight);
    };

    window.addEventListener('resize', handleResize);

    return () => {
      window.removeEventListener('resize', handleResize);
      cancelAnimationFrame(animationId);
      if (containerRef.current && renderer.domElement.parentNode === containerRef.current) {
        containerRef.current.removeChild(renderer.domElement);
      }
      renderer.dispose();
      controls.dispose();
    };
  }, [width, height]);

  // Load and parse 3D Gaussian data
  const loadGaussians = useCallback(async () => {
    if (!filePath || !sceneRef.current) return;

    setLoading(true);
    setError(null);

    try {
      let positions: Float32Array;
      let colors: Float32Array;
      let opacities: Float32Array;
      let counts: number;

      if (format === 'splat') {
        // Parse .splat format
        // Each record: xyz(12 bytes) + scale(12 bytes) + rgba(4 bytes) + rot(4 bytes) = 32 bytes
        const response = await fetch(filePath);
        if (!response.ok) throw new Error(`Failed to load: ${response.status}`);
        const buffer = await response.arrayBuffer();
        const data = new DataView(buffer);
        counts = data.byteLength / 32;

        positions = new Float32Array(counts * 3);
        colors = new Float32Array(counts * 3);
        opacities = new Float32Array(counts);

        for (let i = 0; i < counts; i++) {
          const offset = i * 32;
          
          // xyz (float32, 3 components)
          positions[i * 3] = data.getFloat32(offset, true);
          positions[i * 3 + 1] = data.getFloat32(offset + 4, true);
          positions[i * 3 + 2] = data.getFloat32(offset + 8, true);

          // rgba (uint8, 4 components) - convert to color
          const r = data.getUint8(offset + 24);
          const g = data.getUint8(offset + 25);
          const b = data.getUint8(offset + 26);
          const a = data.getUint8(offset + 27);
          
          colors[i * 3] = r / 255;
          colors[i * 3 + 1] = g / 255;
          colors[i * 3 + 2] = b / 255;
          opacities[i] = a / 255;
        }
      } else {
        // Parse .ply format (binary, little-endian)
        const response = await fetch(filePath);
        if (!response.ok) throw new Error(`Failed to load: ${response.status}`);
        const buffer = await response.arrayBuffer();
        const data = new DataView(buffer);
        const textDecoder = new TextDecoder();

        // Read header
        let headerEnd = 8; // "ply\n" + "1.0\n"
        while (true) {
          const line = textDecoder.decode(
            new Uint8Array(buffer, headerEnd, buffer.byteLength - headerEnd)
          ).split('\n')[0];
          headerEnd += line.length + 1;
          if (line === 'end_header') break;
        }

        // Parse header to get vertex count and property info
        const headerText = textDecoder.decode(
          new Uint8Array(buffer, 0, headerEnd)
        );
        const vertexMatch = headerText.match(/element vertex (\d+)/);
        if (!vertexMatch) throw new Error('Invalid PLY header');
        counts = parseInt(vertexMatch[1]);

        // Find property offsets
        const props: Record<string, number> = {};
        headerText.split('\n').forEach((line) => {
          const match = line.match(/property (\w+) (\w+)/);
          if (match) {
            const type = match[1];
            const name = match[2];
            // Simple size calculation
            let size = 0;
            if (type === 'float') size = 4;
            else if (type === 'int') size = 4;
            else if (type === 'uchar') size = 1;
            props[name] = size;
          }
        });

        // Calculate offsets
        const offsets: Record<string, number> = {};
        let currentOffset = 0;
        Object.keys(props).forEach((key) => {
          offsets[key] = currentOffset;
          currentOffset += props[key];
        });

        positions = new Float32Array(counts * 3);
        colors = new Float32Array(counts * 3);
        opacities = new Float32Array(counts);

        const vertexDataStart = headerEnd;
        for (let i = 0; i < counts; i++) {
          const base = vertexDataStart + i * currentOffset;

          // xyz
          positions[i * 3] = data.getFloat32(base + offsets['x'], true);
          positions[i * 3 + 1] = data.getFloat32(base + offsets['y'], true);
          positions[i * 3 + 2] = data.getFloat32(base + offsets['z'], true);

          // f_dc_0, f_dc_1, f_dc_2 (SH coefficients for color)
          // Standard conversion from SH DC term to RGB
          const f_dc_0 = data.getFloat32(base + offsets['f_dc_0'], true);
          const f_dc_1 = data.getFloat32(base + offsets['f_dc_1'], true);
          const f_dc_2 = data.getFloat32(base + offsets['f_dc_2'], true);
          
          const C0 = 0.28209479177387814;
          colors[i * 3] = Math.max(0, Math.min(1, 0.5 + C0 * f_dc_0));
          colors[i * 3 + 1] = Math.max(0, Math.min(1, 0.5 + C0 * f_dc_1));
          colors[i * 3 + 2] = Math.max(0, Math.min(1, 0.5 + C0 * f_dc_2));

          // opacity
          const opacity_val = data.getFloat32(base + offsets['opacity'], true);
          opacities[i] = 1 / (1 + Math.exp(-opacity_val)); // sigmoid
        }
      }

      // Create Three.js Points
      const geometry = new THREE.BufferGeometry();
      geometry.setAttribute('position', new THREE.BufferAttribute(positions, 3));
      geometry.setAttribute('color', new THREE.BufferAttribute(colors, 3));

      // Create vertex colors material
      const material = new THREE.PointsMaterial({
        size: 0.02,
        vertexColors: true,
        transparent: true,
        opacity: 0.8,
        sizeAttenuation: true,
        depthWrite: false,
      });

      const points = new THREE.Points(geometry, material);
      
      // Remove old points if exists
      if (pointsRef.current) {
        sceneRef.current.remove(pointsRef.current);
        pointsRef.current.geometry.dispose();
        const mat = pointsRef.current.material;
        if (!Array.isArray(mat)) {
          mat.dispose();
        }
      }

      sceneRef.current.add(points);
      pointsRef.current = points;
      setGaussianCount(counts);
      setLoading(false);

      // Adjust camera to fit the scene
      const posAttr = geometry.attributes.position as THREE.BufferAttribute;
      const box = new THREE.Box3().setFromBufferAttribute(posAttr);
      const center = box.getCenter(new THREE.Vector3());
      const size = box.getSize(new THREE.Vector3());
      const maxDim = Math.max(size.x, size.y, size.z);
      
      if (controlsRef.current) {
        controlsRef.current.target.copy(center);
        controlsRef.current.update();
      }
      
      if (cameraRef.current) {
        cameraRef.current.position.set(
          center.x + maxDim * 2,
          center.y + maxDim,
          center.z + maxDim * 2
        );
        cameraRef.current.near = maxDim * 0.01;
        cameraRef.current.far = maxDim * 100;
        cameraRef.current.updateProjectionMatrix();
      }

    } catch (err) {
      console.error('Error loading 3D Gaussians:', err);
      setError(err instanceof Error ? err.message : 'Unknown error');
      setLoading(false);
    }
  }, [filePath, format]);

  // Load when file path changes
  useEffect(() => {
    loadGaussians();
  }, [loadGaussians]);

  // Drag and drop support
  const handleDragOver = useCallback((e: React.DragEvent) => {
    e.preventDefault();
    setIsDragging(true);
  }, []);

  const handleDragLeave = useCallback((e: React.DragEvent) => {
    e.preventDefault();
    setIsDragging(false);
  }, []);

  const handleDrop = useCallback((e: React.DragEvent) => {
    e.preventDefault();
    setIsDragging(false);
    
    const file = e.dataTransfer.files[0];
    if (file && (file.name.endsWith('.ply') || file.name.endsWith('.splat'))) {
      const url = URL.createObjectURL(file);
      // We can't update the prop, but we can trigger a reload
      // For now, just log it
      console.log('Dropped file:', file.name);
      URL.revokeObjectURL(url);
    }
  }, []);

  return (
    <div
      ref={containerRef}
      style={{
        width: '100%',
        height: '100%',
        position: 'relative',
        cursor: isDragging ? 'pointer' : 'grab',
      }}
      onDragOver={handleDragOver}
      onDragLeave={handleDragLeave}
      onDrop={handleDrop}
    >
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
            background: 'rgba(0, 0, 0, 0.5)',
            zIndex: 10,
          }}
        >
          <div style={{ color: 'white', fontSize: '16px' }}>
            Loading 3D Gaussians...
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

      {/* Info overlay */}
      {!loading && gaussianCount > 0 && (
        <div
          style={{
            position: 'absolute',
            top: '10px',
            left: '10px',
            color: 'white',
            fontSize: '12px',
            background: 'rgba(0, 0, 0, 0.6)',
            padding: '8px 12px',
            borderRadius: '4px',
            zIndex: 5,
            pointerEvents: 'none',
          }}
        >
          <div>{gaussianCount.toLocaleString()} Gaussians</div>
          <div style={{ opacity: 0.7, marginTop: '4px' }}>
            Drag to rotate • Scroll to zoom
          </div>
        </div>
      )}
    </div>
  );
};

export default Gaussian3DViewer;
