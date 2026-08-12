import { memo, type FC, useCallback, useRef, useState, useEffect } from 'react';
import { Handle, Position } from '@xyflow/react';
import type { ComfyNodeData } from '@/store/workflow';
import { useWorkflowStore } from '@/store/workflow';
import { api } from '@/api/client';
import { getTypeColor, getCategoryColor, isCustomNode } from '@/components/nodes/nodeColors';
import { AudioVideoTimeline } from '@/components/timeline/AudioVideoTimeline';
import { PromptRelayTimelineEditor } from '@/components/timeline/PromptRelayTimelineEditor';
import { LtxDirectorTimeline } from '@/components/timeline/LtxDirectorTimeline';
import { PremiereTimeline } from '@/components/timeline/PremiereTimeline';

// Node types that support custom resizing
const RESIZABLE_NODE_TYPES = new Set([
  'PromptRelayEncodeTimeline',
  'LTXDirector',
  'VideoTimeline',
]);

const DEFAULT_NODE_WIDTH = 280;
const MIN_NODE_WIDTH = 220;
const MIN_NODE_HEIGHT = 200;
const DEFAULT_TIMELINE_HEIGHT = 160;

interface ComfyNodeProps {
  id: string;
  data: ComfyNodeData;
  selected: boolean;
}

const HEADER_H = 28;
const ROW_H = 24;
const SEP_H = 2;

