import React, { useState, useEffect, useRef, useCallback } from 'react';

// ========== Types ==========

export interface TranscriptSegment {
  id: number;
  start: number;
  end: number;
  text: string;
  speaker?: string;
}

export interface TranscriptData {
  segments: TranscriptSegment[];
  language: string;
  duration_sec: number;
  model_used: string;
  full_text: string;
}

export interface TextEditAction {
  type: 'delete' | 'keep' | 'split';
  segmentId: number;
  /** For split actions */
  splitTime?: number;
  /** For edit text */
  newText?: string;
}

interface TextBasedEditingProps {
  /** Transcript data from WhisperTranscribe node */
  transcript: TranscriptData | null;
  /** Current playback time in seconds */
  currentTime?: number;
  /** Whether timeline is playing */
  isPlaying?: boolean;
  /** Called when user clicks a segment to seek */
  onSeek?: (time: number) => void;
  /** Called when user edits text (delete/reorder segments) - returns edited segments list */
  onSegmentsChange?: (segments: TranscriptSegment[], editActions: TextEditAction[]) => void;
  /** Called when user wants to apply text edits to create cut list */
  onApplyEdits?: (keepRanges: Array<{ start: number; end: number }>) => void;
  /** Optional: highlight color for active segment */
  highlightColor?: string;
  /** Whether the panel is visible */
  isExpanded?: boolean;
  onToggleExpand?: () => void;
}

// ========== Helpers ==========

function formatTime(seconds: number): string {
  if (!isFinite(seconds) || seconds < 0) return '0:00';
  const mins = Math.floor(seconds / 60);
  const secs = Math.floor(seconds % 60);
  const ms = Math.floor((seconds % 1) * 10);
  return `${mins}:${secs.toString().padStart(2, '0')}.${ms}`;
}

// ========== Component ==========

