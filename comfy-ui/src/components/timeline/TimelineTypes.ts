export interface TimelineClip {
  id: string;
  type: 'video' | 'audio';
  startTime: number;
  duration: number;
  source?: File | string;
}

export interface TimelineTrack {
  id: string;
  type: 'video' | 'audio';
  name: string;
  clips: TimelineClip[];
}

export interface TimelineState {
  tracks: TimelineTrack[];
  currentTime: number;
  duration: number;
  zoom: number;
  scrollX: number;
}

export interface AudioVideoTimelineProps {
  videoSource?: File | string;
  audioSource?: File | string;
  onTimeUpdate?: (time: number) => void;
  onVideoOffsetChange?: (offset: number) => void;
  onAudioOffsetChange?: (offset: number) => void;
}