const ComfyNodeComponent: FC<ComfyNodeProps> = memo(({ id, data, selected }) => {
  const { title, outputs, isOutputNode, category, classType } = data;
  const headerColor = getCategoryColor(category);
  const [collapsed, setCollapsed] = useState(false);

  const objectInfo = useWorkflowStore((s) => s.objectInfo[data.classType]);
  const executingNodeId = useWorkflowStore((s) => s.executingNodeId);
  const executedNodeIds = useWorkflowStore((s) => s.executedNodeIds);
  const cachedNodeIds = useWorkflowStore((s) => s.cachedNodeIds);
  const isExecuting = executingNodeId === id;
  const isExecuted = executedNodeIds.includes(id);
  const isCached = cachedNodeIds.includes(id);
  const isCompleted = isExecuted || isCached;

  const allInputSpecs: Array<{
    name: string;
    typeName: string;
    optional: boolean;
    choices?: string[];
    multiline?: boolean;
    timelineHidden?: boolean;
  }> = [];

  if (objectInfo?.input_types?.required) {
    for (const [k, v] of Object.entries(objectInfo.input_types.required)) {
      const spec = v as { type_name: string; extra?: Record<string, unknown> };
      allInputSpecs.push({
        name: k,
        typeName: spec.type_name,
        optional: false,
        choices: (spec.extra?.choices as string[]) || undefined,
        multiline: spec.extra?.multiline === true || (k === 'text' && spec.type_name === 'STRING'),
        timelineHidden: spec.extra?.timelineHidden === true,
      });
    }
  }
  if (objectInfo?.input_types?.optional) {
    for (const [k, v] of Object.entries(objectInfo.input_types.optional)) {
      const spec = v as { type_name: string; extra?: Record<string, unknown> };
      allInputSpecs.push({
        name: k,
        typeName: spec.type_name,
        optional: true,
        choices: (spec.extra?.choices as string[]) || undefined,
        multiline: spec.extra?.multiline === true || (k === 'text' && spec.type_name === 'STRING'),
        timelineHidden: spec.extra?.timelineHidden === true,
      });
    }
  }

  const isImageNode = classType === 'LoadImage';
  const isSaveImageNode = classType === 'SaveImage';
  const isSaveVideoNode = classType === 'SaveVideo';
  const isSaveAudioNode = classType === 'SaveAudio';
  const isLoadAudioNode = classType === 'LoadAudio';
  const isSaveVideoWithAudioNode = classType === 'SaveVideoWithAudio';
  const isPromptRelayTimeline = classType === 'PromptRelayEncodeTimeline';
  const isLtxDirector = classType === 'LTXDirector';
  const isVideoTimeline = classType === 'VideoTimeline';

  const isResizable = RESIZABLE_NODE_TYPES.has(classType);
  const customWidth = (data.customWidth as number) || (isVideoTimeline ? 900 : DEFAULT_NODE_WIDTH);
  const customHeight = (data.customHeight as number) || (isVideoTimeline ? 520 : DEFAULT_TIMELINE_HEIGHT);

  // Resize drag state
  const [isResizing, setIsResizing] = useState(false);
  const resizeStartRef = useRef<{ x: number; y: number; w: number; h: number } | null>(null);

  const handleResizeMouseDown = useCallback((e: React.MouseEvent) => {
    e.stopPropagation();
    e.preventDefault();
    setIsResizing(true);
    resizeStartRef.current = {
      x: e.clientX,
      y: e.clientY,
      w: customWidth,
      h: customHeight,
    };
  }, [customWidth, customHeight]);

  useEffect(() => {
    if (!isResizing) return;
    const handleMouseMove = (e: MouseEvent) => {
      if (!resizeStartRef.current) return;
      const dx = e.clientX - resizeStartRef.current.x;
      const dy = e.clientY - resizeStartRef.current.y;
      const newW = Math.max(MIN_NODE_WIDTH, resizeStartRef.current.w + dx);
      const newH = Math.max(MIN_NODE_HEIGHT, resizeStartRef.current.h + dy);
      useWorkflowStore.getState().updateNodeData(id, {
        customWidth: newW,
        customHeight: newH,
      });
    };
    const handleMouseUp = () => {
      setIsResizing(false);
      resizeStartRef.current = null;
    };
    window.addEventListener('mousemove', handleMouseMove);
    window.addEventListener('mouseup', handleMouseUp);
    return () => {
      window.removeEventListener('mousemove', handleMouseMove);
      window.removeEventListener('mouseup', handleMouseUp);
    };
  }, [isResizing, id]);

  const isPrimitive = (typeName: string) =>
    ['INT', 'FLOAT', 'STRING', 'BOOLEAN', 'COMBO'].includes(typeName);

  const nonPrimitiveInputs = allInputSpecs.filter((s) => !isPrimitive(s.typeName));

  let y = HEADER_H;
  if (!collapsed) {
    if (isImageNode) y += 80;
    if (isSaveImageNode) y += 80;
    if (isSaveVideoNode) y += 80;
    if (isSaveAudioNode) y += 60;
    if (isLoadAudioNode) y += 60;
    if (isSaveVideoWithAudioNode) y += 260;
    if (isPromptRelayTimeline) y += customHeight;
    if (isLtxDirector) y += customHeight + 60;
    if (isVideoTimeline) y += customHeight;
  }

  const inputHandleY: Record<string, number> = {};
  for (const spec of allInputSpecs) {
    const rowH = (!collapsed && spec.multiline) ? ROW_H * 3 : ROW_H;
    inputHandleY[spec.name] = y + rowH / 2;
    y += rowH;
  }

  if (!collapsed && outputs.length > 0) {
    y += SEP_H;
  }

  // Output rows height (for minHeight calculation)
  for (const _output of outputs) {
    y += ROW_H;
  }

  return (
    <div
      style={{
        background: '#1e1e2e',
        borderRadius: 6,
        border: isExecuting
          ? '2px solid #f59e0b'
          : isCompleted
            ? '2px solid #22c55e'
            : selected
              ? '2px solid #fff'
              : '1px solid #333',
        minWidth: MIN_NODE_WIDTH,
        width: isResizable ? customWidth : undefined,
        maxWidth: isResizable ? 1200 : DEFAULT_NODE_WIDTH,
        minHeight: isResizable ? Math.min(MIN_NODE_HEIGHT, 815) : undefined,
        fontSize: 12,
        color: '#e2e8f0',
        boxShadow: isExecuting
          ? '0 0 16px rgba(245, 158, 11, 0.6), 0 16px rgba(245, 158, 11, 0.6), 0 0 4px rgba(245, 158, 11, 0.3)'
          : isCompleted
            ? '0 0 12px rgba(34, 197, 94, 0.4)'
            : selected
              ? '0 0 12px rgba(100, 150, 255, 0.4)'
              : '0 2px 8px rgba(0,0,0,0.3)',
        transition: isResizing ? 'none' : 'border-color 0.3s, box-shadow 0.3s',
        position: 'relative',
      }}
    >
      {nonPrimitiveInputs.map((spec) => (
        <Handle
          key={`in-${spec.name}`}
          type="target"
          position={Position.Left}
          id={spec.name}
          style={{
            background: getTypeColor(spec.typeName),
            width: 12,
            height: 12,
            border: '2px solid #1e1e2e',
            top: inputHandleY[spec.name],
          }}
        />
      ))}

      <div
        style={{
          background: headerColor,
          padding: '5px 8px',
          fontWeight: 600,
          fontSize: 11,
          display: 'flex',
          justifyContent: 'space-between',
          alignItems: 'center',
          borderRadius: '4px 4px 0 0',
          cursor: 'pointer',
          userSelect: 'none',
          height: HEADER_H,
          boxSizing: 'border-box',
        }}
        onDoubleClick={() => setCollapsed(!collapsed)}
      >
        <span style={{ overflow: 'hidden', textOverflow: 'ellipsis', whiteSpace: 'nowrap', flex: 1 }}>
          {title}
        </span>
        <div style={{ display: 'flex', alignItems: 'center', gap: 4 }}>
          {isExecuting && (
            <span style={{
              fontSize: 8,
              background: 'rgba(245, 158, 11, 0.3)',
              color: '#f59e0b',
              padding: '1px 4px',
              borderRadius: 3,
              animation: 'pulse 1.5s ease-in-out infinite',
            }}>
              ●
            </span>
          )}
          {isCompleted && !isExecuting && (
            <span style={{
              fontSize: 8,
              background: 'rgba(34, 197, 94, 0.3)',
              color: '#22c55e',
              padding: '1px 4px',
              borderRadius: 3,
            }}>
              ✓
            </span>
          )}
          {isCustomNode(classType) && (
            <span style={{
              fontSize: 8,
              background: 'rgba(139,107,191,0.3)',
              color: '#b39ddb',
              padding: '1px 4px',
              borderRadius: 3,
            }}>
              custom
            </span>
          )}
          <span style={{ fontSize: 9, opacity: 0.6 }}>#{id}</span>
          {isOutputNode && (
            <span style={{
              fontSize: 8,
              background: 'rgba(255,255,255,0.2)',
              padding: '1px 4px',
              borderRadius: 3,
            }}>
              SAVE
            </span>
          )}
          <span style={{ fontSize: 9, opacity: 0.5 }}>{collapsed ? '▸' : '▾'}</span>
        </div>
      </div>

      {!collapsed && (
        <>
          {(isImageNode) && (
            <ImagePreview filename={data.inputs['image'] ? String(data.inputs['image']) : ''} type="input" />
          )}
          {(isSaveImageNode) && (
            <OutputPreview nodeId={id} />
          )}
          {(isSaveVideoNode) && (
            <VideoPreview nodeId={id} />
          )}
          {(isSaveAudioNode) && (
            <AudioPreview nodeId={id} />
          )}
          {(isLoadAudioNode) && (
            <AudioPreview nodeId={id} />
          )}
          {(isSaveVideoWithAudioNode) && (
            <VideoWithAudioTimeline nodeId={id} />
          )}
          {(isPromptRelayTimeline) && (
            <PromptRelayTimelineEditor
              nodeId={id}
              maxFrames={Number(data.inputs['max_frames'] || data.inputs['duration_frames'] || 129)}
              fps={Number(data.inputs['fps'] || data.inputs['frame_rate'] || 24.0)}
              timeUnits={String(data.inputs['time_units'] || data.inputs['display_mode'] || 'frames')}
              localPrompts={String(data.inputs['local_prompts'] || '')}
              segmentLengths={String(data.inputs['segment_lengths'] || '')}
              timelineData={String(data.inputs['timeline_data'] || '')}
              height={customHeight}
            />
          )}
          {(isLtxDirector) && (
            <LtxDirectorTimeline
              nodeId={id}
              maxFrames={Number(data.inputs['duration_frames'] || 120)}
              fps={Number(data.inputs['frame_rate'] || 24.0)}
              timeUnits={String(data.inputs['display_mode'] || 'seconds')}
              localPrompts={String(data.inputs['local_prompts'] || '')}
              segmentLengths={String(data.inputs['segment_lengths'] || '')}
              timelineData={String(data.inputs['timeline_data'] || '')}
              height={customHeight + 60}
            />
          )}
          {(isVideoTimeline) && (
            <div style={{ padding: '4px 6px', borderBottom: '1px solid #333' }}>
              <PremiereTimeline
                totalDuration={Number(data.inputs['total_duration'] || 10)}
                fps={Number(data.inputs['fps'] || 24)}
                height={customHeight - 16}
                width={customWidth - 24}
              />
            </div>
          )}

          <div style={{ padding: '2px 0' }}>
            {allInputSpecs.filter((s) => !s.timelineHidden).map((spec) => {
              const { name, typeName, choices, multiline } = spec;
              const value = data.inputs[name];
              const typeColor = getTypeColor(typeName);
              const showHandle = !isPrimitive(typeName);

              return (
                <div
                  key={name}
                  style={{
                    padding: '2px 8px',
                    display: 'flex',
                    alignItems: multiline ? 'flex-start' : 'center',
                    gap: 4,
                    height: multiline ? ROW_H * 3 : ROW_H,
                    boxSizing: 'border-box',
                    paddingLeft: showHandle ? 18 : 8,
                  }}
                >
                  {showHandle && (
                    <span style={{
                      width: 8,
                      height: 8,
                      borderRadius: '50%',
                      background: typeColor,
                      border: '1.5px solid #1e1e2e',
                      flexShrink: 0,
                      marginTop: multiline ? 7 : 0,
                    }} />
                  )}
                  <span style={{
                    flex: '0 0 auto',
                    fontSize: 10,
                    color: '#a0aec0',
                    overflow: 'hidden',
                    textOverflow: 'ellipsis',
                    whiteSpace: 'nowrap',
                    lineHeight: multiline ? undefined : '20px',
                    paddingTop: multiline ? 3 : 0,
                  }}>
                    {name}
                  </span>
                  {isPrimitive(typeName) && (
                    <NodeInputField
                      nodeId={id}
                      name={name}
                      value={value}
                      typeName={typeName}
                      choices={choices}
                      classType={classType}
                      multiline={multiline}
                    />
                  )}
                </div>
              );
            })}
          </div>

          {outputs.length > 0 && (
            <>
              <div style={{ height: SEP_H, background: '#333' }} />
              <div style={{ padding: '2px 0', paddingBottom: isResizable ? 18 : 2 }}>
                {outputs.map((output) => {
                  const typeColor = getTypeColor(output.type);
                  return (
                    <div
                      key={output.name}
                      style={{
                        padding: '2px 8px',
                        fontSize: 10,
                        color: '#a0aec0',
                        height: ROW_H,
                        display: 'flex',
                        alignItems: 'center',
                        justifyContent: 'space-between',
                        gap: 4,
                        boxSizing: 'border-box',
                        paddingRight: isResizable ? 24 : 18,
                        position: 'relative',
                      }}
                    >
                      <span style={{ overflow: 'hidden', textOverflow: 'ellipsis', whiteSpace: 'nowrap' }}>
                        {output.name}
                      </span>
                      <div style={{ display: 'flex', alignItems: 'center', gap: 4, flexShrink: 0 }}>
                      <span style={{
                        fontSize: 8,
                        color: typeColor,
                        background: `${typeColor}22`,
                        padding: '0px 3px',
                        borderRadius: 2,
                      }}>
                        {output.type}
                      </span>
                      <Handle
                        type="source"
                        position={Position.Right}
                        id={output.name}
                        style={{
                          background: typeColor,
                          width: 12,
                          height: 12,
                          border: '2px solid #1e1e2e',
                          position: 'absolute',
                          right: -6,
                          top: '50%',
                          transform: 'translateY(-50%)',
                        }}
                      />
                      </div>
                    </div>
                  );
                })}
              </div>
            </>
          )}
        </>
      )}
      {/* Resize handle for resizable nodes */}
      {isResizable && !collapsed && (
        <div
          className="nodrag"
          onMouseDown={handleResizeMouseDown}
          style={{
            position: 'absolute',
            right: 0,
            bottom: 0,
            width: 18,
            height: 18,
            cursor: 'nwse-resize',
            display: 'flex',
            alignItems: 'flex-end',
            justifyContent: 'flex-end',
            padding: '0 3px 3px 0',
            opacity: 0.7,
            zIndex: 10,
            transition: 'opacity 0.15s',
          }}
          onMouseEnter={(e) => { (e.currentTarget.style.opacity = '1'); }}
          onMouseLeave={(e) => { (e.currentTarget.style.opacity = '0.7'); }}
        >
          <svg width="10" height="10" viewBox="0 0 10 10">
            <line x1="9" y1="1" x2="1" y2="9" stroke="#ccc" strokeWidth="1.5" />
            <line x1="9" y1="5" x2="5" y2="9" stroke="#ccc" strokeWidth="1.5" />
          </svg>
        </div>
      )}
    </div>
  );
});

