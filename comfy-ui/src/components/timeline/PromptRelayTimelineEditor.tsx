import { useState, useCallback, useRef, useEffect, memo, type FC } from 'react';
import { useWorkflowStore } from '@/store/workflow';

// 10-color palette with golden angle rotation for additional colors
const PALETTE = [
  '#4a9eff', '#ff6b6b', '#51cf66', '#ffd43b', '#cc5de8',
  '#20c997', '#ff922b', '#748ffc', '#f06595', '#66d9e8',
];

function getSegmentColor(index: number): string {
  return PALETTE[index % PALETTE.length];
}

interface TimelineSegment {
  id: string;
  text: string;
  length: number;
  color: string;
}

interface PromptRelayTimelineEditorProps {
  nodeId: string;
  maxFrames: number;
  fps: number;
  timeUnits: string;
  localPrompts: string;
  segmentLengths: string;
  timelineData: string;
  height?: number;
}

// Zoom limits
const MIN_ZOOM = 0.1;   // 10% (fit all)
const MAX_ZOOM = 20.0;  // 2000% (very detailed)
const ZOOM_STEP = 1.2;  // per wheel notch / button click

export const PromptRelayTimelineEditor: FC<PromptRelayTimelineEditorProps> = memo(({
  nodeId,
  maxFrames,
  fps,
  timeUnits,
  localPrompts,
  segmentLengths,
  timelineData: _timelineData,
  height = 160,
}) => {
  const updateNodeInput = useWorkflowStore((s) => s.updateNodeInput);
  const canvasRef = useRef<HTMLCanvasElement>(null);
  const containerRef = useRef<HTMLDivElement>(null);

  // Zoom & pan state
  const [zoom, setZoom] = useState<number>(1.0);
  const [scrollX, setScrollX] = useState<number>(0); // in frame units

  const [selectedIdx, setSelectedIdx] = useState<number>(-1);
  const [dragIdx, setDragIdx] = useState<number>(-1);
  const [dragType, setDragType] = useState<'move' | 'resize-left' | 'resize-right' | 'pan' | null>(null);
  const [dragStartX, setDragStartX] = useState(0);
  const [dragStartScrollX, setDragStartScrollX] = useState(0);
  const textareaRef = useRef<HTMLTextAreaElement>(null);
  const debounceRef = useRef<ReturnType<typeof setTimeout> | null>(null);

  // Parse segments from local_prompts and segment_lengths
  const parseSegments = useCallback((): TimelineSegment[] => {
    const texts = localPrompts
      ? localPrompts.split('|').map((s: string) => s.trim()).filter((s: string) => s.length > 0)
      : [];
    const lengths = segmentLengths
      ? segmentLengths.split(',').map((s: string) => parseInt(s.trim(), 10)).filter((n: number) => !isNaN(n))
      : [];

    if (texts.length === 0) {
      return [{ id: 'seg-0', text: 'Segment 1', length: maxFrames, color: PALETTE[0] }];
    }

    return texts.map((text, i) => ({
      id: `seg-${i}`,
      text,
      length: i < lengths.length ? lengths[i] : Math.floor(maxFrames / texts.length),
      color: getSegmentColor(i),
    }));
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [localPrompts, segmentLengths, maxFrames]);

  const [segments, setSegments] = useState<TimelineSegment[]>(parseSegments);

  useEffect(() => {
    setSegments(parseSegments());
  }, [parseSegments]);

  // Auto-fit zoom when segments or maxFrames change significantly
  useEffect(() => {
    if (!containerRef.current) return;
    const containerW = containerRef.current.clientWidth;
    const totalLen = segments.reduce((sum, s) => sum + s.length, 0) || maxFrames;
    const fitZoom = containerW / totalLen;
    // Only auto-fit on initial load or large changes
    if (zoom === 1.0 || fitZoom < zoom * 0.3 || fitZoom > zoom * 4) {
      setZoom(Math.max(MIN_ZOOM, Math.min(MAX_ZOOM, fitZoom)));
      setScrollX(0);
    }
    // Only run when segments/maxFrames change, not zoom/scrollX
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [segments, maxFrames]);

  // Sync segments back to node inputs
  const syncToInputs = useCallback((segs: TimelineSegment[]) => {
    const lp = segs.map((s) => s.text).join(' | ');
    const sl = segs.map((s) => s.length).join(',');
    const td = JSON.stringify(segs.map((s) => ({ text: s.text, length: s.length })));

    updateNodeInput(nodeId, 'local_prompts', lp);
    updateNodeInput(nodeId, 'segment_lengths', sl);
    updateNodeInput(nodeId, 'timeline_data', td);
  }, [nodeId, updateNodeInput]);

  // Debounced text update
  const debouncedTextUpdate = useCallback((idx: number, text: string) => {
    if (debounceRef.current) clearTimeout(debounceRef.current);
    debounceRef.current = setTimeout(() => {
      const newSegs = [...segments];
      newSegs[idx] = { ...newSegs[idx], text };
      setSegments(newSegs);
      syncToInputs(newSegs);
    }, 120);
  }, [segments, syncToInputs]);

  // Add segment
  const handleAdd = useCallback(() => {
    const newSegs = [...segments, {
      id: `seg-${segments.length}`,
      text: 'New segment',
      length: Math.floor(maxFrames / (segments.length + 1)),
      color: getSegmentColor(segments.length),
    }];
    setSegments(newSegs);
    syncToInputs(newSegs);
  }, [segments, maxFrames, syncToInputs]);

  // Delete segment
  const handleDelete = useCallback(() => {
    if (selectedIdx < 0 || selectedIdx >= segments.length || segments.length <= 1) return;
    const newSegs = segments.filter((_, i) => i !== selectedIdx);
    setSegments(newSegs);
    setSelectedIdx(-1);
    syncToInputs(newSegs);
  }, [selectedIdx, segments, syncToInputs]);

  // Equalize segments
  const handleEqualize = useCallback(() => {
    const equalLen = Math.floor(maxFrames / segments.length);
    const newSegs = segments.map((s) => ({ ...s, length: equalLen }));
    setSegments(newSegs);
    syncToInputs(newSegs);
  }, [segments, maxFrames, syncToInputs]);

  // Fit to view
  const handleFitView = useCallback(() => {
    if (!containerRef.current) return;
    const containerW = containerRef.current.clientWidth;
    const totalLen = segments.reduce((sum, s) => sum + s.length, 0) || maxFrames;
    const fitZoom = containerW / totalLen;
    setZoom(Math.max(MIN_ZOOM, Math.min(MAX_ZOOM, fitZoom)));
    setScrollX(0);
  }, [segments, maxFrames]);

  // Zoom controls
  const handleZoomIn = useCallback(() => {
    setZoom((z) => Math.min(MAX_ZOOM, z * ZOOM_STEP));
  }, []);

  const handleZoomOut = useCallback(() => {
    setZoom((z) => Math.max(MIN_ZOOM, z / ZOOM_STEP));
  }, []);

  // Wheel zoom (centered on mouse position)
  const handleWheel = useCallback((e: React.WheelEvent<HTMLCanvasElement>) => {
    e.preventDefault();
    if (e.ctrlKey || e.metaKey) {
      // Pinch-zoom or Ctrl+scroll: zoom centered on cursor
      const canvas = canvasRef.current;
      if (!canvas) return;
      const rect = canvas.getBoundingClientRect();
      const mx = e.clientX - rect.left; // px within canvas
      // Convert mx to frame coordinate under cursor before zoom
      const frameUnderCursor = scrollX + mx / zoom;

      const factor = e.deltaY > 0 ? 1 / ZOOM_STEP : ZOOM_STEP;
      setZoom((z) => {
        const newZ = Math.max(MIN_ZOOM, Math.min(MAX_ZOOM, z * factor));
        // Adjust scroll so the frame under cursor stays under the mouse
        setScrollX(() => {
          const newSx = frameUnderCursor - mx / newZ;
          return Math.max(0, newSx);
        });
        return newZ;
      });
    } else {
      // Horizontal scroll: pan
      const canvas = canvasRef.current;
      if (!canvas) return;
      const w = canvas.clientWidth;
      const totalLen = segments.reduce((sum, s) => sum + s.length, 0) || maxFrames;
      const viewableFrames = w / zoom;
      const panDelta = e.deltaX !== 0 ? e.deltaX : e.deltaY;
      const frameDelta = panDelta / zoom;
      setScrollX((sx) => {
        const newSx = sx - frameDelta;
        return Math.max(0, Math.min(totalLen - viewableFrames, newSx));
      });
    }
  }, [zoom, scrollX, segments, maxFrames]);

  // Canvas drawing
  useEffect(() => {
    const canvas = canvasRef.current;
    if (!canvas) return;
    const ctx = canvas.getContext('2d');
    if (!ctx) return;

    const dpr = window.devicePixelRatio || 1;
    const w = canvas.clientWidth;
    const h = canvas.clientHeight;
    canvas.width = w * dpr;
    canvas.height = h * dpr;
    ctx.scale(dpr, dpr);

    // Clear
    ctx.fillStyle = '#111';
    ctx.fillRect(0, 0, w, h);

    const totalLen = segments.reduce((sum, s) => sum + s.length, 0) || maxFrames;
    const rulerH = 22;
    const segY = rulerH + 4;
    const segH = h - segY - 4;
    const viewableFrames = w / zoom;
    const pxPerFrame = zoom; // pixels per frame at current zoom level

    // Draw ruler background
    ctx.fillStyle = '#1a1a2e';
    ctx.fillRect(0, 0, w, rulerH);
    ctx.strokeStyle = '#333';
    ctx.lineWidth = 1;
    ctx.beginPath();
    ctx.moveTo(0, rulerH);
    ctx.lineTo(w, rulerH);
    ctx.stroke();

    // Ruler ticks - adaptive based on zoom level
    let tickStep = 10;
    const minTickPx = 30; // minimum pixels between ticks
    while (tickStep * pxPerFrame < minTickPx && tickStep < totalLen) {
      tickStep *= 2;
    }
    if (pxPerFrame > 40 && tickStep > 1) tickStep = 1;
    if (pxPerFrame > 80) tickStep = 1;

    ctx.fillStyle = '#888';
    ctx.font = '9px monospace';
    ctx.textAlign = 'center';

    // Calculate visible range in frames
    const startFrame = Math.floor(scrollX);
    const endFrame = Math.ceil(scrollX + viewableFrames);

    for (let f = Math.floor(startFrame / tickStep) * tickStep; f <= endFrame + tickStep; f += tickStep) {
      if (f < 0 || f > totalLen) continue;
      const x = (f - scrollX) * pxPerFrame;
      if (x < -50 || x > w + 50) continue;
      ctx.beginPath();
      ctx.moveTo(x, rulerH - 6);
      ctx.lineTo(x, rulerH);
      ctx.stroke();

      const label = timeUnits === 'seconds' && fps > 0
        ? (f / fps).toFixed(1) + 's'
        : String(f);
      ctx.fillText(label, x, rulerH - 8);
    }

    // Draw segments
    let xFrame = 0; // position of current segment start in frame space
    for (let i = 0; i < segments.length; i++) {
      const seg = segments[i];
      const segW_px = seg.length * pxPerFrame;
      const segX_px = (xFrame - scrollX) * pxPerFrame; // pixel position on screen

      // Skip if completely off-screen to the left
      if (segX_px + segW_px < 0) {
        xFrame += seg.length;
        continue;
      }
      // Stop if completely off-screen to the right
      if (segX_px > w) break;

      const isSelected = i === selectedIdx;

      // Segment block
      ctx.fillStyle = isSelected ? seg.color : seg.color + '88';
      ctx.fillRect(segX_px + 1, segY, segW_px - 2, segH);

      // Border for selected
      if (isSelected) {
        ctx.strokeStyle = '#fff';
        ctx.lineWidth = 2;
        ctx.strokeRect(segX_px + 1, segY, segW_px - 2, segH);
      }

      // Segment text (wrap to 2 lines)
      const maxTextW = segW_px - 8;
      if (maxTextW > 15) {
        ctx.fillStyle = '#fff';
        ctx.font = `${Math.min(11, Math.max(8, Math.round(pxPerFrame * 0.8)))}px sans-serif`;
        ctx.textAlign = 'left';

        const words = seg.text.split(' ');
        let line1 = '';
        let line2 = '';
        for (const word of words) {
          const test = line1 ? line1 + ' ' + word : word;
          if (ctx.measureText(test).width > maxTextW) {
            if (!line1) { line1 = word.slice(0, 12) + '\u2026'; break; }
            if (line2) { line2 = line2.slice(0, -1) + '\u2026'; break; }
            line2 = word;
          } else {
            if (line2 || line1) {
              line2 = line2 ? line2 + ' ' + word : word;
            } else {
              line1 = test;
            }
          }
        }
        ctx.fillText(line1, segX_px + 4, segY + segH / 2 - 4);
        if (line2) ctx.fillText(line2, segX_px + 4, segY + segH / 2 + 8);
      }

      // Frame range label
      const startF = Math.round(xFrame);
      const endF = Math.round(xFrame + seg.length);
      if (segW_px > 35) {
        ctx.fillStyle = '#ffffff88';
        ctx.font = '8px monospace';
        ctx.textAlign = 'right';
        ctx.fillText(`${startF}-${endF}`, segX_px + segW_px - 4, segY + 10);
      }

      // Resize handles
      if (isSelected && segW_px > 10) {
        ctx.fillStyle = '#fff';
        ctx.fillRect(segX_px, segY, 3, segH);
        ctx.fillRect(segX_px + segW_px - 3, segY, 3, segH);
      }

      xFrame += seg.length;
    }

    // Draw viewport indicator bar (shows where we are in the full timeline)
    if (viewableFrames < totalLen * 0.95) {
      const indicatorH = 3;
      const indicatorY = h - indicatorH;
      const indicatorW = (viewableFrames / totalLen) * w;
      const indicatorX = (scrollX / totalLen) * w;
      ctx.fillStyle = '#555';
      ctx.fillRect(0, indicatorY, w, indicatorH);
      ctx.fillStyle = '#aaa';
      ctx.fillRect(indicatorX, indicatorY, Math.max(indicatorW, 4), indicatorH);
    }
  }, [segments, selectedIdx, maxFrames, timeUnits, fps, zoom, scrollX]);

  // Canvas mouse handling
  const handleCanvasMouseDown = useCallback((e: React.MouseEvent<HTMLCanvasElement>) => {
    const canvas = canvasRef.current;
    if (!canvas) return;
    const rect = canvas.getBoundingClientRect();
    const mx = e.clientX - rect.left;
    const my = e.clientY - rect.top;
    const pxPerFrame = zoom;
    const rulerH = 26;
    const segY = rulerH;

    if (mx < 0 || my < segY) {
      // Click on ruler area -> start panning
      setDragType('pan');
      setDragIdx(-1);
      setDragStartX(e.clientX);
      setDragStartScrollX(scrollX);
      return;
    }

    // Find which segment was clicked (in frame space)
    let xFrame = 0;
    for (let i = 0; i < segments.length; i++) {
      const segW_px = segments[i].length * pxPerFrame;
      const segX_px = (xFrame - scrollX) * pxPerFrame;
      if (mx >= segX_px && mx < segX_px + segW_px) {
        // Check if near edges for resize
        if (i === selectedIdx) {
          if (mx - segX_px < 6) {
            setDragType('resize-left');
            setDragIdx(i);
            setDragStartX(e.clientX);
            return;
          }
          if (segX_px + segW_px - mx < 6) {
            setDragType('resize-right');
            setDragIdx(i);
            setDragStartX(e.clientX);
            return;
          }
        }
        setSelectedIdx(i);
        setDragType('move');
        setDragIdx(i);
        setDragStartX(e.clientX);
        return;
      }
      xFrame += segments[i].length;
    }
    setSelectedIdx(-1);
  }, [segments, selectedIdx, maxFrames, zoom, scrollX]);

  const handleCanvasMouseMove = useCallback((e: React.MouseEvent<HTMLCanvasElement>) => {
    if (dragType === null) return;

    if (dragType === 'pan') {
      const dx = e.clientX - dragStartX;
      const frameDx = dx / zoom;
      const totalLen = segments.reduce((sum, s) => sum + s.length, 0) || maxFrames;
      const canvas = canvasRef.current;
      const w = canvas ? canvas.clientWidth : 300;
      const viewableFrames = w / zoom;
      setScrollX(Math.max(0, Math.min(totalLen - viewableFrames, dragStartScrollX - frameDx)));
      return;
    }

    if (dragIdx < 0) return;
    const dx = e.clientX - dragStartX;
    const frameDelta = Math.round(dx / zoom);

    if (frameDelta === 0) return;

    const newSegs = [...segments];

    if (dragType === 'resize-right' && dragIdx < newSegs.length - 1) {
      const delta = Math.min(frameDelta, newSegs[dragIdx + 1].length - 1);
      if (delta !== 0) {
        newSegs[dragIdx] = { ...newSegs[dragIdx], length: newSegs[dragIdx].length + delta };
        newSegs[dragIdx + 1] = { ...newSegs[dragIdx + 1], length: newSegs[dragIdx + 1].length - delta };
        setSegments(newSegs);
        setDragStartX(e.clientX);
      }
    } else if (dragType === 'resize-left' && dragIdx > 0) {
      const delta = Math.min(-frameDelta, newSegs[dragIdx - 1].length - 1);
      if (delta !== 0) {
        newSegs[dragIdx - 1] = { ...newSegs[dragIdx - 1], length: newSegs[dragIdx - 1].length - delta };
        newSegs[dragIdx] = { ...newSegs[dragIdx], length: newSegs[dragIdx].length + delta };
        setSegments(newSegs);
        setDragStartX(e.clientX);
      }
    }
  }, [dragType, dragIdx, dragStartX, dragStartScrollX, segments, maxFrames, zoom]);

  const handleCanvasMouseUp = useCallback(() => {
    if (dragType !== null && dragType !== 'pan') {
      syncToInputs(segments);
    }
    setDragType(null);
    setDragIdx(-1);
  }, [dragType, segments, syncToInputs]);

  const selectedSegment = selectedIdx >= 0 && selectedIdx < segments.length
    ? segments[selectedIdx]
    : null;

  const zoomPercent = Math.round(zoom * 100);

  // Calculate canvas height from the total height prop
  const toolbarH = 24;
  const editorH = selectedSegment ? 64 : 0;
  const canvasH = Math.max(50, height - toolbarH - editorH - 16);

  return (
    <div
      ref={containerRef}
      style={{
        borderBottom: '1px solid #333',
        padding: '4px 6px',
      }}
    >
      {/* Toolbar row */}
      <div style={{
        display: 'flex',
        alignItems: 'center',
        gap: 4,
        marginBottom: 2,
        fontSize: 10,
      }}>
        {/* Zoom controls */}
        <button onClick={handleZoomOut} title="Zoom out" style={btnStyle}>−</button>
        <span
          title={`${zoomPercent}%`}
          style={{ color: '#aaa', minWidth: 38, textAlign: 'center', fontSize: 9 }}
        >
          {zoomPercent}%
        </span>
        <button onClick={handleZoomIn} title="Zoom in" style={btnStyle}>+</button>

        <span style={{ width: 1, height: 14, background: '#333', display: 'inline-block' }} />

        <button onClick={handleFitView} title="Fit all" style={btnStyle}>&#8634;</button>

        <span style={{ width: 1, height: 14, background: '#333', display: 'inline-block' }} />

        <button onClick={handleAdd} title="Add segment" style={btnStyle}>+</button>
        <button onClick={handleEqualize} title="Equalize lengths" style={btnStyle}>≡</button>
        {selectedIdx >= 0 && segments.length > 1 && (
          <button onClick={handleDelete} title="Delete segment" style={{ ...btnStyle, color: '#ff6b6b' }}>×</button>
        )}

        <span style={{ flex: 1 }} />

        <span style={{ color: '#888' }}>
          {maxFrames} frames
          {timeUnits === 'seconds' && fps > 0 ? ` (${(maxFrames / fps).toFixed(1)}s)` : ''}
        </span>
      </div>

      {/* Timeline canvas */}
      <canvas
        ref={canvasRef}
        style={{
          width: '100%',
          height: canvasH,
          borderRadius: 3,
          cursor: dragType === 'pan'
            ? 'grabbing'
            : dragType
              ? 'grabbing'
              : 'pointer',
        }}
        onMouseDown={handleCanvasMouseDown}
        onMouseMove={handleCanvasMouseMove}
        onMouseUp={handleCanvasMouseUp}
        onMouseLeave={handleCanvasMouseUp}
        onWheel={handleWheel}
      />

      {/* Selected segment editor */}
      {selectedSegment && (
        <div style={{
          marginTop: 4,
          padding: '4px 6px',
          background: '#1a1a2e',
          borderRadius: 3,
          borderLeft: `3px solid ${selectedSegment.color}`,
        }}>
          <textarea
            ref={textareaRef}
            defaultValue={selectedSegment.text}
            onChange={(e) => debouncedTextUpdate(selectedIdx, e.target.value)}
            rows={2}
            style={{
              width: '100%',
              background: '#2a2a3e',
              border: '1px solid #444',
              borderRadius: 3,
              color: '#e2e8f0',
              padding: '2px 4px',
              fontSize: 10,
              resize: 'vertical',
              fontFamily: 'inherit',
            }}
          />
          <div style={{ display: 'flex', alignItems: 'center', gap: 4, marginTop: 2 }}>
            <span style={{ fontSize: 9, color: '#888' }}>Length:</span>
            <input
              type="number"
              value={selectedSegment.length}
              min={1}
              onChange={(e) => {
                const newLen = Math.max(1, parseInt(e.target.value, 10) || 1);
                const newSegs = [...segments];
                newSegs[selectedIdx] = { ...newSegs[selectedIdx], length: newLen };
                setSegments(newSegs);
                syncToInputs(newSegs);
              }}
              style={{
                width: 50,
                background: '#2a2a3e',
                border: '1px solid #444',
                borderRadius: 3,
                color: '#e2e8f0',
                padding: '1px 4px',
                fontSize: 10,
              }}
            />
            <span style={{ fontSize: 9, color: '#888' }}>frames</span>
          </div>
        </div>
      )}
    </div>
  );
});

PromptRelayTimelineEditor.displayName = 'PromptRelayTimelineEditor';

const btnStyle: React.CSSProperties = {
  background: '#2a2a3e',
  border: '1px solid #444',
  borderRadius: 3,
  color: '#e2e8f0',
  padding: '0 5px',
  fontSize: 11,
  cursor: 'pointer',
  lineHeight: 1.6,
};
