import { useState, useEffect } from 'react';
import { api } from '@/api/client';
import type { ImageEntry } from '@/types/api';

export default function ImageGallery() {
  const [images, setImages] = useState<ImageEntry[]>([]);
  const [selected, setSelected] = useState<string | null>(null);
  const [loading, setLoading] = useState(false);

  useEffect(() => {
    loadImages();
  }, []);

  async function loadImages() {
    setLoading(true);
    try {
      const result = await api.listImages();
      setImages(result.images);
    } catch (err) {
      console.error('Failed to load images:', err);
    } finally {
      setLoading(false);
    }
  }

  return (
    <div style={{ padding: '8px' }}>
      <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', marginBottom: '8px' }}>
        <h3 style={{ margin: 0, fontSize: '14px' }}>Output Images</h3>
        <button onClick={loadImages} disabled={loading} style={{ fontSize: '12px', padding: '2px 8px' }}>
          Refresh
        </button>
      </div>

      {loading && <div style={{ fontSize: '12px', color: '#888' }}>Loading...</div>}

      <div style={{ display: 'grid', gridTemplateColumns: 'repeat(2, 1fr)', gap: '4px', maxHeight: '300px', overflowY: 'auto' }}>
        {images.map((img, idx) => {
          const url = api.getImageUrl(img.filename, img.subfolder, img.type);
          return (
            <div
              key={`${img.filename}-${idx}`}
              onClick={() => setSelected(selected === img.filename ? null : img.filename)}
              style={{
                cursor: 'pointer',
                border: selected === img.filename ? '2px solid #4a9eff' : '1px solid #333',
                borderRadius: '4px',
                overflow: 'hidden',
              }}
            >
              <img
                src={url}
                alt={img.filename}
                loading="lazy"
                style={{ width: '100%', height: '80px', objectFit: 'cover', display: 'block' }}
              />
              <div style={{ fontSize: '10px', padding: '2px', color: '#aaa', whiteSpace: 'nowrap', overflow: 'hidden', textOverflow: 'ellipsis' }}>
                {img.filename}
              </div>
            </div>
          );
        })}
      </div>

      {images.length === 0 && !loading && (
        <div style={{ fontSize: '12px', color: '#666', textAlign: 'center', padding: '16px' }}>
          No images yet
        </div>
      )}

      {selected && (
        <div style={{ marginTop: '8px' }}>
          <img
            src={api.getImageUrl(selected)}
            alt={selected}
            style={{ width: '100%', borderRadius: '4px' }}
          />
        </div>
      )}
    </div>
  );
}