ComfyNodeComponent.displayName = 'ComfyNode';

const ImagePreview: FC<{ filename: string; type: string }> = memo(({ filename, type }) => {
  if (!filename) {
    return (
      <div style={{
        padding: '4px 6px',
        borderBottom: '1px solid #333',
        height: 72,
        display: 'flex',
        alignItems: 'center',
        justifyContent: 'center',
        background: '#111',
        borderRadius: 3,
        margin: '4px 6px',
      }}>
        <span style={{ fontSize: 10, color: '#666' }}>Preview</span>
      </div>
    );
  }
  const url = type === 'input'
    ? api.getInputImageUrl(filename)
    : `${window.location.origin}/view?filename=${encodeURIComponent(filename)}`;

  return (
    <div style={{
      padding: '4px 6px',
      borderBottom: '1px solid #333',
      height: 80,
      overflow: 'hidden',
    }}>
      <img
        src={url}
        alt={filename}
        style={{
          width: '100%',
          height: '100%',
          objectFit: 'contain',
          borderRadius: 3,
          background: '#111',
          display: 'block',
        }}
        onError={(e) => {
          (e.target as HTMLImageElement).style.display = 'none';
        }}
      />
    </div>
  );
});

ImagePreview.displayName = 'ImagePreview';

const OutputPreview: FC<{ nodeId: string }> = memo(({ nodeId }) => {
  const outputImages = useWorkflowStore((s) => s.outputImages);
  const images = outputImages[nodeId];

  if (!images || images.length === 0) {
    return (
      <div style={{
        padding: '4px 6px',
        borderBottom: '1px solid #333',
        height: 72,
        display: 'flex',
        alignItems: 'center',
        justifyContent: 'center',
        background: '#111',
        borderRadius: 3,
        margin: '4px 6px',
      }}>
        <span style={{ fontSize: 10, color: '#666' }}>Preview</span>
      </div>
    );
  }

  return (
    <div style={{
      padding: '4px 6px',
      borderBottom: '1px solid #333',
      height: 80,
      overflow: 'hidden',
    }}>
      {images.map((img, i) => (
        <img
          key={i}
          src={`${window.location.origin}/view?filename=${encodeURIComponent(img.filename)}&subfolder=${encodeURIComponent(img.subfolder || '')}`}
          alt={img.filename}
          style={{
            width: '100%',
            height: '100%',
            objectFit: 'contain',
            borderRadius: 3,
            background: '#111',
            display: 'block',
          }}
          onError={(e) => {
            (e.target as HTMLImageElement).style.display = 'none';
          }}
        />
      ))}
    </div>
  );
});

