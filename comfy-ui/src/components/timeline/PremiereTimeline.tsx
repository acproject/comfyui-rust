import { useState, useCallback, useRef, useEffect } from 'react';
import TextBasedEditing from './TextBasedEditing';
import type { TranscriptData } from './TextBasedEditing';

export interface PremiereClip {
  id: string;
  trackId: string;
  startTime: number;
  inPoint: number;
  outPoint: number;
  duration: number;
  opacity?: number;
  volume?: number;
  name: string;
  hasVideo?: boolean;
  hasAudio?: boolean;
}

export interface PremiereTrack {
  id: string;
  type: 'video' | 'audio';
  name: string;
  clips: PremiereClip[];
  locked: boolean;
  muted: boolean;
  solo: boolean;
  visible: boolean;
  height: number;
}

export interface PremiereTimelineProps {
  tracks?: PremiereTrack[];
  currentTime?: number;
  totalDuration?: number;
  fps?: number;
  width?: number;
  height?: number;
  transcript?: TranscriptData | null;
  onClipDrag?: (clipId: string, newStartTime: number) => void;
  onTrackToggle?: (trackId: string, property: 'locked' | 'muted' | 'solo' | 'visible') => void;
  onTimeChange?: (time: number) => void;
  onApplyTextEdits?: (keepRanges: Array<{ start: number; end: number }>) => void;
}

const TRACK_CONTROL_WIDTH = 140;
const RULER_HEIGHT = 32;
const DEFAULT_VIDEO_TRACK_HEIGHT = 50;
const DEFAULT_AUDIO_TRACK_HEIGHT = 60;
const DEFAULT_FPS = 24;

