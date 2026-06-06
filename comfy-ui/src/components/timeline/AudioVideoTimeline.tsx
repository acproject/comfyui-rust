import { useState, useCallback, useRef, useEffect } from 'react';
import { WaveformVisualizer } from './WaveformVisualizer';
import type { AudioVideoTimelineProps, TimelineState } from './TimelineTypes';

export function AudioVideoTimeline({
  videoSource,
  audioSource,
  // onTimeUpdate,
  onVideoOffsetChange,
  onAudioOffsetChange,
}: AudioVideoTimelineProps) {
  const [state, setState] = useState<TimelineState>({
    tracks: [
      {
        id: 'video',
        type: 'video',
        name: 'Video Track',
        clips: videoSource
          ? [
              {
                id: 'video-clip-1',
                type: 'video',
                startTime: 0,
                duration: 10,
                source: videoSource,
              },
            ]
          : [],
      },
      {
        id: 'audio',
        type: 'audio',
        name: 'Audio Track',
        clips: audioSource
          ? [
              {
                id: 'audio-clip-1',
                type: 'audio',
                startTime: 0,
                duration: 10,
                source: audioSource,
              },
            ]
          : [],
      },
    ],
    currentTime: 0,
    duration: 10,
    zoom: 1,
    scrollX: 0,
  });

  const timelineRef = useRef<HTMLDivElement>(null);
  const videoDragRef = useRef<{ isDragging: boolean; startX: number; startOffset: number }>({
    isDragging: false,
    startX: 0,
    startOffset: 0,
  });
  const audioDragRef = useRef<{ isDragging: boolean; startX: number; startOffset: number }>({
    isDragging: false,
    startX: 0,
    startOffset: 0,
  });

  const videoClip = state.tracks[0].clips[0];
  const audioClip = state.tracks[1].clips[0];

  const handleMouseDown = useCallback(
    (track: 'video' | 'audio', e: MouseEvent | React.MouseEvent) => {
      const ref = track === 'video' ? videoDragRef : audioDragRef;
      ref.current = {
        isDragging: true,
        startX: e.clientX,
        startOffset: track === 'video' ? (videoClip?.startTime || 0) : (audioClip?.startTime || 0),
      };
    },
    [videoClip, audioClip]
  );

  const handleMouseMove = useCallback(
    (e: MouseEvent) => {
      const { zoom } = state;
      const pixelsPerSecond = 50 * zoom;

      if (videoDragRef.current.isDragging) {
        const deltaX = e.clientX - videoDragRef.current.startX;
        const deltaTime = deltaX / pixelsPerSecond;
        const newStartTime = Math.max(0, videoDragRef.current.startOffset + deltaTime);

        setState((prev) => {
          const newTracks = [...prev.tracks];
          if (newTracks[0].clips.length > 0) {
            newTracks[0] = {
              ...newTracks[0],
              clips: [
                {
                  ...newTracks[0].clips[0],
                  startTime: newStartTime,
                },
              ],
            };
          }
          return { ...prev, tracks: newTracks };
        });

        onVideoOffsetChange?.(newStartTime);
      }

      if (audioDragRef.current.isDragging) {
        const deltaX = e.clientX - audioDragRef.current.startX;
        const deltaTime = deltaX / pixelsPerSecond;
        const newStartTime = Math.max(0, audioDragRef.current.startOffset + deltaTime);

        setState((prev) => {
          const newTracks = [...prev.tracks];
          if (newTracks[1].clips.length > 0) {
            newTracks[1] = {
              ...newTracks[1],
              clips: [
                {
                  ...newTracks[1].clips[0],
                  startTime: newStartTime,
                },
              ],
            };
          }
          return { ...prev, tracks: newTracks };
        });

        onAudioOffsetChange?.(newStartTime);
      }
    },
    [state.zoom, state.scrollX, onVideoOffsetChange, onAudioOffsetChange]
  );

  const handleMouseUp = useCallback(() => {
    videoDragRef.current.isDragging = false;
    audioDragRef.current.isDragging = false;
  }, []);

  useEffect(() => {
    window.addEventListener('mouseup', handleMouseUp);
    window.addEventListener('mousemove', handleMouseMove);
    return () => {
      window.removeEventListener('mouseup', handleMouseUp);
      window.removeEventListener('mousemove', handleMouseMove);
    };
  }, [handleMouseUp, handleMouseMove]);

  const handleZoomIn = useCallback(() => {
    setState((prev) => ({
      ...prev,
      zoom: Math.min(prev.zoom * 1.2, 5),
    }));
  }, []);

  const handleZoomOut = useCallback(() => {
    setState((prev) => ({
      ...prev,
      zoom: Math.max(prev.zoom / 1.2, 0.2),
    }));
  }, []);

  const pixelsPerSecond = 50 * state.zoom;
  const timelineWidth = state.duration * pixelsPerSecond;

  const renderTimeRuler = () => {
    const markers = [];
    const interval = state.zoom < 0.5 ? 5 : state.zoom < 2 ? 1 : 0.5;

    for (let t = 0; t <= state.duration; t += interval) {
      const x = t * pixelsPerSecond;
      markers.push(
        <div
          key={t}
          className="absolute text-xs text-gray-400"
          style={{ left: x, bottom: 0 }}
        >
          {t.toFixed(t < 1 ? 1 : 0)}s
        </div>
      );

      if (interval > 0.5) {
        for (let i = 1; i < (interval < 5 ? interval * 2 : 5); i++) {
          const subX = (t + i * (interval / (interval < 5 ? 10 : 5))) * pixelsPerSecond;
          if (subX < timelineWidth) {
            markers.push(
              <div
                key={`${t}-${i}`}
                className="absolute w-px h-1 bg-gray-600"
                style={{ left: subX, bottom: 0 }}
              />
            );
          }
        }
      }
    }
    return markers;
  };

  return (
    <div className="w-full bg-gray-800 rounded-lg border border-gray-700 overflow-hidden">
      <div className="flex items-center justify-between p-3 border-b border-gray-700">
        <h3 className="text-white font-medium">Audio/Video Timeline</h3>
        <div className="flex gap-2">
          <button
            onClick={handleZoomOut}
            className="px-3 py-1 bg-gray-700 hover:bg-gray-600 text-white text-sm rounded"
          >
            -
          </button>
          <span className="text-gray-400 text-sm self-center">
            {(state.zoom * 100).toFixed(0)}%
          </span>
          <button
            onClick={handleZoomIn}
            className="px-3 py-1 bg-gray-700 hover:bg-gray-600 text-white text-sm rounded"
          >
            +
          </button>
        </div>
      </div>

      <div
        ref={timelineRef}
        className="overflow-x-auto bg-gray-900"
        style={{ height: '220px' }}
      >
        <div
          className="relative"
          style={{ width: timelineWidth, height: '100%' }}
        >
          <div
            className="absolute left-0 right-0 bottom-0 h-8 border-t border-gray-700"
            style={{ zIndex: 1 }}
          >
            {renderTimeRuler()}
          </div>

          <div
            className="absolute top-0 w-0.5 bg-red-500"
            style={{
              left: state.currentTime * pixelsPerSecond,
              height: 'calc(100% - 32px)',
              zIndex: 10,
            }}
          />

          <div className="absolute top-0 left-0 right-0">
            <div className="h-20 border-b border-gray-700 flex items-center px-4">
              <span className="text-gray-400 text-sm w-24 flex-shrink-0">Video</span>
              {videoClip && (
                <div
                  className="flex-1 ml-2 h-14 bg-blue-600 rounded cursor-move flex items-center px-3"
                  style={{
                    marginLeft: videoClip.startTime * pixelsPerSecond,
                  }}
                  onMouseDown={(e) => handleMouseDown('video', e)}
                >
                  <span className="text-white text-sm">Video Clip</span>
                </div>
              )}
            </div>

            <div className="h-20 border-b border-gray-700 flex items-center px-4">
              <span className="text-gray-400 text-sm w-24 flex-shrink-0">Audio</span>
              {audioClip && (
                <div
                  className="flex-1 ml-2 h-14 bg-cyan-700 rounded cursor-move overflow-hidden"
                  style={{
                    marginLeft: audioClip.startTime * pixelsPerSecond,
                  }}
                  onMouseDown={(e) => handleMouseDown('audio', e)}
                >
                  <div className="p-2">
                    <WaveformVisualizer audioSource={audioSource} height={48} />
                  </div>
                </div>
              )}
            </div>
          </div>
        </div>
      </div>

      <div className="p-3 border-t border-gray-700 flex items-center justify-between">
        <div className="text-gray-400 text-xs">
          Drag clips to adjust timing offsets
        </div>
        <div className="text-gray-400 text-xs">
          Video offset: {videoClip?.startTime.toFixed(2) || 0}s | 
          Audio offset: {audioClip?.startTime.toFixed(2) || 0}s
        </div>
      </div>
    </div>
  );
}