OutputPreview.displayName = 'OutputPreview';

const VideoPreview: FC<{ nodeId: string }> = memo(({ nodeId }) => {
  const outputImages = useWorkflowStore((s) => s.outputImages);
  const videos = outputImages[nodeId];

  if (!videos || videos.length === 0) {
    return (
      <div style={{
        padding: '4px 6px',
        borderBottom: '1px solid #333',
        height: 72,
        display: 'flex',
        alignItems: 'center',
        justifyContent: 'center',
        background: '#111',
        borderRadius: 3,
        margin: '4px 6px',
      }}>
        <span style={{ fontSize: 10, color: '#666' }}>Video Preview</span>
      </div>
    );
  }

  return (
    <div style={{
      padding: '4px 6px',
      borderBottom: '1px solid #333',
      height: 80,
      overflow: 'hidden',
    }}>
      {videos.map((vid, i) => {
        const ext = vid.filename.split('.').pop()?.toLowerCase() || '';
        const url = `${window.location.origin}/view_video?filename=${encodeURIComponent(vid.filename)}&subfolder=${encodeURIComponent(vid.subfolder || '')}`;
        if (['mp4', 'webm', 'mov', 'avi'].includes(ext)) {
          return (
            <video
              key={i}
              src={url}
              controls
              muted
              loop
              style={{
                width: '100%',
                height: '100%',
                objectFit: 'contain',
                borderRadius: 3,
                background: '#111',
                display: 'block',
              }}
              onError={(e) => {
                (e.target as HTMLVideoElement).style.display = 'none';
              }}
            />
          );
        }
        return (
          <img
            key={i}
            src={url}
            alt={vid.filename}
            style={{
              width: '100%',
              height: '100%',
              objectFit: 'contain',
              borderRadius: 3,
              background: '#111',
              display: 'block',
            }}
            onError={(e) => {
              (e.target as HTMLImageElement).style.display = 'none';
            }}
          />
        );
      })}
    </div>
  );
});

