import { type FC } from 'react';
import { useWorkflowStore } from '@/store/workflow';

const ProgressBar: FC = () => {
  const progress = useWorkflowStore((s) => s.progress);
  const executingPromptId = useWorkflowStore((s) => s.executingPromptId);
  const executingNodeId = useWorkflowStore((s) => s.executingNodeId);

  if (!executingPromptId) return null;

  return (
    <div
      style={{
        position: 'absolute',
        bottom: 0,
        left: 0,
        right: 0,
        background: '#1a202c',
        borderTop: '1px solid #2d3748',
        padding: '6px 12px',
        display: 'flex',
        alignItems: 'center',
        gap: 12,
        fontSize: 12,
        color: '#a0aec0',
        zIndex: 10,
      }}
    >
      <span>
        Executing: {executingNodeId || '...'}
      </span>
      {progress && (
        <>
          <div
            style={{
              flex: 1,
              height: 6,
              background: '#2d3748',
              borderRadius: 3,
              overflow: 'hidden',
            }}
          >
            <div
              style={{
                width: `${(progress.value / progress.max) * 100}%`,
                height: '100%',
                background: '#5a6abf',
                borderRadius: 3,
                transition: 'width 0.2s',
              }}
            />
          </div>
          <span>
            {progress.value}/{progress.max}
          </span>
        </>
      )}
    </div>
  );
};

export { ProgressBar };