const TextBasedEditing: React.FC<TextBasedEditingProps> = ({
  transcript,
  currentTime = 0,
  isPlaying = false,
  onSeek,
  onSegmentsChange,
  onApplyEdits,
  highlightColor = '#4a9eff',
  isExpanded = true,
  onToggleExpand,
}) => {
  const [editingSegments, setEditingSegments] = useState<TranscriptSegment[]>([]);
  const [selectedSegmentId, setSelectedSegmentId] = useState<number | null>(null);
  const [deletedSegments, setDeletedSegments] = useState<Set<number>>(new Set());
  const [editingSegmentId, setEditingSegmentId] = useState<number | null>(null);
  const [editText, setEditText] = useState('');
  const containerRef = useRef<HTMLDivElement>(null);
  const activeSegmentRef = useRef<HTMLDivElement>(null);
  const editActionsRef = useRef<TextEditAction[]>([]);

  // Sync segments when transcript changes
  useEffect(() => {
    if (transcript) {
      setEditingSegments([...transcript.segments]);
      setDeletedSegments(new Set());
      editActionsRef.current = [];
    }
  }, [transcript]);

  // Auto-scroll to active segment during playback
  useEffect(() => {
    if (isPlaying && activeSegmentRef.current && containerRef.current) {
      const container = containerRef.current;
      const activeEl = activeSegmentRef.current;
      const containerRect = container.getBoundingClientRect();
      const activeRect = activeEl.getBoundingClientRect();
      
      if (activeRect.top < containerRect.top || activeRect.bottom > containerRect.bottom) {
        activeEl.scrollIntoView({ behavior: 'smooth', block: 'center' });
      }
    }
  }, [currentTime, isPlaying]);

  // Find active segment based on currentTime
  const activeSegment = editingSegments.find(
    (seg) => currentTime >= seg.start && currentTime < seg.end && !deletedSegments.has(seg.id)
  );

  const handleSegmentClick = useCallback((seg: TranscriptSegment) => {
    setSelectedSegmentId(seg.id);
    onSeek?.(seg.start);
  }, [onSeek]);

  const handleDeleteSegment = useCallback((segId: number, e: React.MouseEvent) => {
    e.stopPropagation();
    setDeletedSegments((prev) => {
      const next = new Set(prev);
      if (next.has(segId)) {
        next.delete(segId);
        // Remove the delete action
        editActionsRef.current = editActionsRef.current.filter(
          (a) => !(a.type === 'delete' && a.segmentId === segId)
        );
      } else {
        next.add(segId);
        editActionsRef.current.push({ type: 'delete', segmentId: segId });
      }
      return next;
    });
  }, []);

  const handleStartEdit = useCallback((seg: TranscriptSegment, e: React.MouseEvent) => {
    e.stopPropagation();
    setEditingSegmentId(seg.id);
    setEditText(seg.text);
  }, []);

  const handleSaveEdit = useCallback((segId: number) => {
    setEditingSegments((prev) =>
      prev.map((s) => (s.id === segId ? { ...s, text: editText.trim() } : s))
    );
    setEditingSegmentId(null);
    
    const action = editActionsRef.current.find(
      (a) => a.type === 'split' && a.segmentId === segId
    );
    if (action) {
      action.newText = editText.trim();
    } else {
      editActionsRef.current.push({ type: 'keep', segmentId: segId, newText: editText.trim() });
    }
  }, [editText]);

  const handleKeyDown = useCallback((e: React.KeyboardEvent, segId: number) => {
    if (e.key === 'Enter' && !e.shiftKey) {
      e.preventDefault();
      handleSaveEdit(segId);
    } else if (e.key === 'Escape') {
      setEditingSegmentId(null);
    }
  }, [handleSaveEdit]);

  const computeKeepRanges = useCallback((): Array<{ start: number; end: number }> => {
    if (!transcript) return [];
    
    const ranges: Array<{ start: number; end: number }> = [];
    let currentRange: { start: number; end: number } | null = null;
    
    // Sort segments by start time
    const sorted = [...editingSegments]
      .filter((s) => !deletedSegments.has(s.id))
      .sort((a, b) => a.start - b.start);
    
    for (const seg of sorted) {
      if (currentRange === null) {
        currentRange = { start: seg.start, end: seg.end };
      } else {
        // If gap is small (< 0.5s), merge; otherwise start new range
        const gap = seg.start - currentRange.end;
        if (gap < 0.5) {
          currentRange.end = seg.end;
        } else {
          ranges.push(currentRange);
          currentRange = { start: seg.start, end: seg.end };
        }
      }
    }
    if (currentRange) {
      ranges.push(currentRange);
    }
    
    return ranges;
  }, [transcript, editingSegments, deletedSegments]);

  const handleApplyEdits = useCallback(() => {
    const ranges = computeKeepRanges();
    onApplyEdits?.(ranges);
    onSegmentsChange?.(
      editingSegments.filter((s) => !deletedSegments.has(s.id)),
      editActionsRef.current
    );
  }, [computeKeepRanges, editingSegments, deletedSegments, onApplyEdits, onSegmentsChange]);

  const handleRestoreAll = useCallback(() => {
    setDeletedSegments(new Set());
    if (transcript) {
      setEditingSegments([...transcript.segments]);
    }
    editActionsRef.current = [];
  }, [transcript]);

  // Stats
  const totalWords = editingSegments.reduce((acc, s) => acc + s.text.split(/\s+/).filter(Boolean).length, 0);
  const keptSegments = editingSegments.filter((s) => !deletedSegments.has(s.id));
  const keptWords = keptSegments.reduce((acc, s) => acc + s.text.split(/\s+/).filter(Boolean).length, 0);
  const keptDuration = keptSegments.reduce((acc, s) => acc + (s.end - s.start), 0);
  const totalDuration = transcript?.duration_sec || 0;

  if (!transcript) {
    return (
      <div style={{
        background: '#1e1e1e',
        borderRadius: '8px',
        padding: '20px',
        color: '#888',
        fontSize: '13px',
        textAlign: 'center',
      }}>
        <div style={{ fontSize: '32px', marginBottom: '12px' }}>📝</div>
        <div>Text Based Editing</div>
        <div style={{ fontSize: '12px', marginTop: '8px', opacity: 0.7 }}>
          Connect a WhisperTranscribe node to enable text-based editing.
        </div>
        <div style={{ fontSize: '11px', marginTop: '12px', opacity: 0.5 }}>
          Click on words to jump to that point in the timeline.
          Delete filler words ("um", "uh", pauses) by striking through text.
        </div>
      </div>
    );
  }

  return (
    <div style={{
      background: '#1a1a1a',
      borderRadius: '8px',
      display: 'flex',
      flexDirection: 'column',
      height: '100%',
      overflow: 'hidden',
      border: '1px solid #333',
    }}>
      {/* Header */}
      <div style={{
        padding: '10px 14px',
        background: '#222',
        borderBottom: '1px solid #333',
        display: 'flex',
        alignItems: 'center',
        justifyContent: 'space-between',
        flexShrink: 0,
      }}>
        <div style={{ display: 'flex', alignItems: 'center', gap: '10px' }}>
          <span style={{ fontSize: '16px' }}>📝</span>
          <div>
            <div style={{ fontSize: '13px', fontWeight: 600, color: '#e0e0e0' }}>
              Text Based Editing
            </div>
            <div style={{ fontSize: '11px', color: '#888' }}>
              {transcript.language.toUpperCase()} · {transcript.model_used} · {keptSegments.length}/{editingSegments.length} segments
            </div>
          </div>
        </div>
        <div style={{ display: 'flex', gap: '8px', alignItems: 'center' }}>
          <span style={{ fontSize: '11px', color: '#888' }}>
            {formatTime(keptDuration)} / {formatTime(totalDuration)}
          </span>
          <button
            onClick={handleRestoreAll}
            style={{
              background: 'transparent',
              border: '1px solid #444',
              color: '#aaa',
              borderRadius: '4px',
              padding: '4px 10px',
              fontSize: '11px',
              cursor: 'pointer',
            }}
          >
            Restore All
          </button>
          <button
            onClick={handleApplyEdits}
            style={{
              background: highlightColor,
              border: 'none',
              color: '#fff',
              borderRadius: '4px',
              padding: '5px 14px',
              fontSize: '12px',
              fontWeight: 600,
              cursor: 'pointer',
            }}
          >
            Apply Cuts
          </button>
          {onToggleExpand && (
            <button
              onClick={onToggleExpand}
              style={{
                background: 'transparent',
                border: 'none',
                color: '#888',
                cursor: 'pointer',
                fontSize: '14px',
                padding: '4px',
              }}
            >
              {isExpanded ? '−' : '+'}
            </button>
          )}
        </div>
      </div>

      {/* Stats bar */}
      <div style={{
        padding: '6px 14px',
        background: '#1d1d1d',
        borderBottom: '1px solid #2a2a2a',
        display: 'flex',
        gap: '16px',
        fontSize: '11px',
        color: '#777',
        flexShrink: 0,
      }}>
        <span>{keptWords} / {totalWords} words kept</span>
        <span>{Math.round((keptDuration / Math.max(totalDuration, 0.01)) * 100)}% duration</span>
        <span>{deletedSegments.size} segments removed</span>
      </div>

      {/* Transcript segments */}
      <div
        ref={containerRef}
        style={{
          flex: 1,
          overflowY: 'auto',
          padding: '12px 14px',
          lineHeight: '1.8',
          fontSize: '14px',
        }}
      >
        {editingSegments.map((seg) => {
          const isDeleted = deletedSegments.has(seg.id);
          const isActive = activeSegment?.id === seg.id;
          const isSelected = selectedSegmentId === seg.id;
          const isEditing = editingSegmentId === seg.id;

          return (
            <div
              key={seg.id}
              ref={isActive && !isDeleted ? activeSegmentRef : null}
              onClick={() => !isEditing && handleSegmentClick(seg)}
              style={{
                display: 'inline',
                cursor: 'pointer',
                padding: '2px 0',
                marginRight: '4px',
                transition: 'all 0.15s ease',
              }}
            >
              {/* Timestamp tag (shown on hover or when active) */}
              <span
                style={{
                  fontSize: '10px',
                  color: isActive ? highlightColor : '#555',
                  marginRight: '6px',
                  fontFamily: 'monospace',
                  verticalAlign: 'super',
                  opacity: isActive || isSelected ? 1 : 0,
                  transition: 'opacity 0.2s',
                }}
                className="transcript-timestamp"
              >
                {formatTime(seg.start)}
              </span>

              {isEditing ? (
                <input
                  autoFocus
                  value={editText}
                  onChange={(e) => setEditText(e.target.value)}
                  onBlur={() => handleSaveEdit(seg.id)}
                  onKeyDown={(e) => handleKeyDown(e, seg.id)}
                  onClick={(e) => e.stopPropagation()}
                  style={{
                    background: '#333',
                    border: `1px solid ${highlightColor}`,
                    color: '#fff',
                    borderRadius: '3px',
                    padding: '1px 6px',
                    fontSize: '14px',
                    fontFamily: 'inherit',
                    outline: 'none',
                    minWidth: '200px',
                  }}
                />
              ) : (
                <span
                  onDoubleClick={(e) => handleStartEdit(seg, e)}
                  style={{
                    color: isDeleted ? '#555' : isActive ? '#fff' : '#ccc',
                    textDecoration: isDeleted ? 'line-through' : 'none',
                    opacity: isDeleted ? 0.4 : 1,
                    background: isActive
                      ? `${highlightColor}33`
                      : isSelected
                      ? '#ffffff10'
                      : 'transparent',
                    borderRadius: '3px',
                    padding: '1px 3px',
                    transition: 'all 0.15s ease',
                  }}
                  title={`${formatTime(seg.start)} - ${formatTime(seg.end)} · Double-click to edit · Click to seek`}
                >
                  {seg.text}
                </span>
              )}

              {/* Delete button on hover */}
              {!isEditing && (
                <span
                  onClick={(e) => handleDeleteSegment(seg.id, e)}
                  style={{
                    fontSize: '11px',
                    color: isDeleted ? '#4a9eff' : '#666',
                    marginLeft: '2px',
                    cursor: 'pointer',
                    opacity: 0,
                    transition: 'opacity 0.15s',
                    verticalAlign: 'middle',
                  }}
                  className="transcript-delete-btn"
                  title={isDeleted ? 'Restore segment' : 'Delete segment (strike through)'}
                >
                  {isDeleted ? '↩' : '✕'}
                </span>
              )}
            </div>
          );
        })}

        {editingSegments.length === 0 && (
          <div style={{ color: '#666', textAlign: 'center', padding: '40px' }}>
            No transcript segments available.
          </div>
        )}
      </div>

      {/* Footer with tips */}
      <div style={{
        padding: '8px 14px',
        background: '#1d1d1d',
        borderTop: '1px solid #2a2a2a',
        fontSize: '10px',
        color: '#555',
        flexShrink: 0,
        display: 'flex',
        justifyContent: 'space-between',
      }}>
        <span>Click: seek · Double-click: edit · ✕: strike to delete</span>
        <span>Similar to Descript / Premiere Text-Based Editing</span>
      </div>

      {/* Hover styles (injected) */}
      <style>{`
        .transcript-timestamp { display: none; }
        div:hover > .transcript-timestamp,
        .transcript-timestamp:hover { display: inline; opacity: 1 !important; }
        div:hover > .transcript-delete-btn { opacity: 0.7; }
        .transcript-delete-btn:hover { opacity: 1 !important; color: #ff6b6b !important; }
      `}</style>
    </div>
  );
};

export default TextBasedEditing;