VideoPreview.displayName = 'VideoPreview';

const AudioPreview: FC<{ nodeId: string }> = memo(({ nodeId }) => {
  const outputAudios = useWorkflowStore((s) => s.outputAudios);
  const audios = outputAudios[nodeId];

  if (!audios || audios.length === 0) {
    return (
      <div style={{
        padding: '4px 6px',
        borderBottom: '1px solid #333',
        height: 50,
        display: 'flex',
        alignItems: 'center',
        justifyContent: 'center',
        background: '#111',
        borderRadius: 3,
        margin: '4px 6px',
      }}>
        <span style={{ fontSize: 10, color: '#666' }}>Audio Preview</span>
      </div>
    );
  }

  return (
    <div style={{
      padding: '4px 6px',
      borderBottom: '1px solid #333',
    }}>
      {audios.map((aud, i) => {
        const url = `${window.location.origin}/view_audio?filename=${encodeURIComponent(aud.filename)}&subfolder=${encodeURIComponent(aud.subfolder || '')}`;
        return (
          <div key={i} style={{
            background: '#1a1a2e',
            borderRadius: 3,
            padding: '4px 6px',
            marginBottom: 2,
          }}>
            <div style={{ fontSize: 9, color: '#888', marginBottom: 2, overflow: 'hidden', textOverflow: 'ellipsis', whiteSpace: 'nowrap' }}>
              {aud.filename}
            </div>
            <audio
              src={url}
              controls
              style={{
                width: '100%',
                height: 28,
                borderRadius: 2,
              }}
            />
          </div>
        );
      })}
    </div>
  );
});