export function PremiereTimeline({
  tracks: customTracks,
  currentTime = 0,
  totalDuration = 10,
  fps = DEFAULT_FPS,
  width = 800,
  height = 400,
  transcript = null,
  onClipDrag,
  onTrackToggle,
  onTimeChange,
  onApplyTextEdits,
}: PremiereTimelineProps) {
  const [zoom, setZoom] = useState(1);
  const [scrollX, setScrollX] = useState(0);
  const [playhead, setPlayhead] = useState(currentTime);
  const [isDraggingPlayhead, setIsDraggingPlayhead] = useState(false);
  const [draggingClip, setDraggingClip] = useState<{ id: string; startX: number; startTime: number } | null>(null);
  const [isFocused, setIsFocused] = useState(false);
  const [showTextPanel, setShowTextPanel] = useState(false);
  const [isPlaying, setIsPlaying] = useState(false);

  const timelineRef = useRef<HTMLDivElement>(null);
  const containerRef = useRef<HTMLDivElement>(null);

  // Default tracks matching VideoTimeline node (V1-V4, A1-A4)
  const defaultTracks: PremiereTrack[] = [
    { id: 'v4', type: 'video', name: 'V4', clips: [], locked: false, muted: false, solo: false, visible: true, height: DEFAULT_VIDEO_TRACK_HEIGHT },
    { id: 'v3', type: 'video', name: 'V3', clips: [], locked: false, muted: false, solo: false, visible: true, height: DEFAULT_VIDEO_TRACK_HEIGHT },
    { id: 'v2', type: 'video', name: 'V2', clips: [], locked: false, muted: false, solo: false, visible: true, height: DEFAULT_VIDEO_TRACK_HEIGHT },
    { id: 'v1', type: 'video', name: 'V1', clips: [], locked: false, muted: false, solo: false, visible: true, height: DEFAULT_VIDEO_TRACK_HEIGHT },
    { id: 'a1', type: 'audio', name: 'A1', clips: [], locked: false, muted: false, solo: false, visible: true, height: DEFAULT_AUDIO_TRACK_HEIGHT },
    { id: 'a2', type: 'audio', name: 'A2', clips: [], locked: false, muted: false, solo: false, visible: true, height: DEFAULT_AUDIO_TRACK_HEIGHT },
    { id: 'a3', type: 'audio', name: 'A3', clips: [], locked: false, muted: false, solo: false, visible: true, height: DEFAULT_AUDIO_TRACK_HEIGHT },
    { id: 'a4', type: 'audio', name: 'A4', clips: [], locked: false, muted: false, solo: false, visible: true, height: DEFAULT_AUDIO_TRACK_HEIGHT },
  ];

  const tracks = customTracks || defaultTracks;

  const pixelsPerSecond = 60 * zoom;
  const timelineContentWidth = Math.max(totalDuration * pixelsPerSecond, width - TRACK_CONTROL_WIDTH);
  const viewportWidth = width - TRACK_CONTROL_WIDTH;

  const totalTracksHeight = tracks.reduce((sum, t) => sum + t.height, 0);
  const timelineHeight = Math.max(totalTracksHeight + RULER_HEIGHT, height);

  // Scroll to make playhead visible in center (or at least within viewport)
  const scrollToPlayhead = useCallback((center: boolean = false) => {
    const playheadX = playhead * pixelsPerSecond;
    const desiredScrollX = center
      ? Math.max(0, playheadX - viewportWidth / 2)
      : Math.max(0, Math.min(playheadX - viewportWidth * 0.1, playheadX));
    const maxScrollX = Math.max(0, timelineContentWidth - viewportWidth);
    setScrollX(Math.min(desiredScrollX, maxScrollX));
  }, [playhead, pixelsPerSecond, viewportWidth, timelineContentWidth]);

  // Seek to a specific time
  const seekTo = useCallback((time: number) => {
    const clamped = Math.max(0, Math.min(totalDuration, time));
    setPlayhead(clamped);
    onTimeChange?.(clamped);
  }, [totalDuration, onTimeChange]);

  // Nudge playhead by N frames
  const nudgeFrames = useCallback((frames: number) => {
    const delta = frames / fps;
    seekTo(playhead + delta);
  }, [playhead, fps, seekTo]);

  // Nudge playhead by N seconds
  const nudgeSeconds = useCallback((seconds: number) => {
    seekTo(playhead + seconds);
  }, [playhead, seekTo]);

  // Zoom controls
  const zoomIn = useCallback(() => setZoom((z) => Math.min(10, z * 1.2)), []);
  const zoomOut = useCallback(() => setZoom((z) => Math.max(0.1, z / 1.2)), []);
  const zoomFit = useCallback(() => {
    const desiredZoom = (viewportWidth * 0.95) / totalDuration / 60;
    setZoom(Math.max(0.1, Math.min(10, desiredZoom)));
    setScrollX(0);
  }, [viewportWidth, totalDuration]);

  // Keyboard shortcuts
  useEffect(() => {
    if (!isFocused) return;

    const handleKeyDown = (e: KeyboardEvent) => {
      // Don't intercept if user is typing in an input
      const target = e.target as HTMLElement;
      if (target.tagName === 'INPUT' || target.tagName === 'TEXTAREA' || target.isContentEditable) {
        return;
      }

      switch (e.key) {
        case 'Home':
          e.preventDefault();
          seekTo(0);
          setScrollX(0);
          break;
        case 'End':
          e.preventDefault();
          seekTo(totalDuration);
          scrollToPlayhead(true);
          break;
        case 'ArrowLeft':
          e.preventDefault();
          if (e.shiftKey) {
            nudgeSeconds(-1);
          } else {
            nudgeFrames(-1);
          }
          scrollToPlayhead();
          break;
        case 'ArrowRight':
          e.preventDefault();
          if (e.shiftKey) {
            nudgeSeconds(1);
          } else {
            nudgeFrames(1);
          }
          scrollToPlayhead();
          break;
        case 'c':
        case 'C':
          e.preventDefault();
          scrollToPlayhead(true);
          break;
        case '+':
        case '=':
          e.preventDefault();
          zoomIn();
          break;
        case '-':
        case '_':
          e.preventDefault();
          zoomOut();
          break;
        case '\\':
          e.preventDefault();
          zoomFit();
          break;
        case '0':
          if (e.ctrlKey || e.metaKey) {
            e.preventDefault();
            zoomFit();
          }
          break;
      }
    };

    window.addEventListener('keydown', handleKeyDown);
    return () => window.removeEventListener('keydown', handleKeyDown);
  }, [isFocused, seekTo, totalDuration, nudgeFrames, nudgeSeconds, scrollToPlayhead, zoomIn, zoomOut, zoomFit]);

  // Sync external currentTime prop
  useEffect(() => {
    setPlayhead(currentTime);
  }, [currentTime]);

  // Generate time ruler markers
  const renderTimeRuler = useCallback(() => {
    const markers: React.ReactNode[] = [];
    let interval: number;
    let labelInterval: number;

    if (zoom < 0.3) {
      interval = 10;
      labelInterval = 10;
    } else if (zoom < 0.7) {
      interval = 5;
      labelInterval = 5;
    } else if (zoom < 1.5) {
      interval = 1;
      labelInterval = 1;
    } else if (zoom < 3) {
      interval = 0.5;
      labelInterval = 1;
    } else {
      interval = 0.1;
      labelInterval = 0.5;
    }

    for (let t = 0; t <= totalDuration + interval; t += interval) {
      const x = t * pixelsPerSecond - scrollX;
      if (x < -50 || x > width) continue;

      const isMajor = Math.abs(t % labelInterval) < 0.001;
      const markerHeight = isMajor ? RULER_HEIGHT : RULER_HEIGHT / 2;

      markers.push(
        <div
          key={`tick-${t}`}
          className="absolute"
          style={{
            left: x + TRACK_CONTROL_WIDTH,
            top: 0,
            width: 1,
            height: markerHeight,
            backgroundColor: isMajor ? '#4a4a5a' : '#3a3a4a',
          }}
        />
      );

      if (isMajor) {
        const minutes = Math.floor(t / 60);
        const seconds = Math.floor(t % 60);
        const frames = Math.floor((t % 1) * fps);
        let timeLabel: string;
        if (minutes > 0) {
          timeLabel = `${minutes}:${seconds.toString().padStart(2, '0')}`;
        } else if (zoom >= 3) {
          timeLabel = `${seconds.toString().padStart(2, '0')}:${frames.toString().padStart(2, '0')}`;
        } else {
          timeLabel = `${seconds}`;
        }
        markers.push(
          <div
            key={`label-${t}`}
            className="absolute text-xs text-gray-400 font-mono"
            style={{
              left: x + TRACK_CONTROL_WIDTH + 4,
              top: 2,
              fontSize: 10,
            }}
          >
            {timeLabel}
          </div>
        );
      }
    }
    return markers;
  }, [zoom, totalDuration, pixelsPerSecond, scrollX, width, fps]);

  // Handle playhead drag
  const handleTimelineClick = useCallback((e: React.MouseEvent) => {
    if (!timelineRef.current) return;
    const rect = timelineRef.current.getBoundingClientRect();
    const x = e.clientX - rect.left - TRACK_CONTROL_WIDTH + scrollX;
    const time = Math.max(0, Math.min(totalDuration, x / pixelsPerSecond));
    seekTo(time);
  }, [pixelsPerSecond, scrollX, totalDuration, seekTo]);

  const handlePlayheadMouseDown = useCallback((e: React.MouseEvent) => {
    e.stopPropagation();
    setIsDraggingPlayhead(true);
  }, []);

  // Handle clip drag
  const handleClipMouseDown = useCallback((e: React.MouseEvent, clip: PremiereClip) => {
    e.stopPropagation();
    setDraggingClip({
      id: clip.id,
      startX: e.clientX,
      startTime: clip.startTime,
    });
  }, []);

  useEffect(() => {
    const handleMouseMove = (e: MouseEvent) => {
      if (isDraggingPlayhead && timelineRef.current) {
        const rect = timelineRef.current.getBoundingClientRect();
        const x = e.clientX - rect.left - TRACK_CONTROL_WIDTH + scrollX;
        const time = Math.max(0, Math.min(totalDuration, x / pixelsPerSecond));
        setPlayhead(time);
        onTimeChange?.(time);
      }

      if (draggingClip) {
        const deltaX = e.clientX - draggingClip.startX;
        const deltaTime = deltaX / pixelsPerSecond;
        const newStartTime = Math.max(0, draggingClip.startTime + deltaTime);
        onClipDrag?.(draggingClip.id, newStartTime);
      }
    };

    const handleMouseUp = () => {
      setIsDraggingPlayhead(false);
      setDraggingClip(null);
    };

    window.addEventListener('mousemove', handleMouseMove);
    window.addEventListener('mouseup', handleMouseUp);
    return () => {
      window.removeEventListener('mousemove', handleMouseMove);
      window.removeEventListener('mouseup', handleMouseUp);
    };
  }, [isDraggingPlayhead, draggingClip, pixelsPerSecond, scrollX, totalDuration, onTimeChange, onClipDrag]);

  // Handle horizontal scroll
  const handleWheel = useCallback((e: React.WheelEvent) => {
    if (e.ctrlKey || e.metaKey) {
      // Zoom with Ctrl+Wheel
      e.preventDefault();
      const delta = e.deltaY > 0 ? 0.9 : 1.1;
      setZoom((z) => Math.max(0.1, Math.min(10, z * delta)));
    } else {
      // Scroll horizontally
      setScrollX((s) => {
        const maxScroll = Math.max(0, timelineContentWidth - viewportWidth);
        return Math.max(0, Math.min(maxScroll, s + e.deltaY));
      });
    }
  }, [timelineContentWidth, viewportWidth]);

  const handleTrackToggle = useCallback((trackId: string, property: 'locked' | 'muted' | 'solo' | 'visible') => {
    onTrackToggle?.(trackId, property);
  }, [onTrackToggle]);

  // Format SMPTE timecode
  const formatTimecode = (t: number): string => {
    const minutes = Math.floor(t / 60);
    const seconds = Math.floor(t % 60);
    const frames = Math.floor((t % 1) * fps);
    return `${minutes}:${seconds.toString().padStart(2, '0')}:${frames.toString().padStart(2, '0')}`;
  };

  // Render track controls (left panel)
  const renderTrackControls = () => {
    let currentY = RULER_HEIGHT;
    return tracks.map((track) => {
      const trackY = currentY;
      currentY += track.height;

      const isVideo = track.type === 'video';
      const trackColor = isVideo ? '#3b82f6' : '#06b6d4';
      const bgColor = isVideo ? '#1e3a5f' : '#164e63';
      const mutedBg = track.muted ? '#3f3f46' : bgColor;

      return (
        <div
          key={`control-${track.id}`}
          className="absolute left-0 flex items-center border-r border-b border-gray-700 px-2"
          style={{
            top: trackY,
            width: TRACK_CONTROL_WIDTH,
            height: track.height,
            backgroundColor: track.locked ? '#27272a' : mutedBg,
          }}
        >
          <div className="flex flex-col gap-1 w-full">
            <div className="flex items-center justify-between">
              <span
                className="text-xs font-bold px-1.5 py-0.5 rounded"
                style={{ backgroundColor: trackColor, color: 'white' }}
              >
                {track.name}
              </span>
              <div className="flex gap-0.5">
                {isVideo ? (
                  <>
                    <button
                      className={`w-5 h-5 flex items-center justify-center text-xs rounded ${
                        track.visible ? 'bg-gray-600 text-white' : 'bg-gray-800 text-gray-500'
                      }`}
                      onClick={() => handleTrackToggle(track.id, 'visible')}
                      title="Toggle Visibility (Eye icon)"
                    >
                      👁
                    </button>
                  </>
                ) : (
                  <>
                    <button
                      className={`w-5 h-5 flex items-center justify-center text-xs rounded font-bold ${
                        track.muted ? 'bg-red-600 text-white' : 'bg-gray-600 text-white'
                      }`}
                      onClick={() => handleTrackToggle(track.id, 'muted')}
                      title="Mute (M)"
                    >
                      M
                    </button>
                    <button
                      className={`w-5 h-5 flex items-center justify-center text-xs rounded font-bold ${
                        track.solo ? 'bg-yellow-600 text-white' : 'bg-gray-600 text-white'
                      }`}
                      onClick={() => handleTrackToggle(track.id, 'solo')}
                      title="Solo (S)"
                    >
                      S
                    </button>
                  </>
                )}
                <button
                  className={`w-5 h-5 flex items-center justify-center text-xs rounded ${
                    track.locked ? 'bg-gray-500 text-white' : 'bg-gray-700 text-gray-400'
                  }`}
                  onClick={() => handleTrackToggle(track.id, 'locked')}
                  title="Lock Track"
                >
                  🔒
                </button>
              </div>
            </div>
            <div className="flex gap-0.5">
              <button
                className="w-5 h-5 flex items-center justify-center text-xs bg-gray-700 text-gray-400 rounded hover:bg-gray-600"
                title="Collapse/Expand"
              >
                ▶
              </button>
              <button
                className="w-5 h-5 flex items-center justify-center text-xs bg-gray-700 text-gray-400 rounded hover:bg-gray-600"
                title="Keyframes"
              >
                ◆
              </button>
              <button
                className="w-5 h-5 flex items-center justify-center text-xs bg-gray-700 text-gray-400 rounded hover:bg-gray-600"
                title="Track Output"
              >
                ⊙
              </button>
              <div className="flex-1" />
              <span className="text-[10px] text-gray-500">
                {isVideo ? 'Video' : 'Audio'}
              </span>
            </div>
          </div>
        </div>
      );
    });
  };

  // Render tracks and clips
  const renderTracks = () => {
    let currentY = RULER_HEIGHT;
    return tracks.map((track) => {
      const trackY = currentY;
      currentY += track.height;

      const isVideo = track.type === 'video';
      const clipColor = isVideo ? '#2563eb' : '#0891b2';
      const clipBg = isVideo ? 'linear-gradient(180deg, #3b82f6 0%, #2563eb 100%)' : 'linear-gradient(180deg, #06b6d4 0%, #0891b2 100%)';
      const trackBg = isVideo ? '#0f172a' : '#0c1929';
      const mutedStyle = track.muted ? { opacity: 0.4, filter: 'grayscale(50%)' } : {};

      return (
        <div
          key={`track-${track.id}`}
          className="absolute"
          style={{
            top: trackY,
            left: 0,
            right: 0,
            height: track.height,
            backgroundColor: track.locked ? '#18181b' : trackBg,
            borderBottom: '1px solid #27272a',
            ...mutedStyle,
          }}
        >
          {/* Track background stripes */}
          <div
            className="absolute inset-0 opacity-10"
            style={{
              backgroundImage:
                'repeating-linear-gradient(90deg, transparent, transparent 99px, #333 99px, #333 100px)',
              backgroundPosition: `${TRACK_CONTROL_WIDTH - scrollX}px 0`,
            }}
          />

          {/* Clips */}
          {track.clips.map((clip) => {
            const clipLeft = TRACK_CONTROL_WIDTH + clip.startTime * pixelsPerSecond - scrollX;
            const clipWidth = (clip.outPoint - clip.inPoint) * pixelsPerSecond;

            if (clipLeft + clipWidth < TRACK_CONTROL_WIDTH || clipLeft > width) return null;

            return (
              <div
                key={clip.id}
                className={`absolute rounded cursor-move overflow-hidden select-none ${
                  track.locked ? 'cursor-not-allowed opacity-60' : ''
                }`}
                style={{
                  left: clipLeft,
                  top: 4,
                  width: Math.max(clipWidth, 20),
                  height: track.height - 8,
                  background: clipBg,
                  border: '1px solid rgba(255,255,255,0.2)',
                  boxShadow: '0 1px 3px rgba(0,0,0,0.3)',
                }}
                onMouseDown={(e) => !track.locked && handleClipMouseDown(e, clip)}
              >
                {/* Clip header */}
                <div
                  className="px-2 py-0.5 text-[10px] text-white font-medium truncate"
                  style={{ backgroundColor: clipColor, borderBottom: '1px solid rgba(255,255,255,0.1)' }}
                >
                  {clip.name}
                </div>

                {/* Clip content area */}
                <div className="flex-1 px-1" style={{ height: track.height - 8 - 18 }}>
                  {isVideo ? (
                    <div className="w-full h-full flex items-center justify-center">
                      <div
                        className="w-full h-full opacity-30"
                        style={{
                          background:
                            'repeating-linear-gradient(45deg, rgba(255,255,255,0.1) 0px, rgba(255,255,255,0.1) 4px, transparent 4px, transparent 8px)',
                        }}
                      />
                    </div>
                  ) : (
                    <div className="w-full h-full flex items-center justify-center overflow-hidden">
                      <div className="text-white/50 text-[9px]">
                        ♪ Audio Waveform ♪
                      </div>
                    </div>
                  )}
                </div>

                {/* Clip handles (in/out points) */}
                <div
                  className="absolute left-0 top-0 bottom-0 w-2 cursor-ew-resize"
                  style={{ backgroundColor: 'rgba(255,255,255,0.2)' }}
                />
                <div
                  className="absolute right-0 top-0 bottom-0 w-2 cursor-ew-resize"
                  style={{ backgroundColor: 'rgba(255,255,255,0.2)' }}
                />
              </div>
            );
          })}
        </div>
      );
    });
  };

  return (
    <div
      ref={containerRef}
      className={`w-full bg-[#1a1a1f] rounded-lg border overflow-hidden select-none flex flex-col ${
        isFocused ? 'border-blue-500 ring-1 ring-blue-500/30' : 'border-gray-700'
      }`}
      style={{ fontSize: 11 }}
      tabIndex={0}
      onFocus={() => setIsFocused(true)}
      onBlur={() => setIsFocused(false)}
      onClick={() => containerRef.current?.focus()}
    >
      {/* Toolbar */}
      <div className="flex items-center justify-between px-3 py-2 bg-[#252528] border-b border-gray-700">
        <div className="flex items-center gap-2">
          <span className="text-white font-medium text-xs">🎬 Multi-Track Timeline</span>
          <span className="text-gray-500 text-[10px]">
            {tracks.filter((t) => t.type === 'video').length}V | {tracks.filter((t) => t.type === 'audio').length}A
          </span>
          {isFocused && (
            <span className="text-[9px] text-blue-400 bg-blue-900/30 px-1.5 py-0.5 rounded">
              Shortcuts Active
            </span>
          )}
        </div>
        <div className="flex items-center gap-2">
          <span className="text-gray-400 text-[10px] font-mono" title="Current Time (SMPTE)">
            {formatTimecode(playhead)}
          </span>
          <div className="w-px h-4 bg-gray-600" />
          {/* Transport controls */}
          <button
            onClick={() => { seekTo(0); setScrollX(0); }}
            className="px-1.5 py-0.5 bg-gray-700 hover:bg-gray-600 text-white text-xs rounded"
            title="Go to Start (Home)"
          >
            ⏮
          </button>
          <button
            onClick={() => scrollToPlayhead(true)}
            className="px-1.5 py-0.5 bg-gray-700 hover:bg-gray-600 text-white text-xs rounded"
            title="Center Playhead (C)"
          >
            ⊙
          </button>
          <button
            onClick={() => { seekTo(totalDuration); scrollToPlayhead(true); }}
            className="px-1.5 py-0.5 bg-gray-700 hover:bg-gray-600 text-white text-xs rounded"
            title="Go to End (End)"
          >
            ⏭
          </button>
          <div className="w-px h-4 bg-gray-600" />
          {/* Nudge controls */}
          <button
            onClick={() => { nudgeFrames(-1); scrollToPlayhead(); }}
            className="px-1.5 py-0.5 bg-gray-700 hover:bg-gray-600 text-white text-xs rounded"
            title="Previous Frame (←)"
          >
            ◀
          </button>
          <button
            onClick={() => { nudgeFrames(1); scrollToPlayhead(); }}
            className="px-1.5 py-0.5 bg-gray-700 hover:bg-gray-600 text-white text-xs rounded"
            title="Next Frame (→)"
          >
            ▶
          </button>
          <div className="w-px h-4 bg-gray-600" />
          {/* Zoom controls */}
          <button
            onClick={zoomOut}
            className="px-2 py-0.5 bg-gray-700 hover:bg-gray-600 text-white text-xs rounded"
            title="Zoom Out (-)"
          >
            −
          </button>
          <span className="text-gray-400 text-[10px] w-10 text-center">
            {Math.round(zoom * 100)}%
          </span>
          <button
            onClick={zoomIn}
            className="px-2 py-0.5 bg-gray-700 hover:bg-gray-600 text-white text-xs rounded"
            title="Zoom In (+)"
          >
            +
          </button>
          <button
            onClick={zoomFit}
            className="px-2 py-0.5 bg-gray-700 hover:bg-gray-600 text-white text-[10px] rounded"
            title="Fit to Window (\)"
          >
            Fit
          </button>
          <div className="w-px h-4 bg-gray-600" />
          <button
            onClick={() => setShowTextPanel(!showTextPanel)}
            className={`px-2 py-0.5 text-white text-[10px] rounded flex items-center gap-1 ${
              showTextPanel ? 'bg-blue-600 hover:bg-blue-500' : 'bg-gray-700 hover:bg-gray-600'
            }`}
            title="Toggle Text-Based Editing Panel"
          >
            📝 Text
          </button>
        </div>
      </div>

      {/* Main content area: Timeline + optional Text panel */}
      <div className="flex flex-col" style={{ flex: 1, minHeight: 0 }}>
        {/* Timeline area */}
        <div
          ref={timelineRef}
          className="relative overflow-hidden cursor-crosshair flex-shrink-0"
          style={{ height: timelineHeight }}
          onClick={handleTimelineClick}
          onWheel={handleWheel}
        >
          {/* Time ruler background */}
          <div
            className="absolute left-0 right-0 top-0 border-b border-gray-700"
            style={{
              height: RULER_HEIGHT,
              backgroundColor: '#202025',
              marginLeft: TRACK_CONTROL_WIDTH,
            }}
          >
            {renderTimeRuler()}
          </div>

          {/* Track controls background */}
          <div
            className="absolute left-0 top-0 bottom-0 border-r border-gray-700"
            style={{ width: TRACK_CONTROL_WIDTH, backgroundColor: '#1f1f23' }}
          />

          {/* Tracks */}
          {renderTracks()}
          {renderTrackControls()}

          {/* Playhead */}
          <div
            className="absolute top-0 bottom-0 z-20 pointer-events-none"
            style={{
              left: TRACK_CONTROL_WIDTH + playhead * pixelsPerSecond - scrollX,
            }}
          >
            {/* Playhead line */}
            <div
              className="absolute top-0 bottom-0 w-0.5 bg-red-500"
              style={{ boxShadow: '0 0 4px rgba(239, 68, 68, 0.5)' }}
            />
            {/* Playhead head */}
            <div
              className="absolute -top-0 -left-2 w-4 h-4 bg-red-500 cursor-ew-resize pointer-events-auto"
              style={{
                clipPath: 'polygon(0 0, 100% 0, 50% 100%)',
              }}
              onMouseDown={handlePlayheadMouseDown}
            />
          </div>

          {/* Left border for track controls area */}
          <div
            className="absolute top-0 bottom-0 w-px bg-gray-600"
            style={{ left: TRACK_CONTROL_WIDTH }}
          />
        </div>

        {/* Text-Based Editing panel */}
        {showTextPanel && (
          <div className="border-t border-gray-700" style={{ height: 280, minHeight: 200 }}>
            <TextBasedEditing
              transcript={transcript}
              currentTime={playhead}
              isPlaying={isPlaying}
              onSeek={(time) => {
                seekTo(time);
                scrollToPlayhead();
              }}
              onApplyEdits={(ranges) => {
                onApplyTextEdits?.(ranges);
              }}
              highlightColor="#4a9eff"
              isExpanded={showTextPanel}
              onToggleExpand={() => setShowTextPanel(false)}
            />
          </div>
        )}
      </div>

      {/* Status bar */}
      <div className="flex items-center justify-between px-3 py-1.5 bg-[#252528] border-t border-gray-700 text-[10px] text-gray-500">
        <span>
          Click timeline to focus • Home/End: jump • ←/→: frame step • Shift+←/→: 1s • C: center playhead • +/−: zoom • \: fit
        </span>
        <span>
          Duration: {totalDuration.toFixed(1)}s | {fps} fps
        </span>
      </div>
    </div>
  );
}