AudioPreview.displayName = 'AudioPreview';

const VideoWithAudioTimeline: FC<{ nodeId: string }> = memo(({ nodeId }) => {
  const outputImages = useWorkflowStore((s) => s.outputImages);
  const outputAudios = useWorkflowStore((s) => s.outputAudios);
  
  const videos = outputImages[nodeId] || [];
  const audios = outputAudios[nodeId] || [];

  let videoUrl: string | undefined;
  let audioUrl: string | undefined;

  if (videos.length > 0) {
    videoUrl = `${window.location.origin}/view_video?filename=${encodeURIComponent(videos[0].filename)}&subfolder=${encodeURIComponent(videos[0].subfolder || '')}`;
  }

  if (audios.length > 0) {
    audioUrl = `${window.location.origin}/view_audio?filename=${encodeURIComponent(audios[0].filename)}&subfolder=${encodeURIComponent(audios[0].subfolder || '')}`;
  }

  if (!videoUrl && !audioUrl) {
    return (
      <div style={{
        padding: '4px 6px',
        borderBottom: '1px solid #333',
      }}>
        <div style={{
          height: 240,
          display: 'flex',
          alignItems: 'center',
          justifyContent: 'center',
          background: '#111',
          borderRadius: 3,
          margin: '4px 0',
        }}>
          <span style={{ fontSize: 10, color: '#666' }}>Timeline Preview</span>
        </div>
      </div>
    );
  }

  return (
    <div style={{
      padding: '4px 6px',
      borderBottom: '1px solid #333',
    }}>
      <AudioVideoTimeline
        videoSource={videoUrl}
        audioSource={audioUrl}
      />
    </div>
  );
});

VideoWithAudioTimeline.displayName = 'VideoWithAudioTimeline';

interface NodeInputFieldProps {
  nodeId: string;
  name: string;
  value: unknown;
  typeName: string;
  choices?: string[];
  classType: string;
  multiline?: boolean;
}

const NodeInputField: FC<NodeInputFieldProps> = memo(({ nodeId, name, value, typeName, choices, classType, multiline }) => {
  const updateNodeInput = useWorkflowStore((s) => s.updateNodeInput);
  const fileInputRef = useRef<HTMLInputElement>(null);

  const handleChange = useCallback(
    (newValue: unknown) => {
      updateNodeInput(nodeId, name, newValue);
    },
    [nodeId, name, updateNodeInput]
  );

  const handleUpload = useCallback(
    async (e: React.ChangeEvent<HTMLInputElement>) => {
      const file = e.target.files?.[0];
      if (!file) return;
      try {
        const result = await api.uploadInputImage(file);
        handleChange(result.name);
      } catch (err) {
        console.error('Failed to upload image:', err);
      }
    },
    [handleChange]
  );

  const baseStyle: React.CSSProperties = {
    background: '#2a2a3e',
    border: '1px solid #444',
    borderRadius: 3,
    color: '#e2e8f0',
    padding: '1px 4px',
    fontSize: 10,
    outline: 'none',
    flex: 1,
    minWidth: 60,
  };

  if (typeName === 'BOOLEAN') {
    return (
      <input
        type="checkbox"
        checked={!!value}
        onChange={(e) => handleChange(e.target.checked)}
        style={{ width: 12, height: 12, cursor: 'pointer', marginTop: 4 }}
      />
    );
  }

  if (typeName === 'INT') {
    return (
      <input
        type="number"
        value={Number(value) || 0}
        onChange={(e) => handleChange(parseInt(e.target.value, 10) || 0)}
        style={{ ...baseStyle, width: 70 }}
      />
    );
  }

  if (typeName === 'FLOAT') {
    return (
      <input
        type="number"
        step="0.1"
        value={Number(value) || 0}
        onChange={(e) => handleChange(parseFloat(e.target.value) || 0)}
        style={{ ...baseStyle, width: 70 }}
      />
    );
  }

  if (typeName === 'STRING') {
    if (multiline) {
      return (
        <textarea
          value={String(value || '')}
          onChange={(e) => handleChange(e.target.value)}
          rows={3}
          style={{
            ...baseStyle,
            width: '100%',
            resize: 'vertical',
            fontFamily: 'inherit',
            lineHeight: 1.4,
          }}
        />
      );
    }
    return (
      <input
        type="text"
        value={String(value || '')}
        onChange={(e) => handleChange(e.target.value)}
        style={{ ...baseStyle, width: 90 }}
      />
    );
  }

  if (typeName === 'COMBO') {
    const isImageField = classType === 'LoadImage' && name === 'image';
    const choicesArray = choices || [];
    const currentValue = String(value || '');
    // Ensure value matches one of the choices, otherwise use first choice or empty string
    const safeValue = choicesArray.length > 0 && choicesArray.includes(currentValue)
      ? currentValue
      : (choicesArray.length > 0 ? choicesArray[0] : '');

    return (
      <div style={{ display: 'flex', alignItems: 'center', gap: 2, flex: 1 }}>
        <select
          value={safeValue}
          onChange={(e) => handleChange(e.target.value)}
          style={{
            ...baseStyle,
            width: isImageField ? 70 : 90,
            cursor: 'pointer',
          }}
        >
          {choicesArray.length === 0 && (
            <option value="" disabled>No options available</option>
          )}
          {choicesArray.map((c) => (
            <option key={c} value={c}>
              {c.length > 36 ? c.slice(0, 33) + '…' : c}
            </option>
          ))}
        </select>
        {isImageField && (
          <>
            <button
              onClick={() => fileInputRef.current?.click()}
              title="Upload image"
              style={{
                background: '#4a5568',
                border: '1px solid #5a6578',
                borderRadius: 3,
                color: '#e2e8f0',
                padding: '1px 5px',
                fontSize: 10,
                cursor: 'pointer',
                lineHeight: 1,
              }}
            >
              📁
            </button>
            <input
              ref={fileInputRef}
              type="file"
              accept="image/*"
              style={{ display: 'none' }}
              onChange={handleUpload}
            />
          </>
        )}
      </div>
    );
  }

  return <span style={{ fontSize: 9, color: '#718096' }}>{String(value)}</span>;
});

NodeInputField.displayName = 'NodeInputField';

export { ComfyNodeComponent };
