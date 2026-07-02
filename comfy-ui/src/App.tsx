import { useState, type FC } from 'react';
import { ReactFlowProvider } from '@xyflow/react';
import { GraphEditor } from '@/components/editor/GraphEditor';
import { Toolbar } from '@/components/editor/Toolbar';
import { ProgressBar } from '@/components/editor/ProgressBar';
import { NodePanel } from '@/components/sidebar/NodePanel';
import { PropertyPanel } from '@/components/sidebar/PropertyPanel';
import ImageGallery from '@/components/sidebar/ImageGallery';
import AssetManager from '@/components/sidebar/AssetManager';
import Gaussian3DPanel from '@/components/sidebar/Gaussian3DPanel';
import WorkflowManager from '@/components/sidebar/WorkflowManager';
import { ModelManager } from '@/components/sidebar/ModelManager';
import { LlmSettings } from '@/components/sidebar/LlmSettings';
import { CustomNodePanel } from '@/components/custom/CustomNodePanel';
import { AIAgent } from '@/components/agent/AIAgent';
import { useWebSocket } from '@/hooks/useWebSocket';
import { useInitApp } from '@/hooks/useInitApp';
import { useWorkflowStore } from '@/store/workflow';
import { getPluginManager } from '@/plugins/manager';
import promptRelayPlugin from '@/plugins/promptRelayPlugin';

type SidebarTab = 'nodes' | 'properties' | 'assets' | '3d' | 'workflows' | 'models' | 'custom' | 'llm';

export interface PanelVisibility {
  showSidebar: boolean;
  showAgent: boolean;
}

const AppInner: FC = () => {
  useWebSocket();
  useInitApp();

  // Register plugins
  const pluginManager = getPluginManager();
  if (!pluginManager.getPlugin('PromptRelay')) {
    pluginManager.registerPlugin(promptRelayPlugin);
  }

  const [activeTab, setActiveTab] = useState<SidebarTab>('nodes');
  const [showSidebar, setShowSidebar] = useState(true);
  const [showAgent, setShowAgent] = useState(true);
  const [gaussian3DFile, setGaussian3DFile] = useState<string>('');
  const [gaussian3DFormat, setGaussian3DFormat] = useState<'ply' | 'splat'>('ply');
  const loadWorkflowFromJson = useWorkflowStore((s) => s.loadWorkflowFromJson);
  const getWorkflowAsJson = useWorkflowStore((s) => s.getWorkflowAsJson);

  const handleLoadWorkflow = (workflow: unknown) => {
    if (workflow && typeof workflow === 'object') {
      loadWorkflowFromJson(workflow as Record<string, unknown>);
    }
  };

  const handleGetCurrentWorkflow = () => {
    return getWorkflowAsJson();
  };

  return (
    <div
      style={{
        width: '100vw',
        height: '100vh',
        display: 'flex',
        flexDirection: 'column',
        background: '#0f1117',
        color: '#e2e8f0',
      }}
    >
      <Toolbar
        showSidebar={showSidebar}
        showAgent={showAgent}
        onToggleSidebar={() => setShowSidebar((v) => !v)}
        onToggleAgent={() => setShowAgent((v) => !v)}
      />
      <div style={{ flex: 1, display: 'flex', overflow: 'hidden', position: 'relative' }}>
        {showSidebar && (
          <div style={{ width: '260px', display: 'flex', flexDirection: 'column', borderRight: '1px solid #2d3748', background: '#1a202c' }}>
            <div style={{ display: 'flex', flexDirection: 'column', borderBottom: '1px solid #2d3748' }}>
              {([
                ['nodes', 'properties', 'assets', '3d'],
                ['workflows', 'models', 'custom', 'llm'],
              ] as SidebarTab[][]).map((row, rowIdx) => (
                <div key={rowIdx} style={{ display: 'flex' }}>
                  {row.map((tab) => (
                    <button
                      key={tab}
                      onClick={() => setActiveTab(tab)}
                      style={{
                        flex: 1,
                        padding: '6px 0',
                        fontSize: '11px',
                        background: activeTab === tab ? '#2d3748' : 'transparent',
                        border: 'none',
                        borderBottom: activeTab === tab ? '2px solid #4a9eff' : '2px solid transparent',
                        color: activeTab === tab ? '#e2e8f0' : '#718096',
                        cursor: 'pointer',
                        textTransform: 'capitalize',
                      }}
                    >
                      {tab === 'custom' ? '⚙' : tab === 'models' ? '📦' : tab === 'llm' ? '🤖' : tab === '3d' ? '🎮' : tab === 'assets' ? '📁' : tab}
                    </button>
                  ))}
                </div>
              ))}
            </div>
            <div style={{ flex: 1, overflow: 'auto' }}>
              {activeTab === 'nodes' && <NodePanel />}
              {activeTab === 'properties' && <PropertyPanel />}
              {activeTab === 'assets' && <AssetManager />}
              {activeTab === '3d' && gaussian3DFile && (
                <Gaussian3DPanel
                  filePath={gaussian3DFile}
                  format={gaussian3DFormat}
                />
              )}
              {activeTab === '3d' && !gaussian3DFile && (
                <div
                  style={{
                    display: 'flex',
                    alignItems: 'center',
                    justifyContent: 'center',
                    height: '100%',
                    color: '#718096',
                    fontSize: '13px',
                    textAlign: 'center',
                    padding: '20px',
                  }}
                >
                  <div>
                    <div style={{ fontSize: '32px', marginBottom: '12px' }}>🎮</div>
                    <div>No 3D Gaussian data to display</div>
                    <div style={{ fontSize: '11px', marginTop: '8px', opacity: 0.7 }}>
                      Run a TripoSplat node to generate 3D output
                    </div>
                  </div>
                </div>
              )}
              {activeTab === 'workflows' && (
                <WorkflowManager
                  onLoadWorkflow={handleLoadWorkflow}
                  getCurrentWorkflow={handleGetCurrentWorkflow}
                />
              )}
              {activeTab === 'models' && <ModelManager />}
              {activeTab === 'custom' && <CustomNodePanel />}
              {activeTab === 'llm' && <LlmSettings />}
            </div>
          </div>
        )}

        <div style={{ flex: 1, position: 'relative' }}>
          <GraphEditor />
          <ProgressBar />
        </div>

        {showAgent && <AIAgent />}
      </div>
    </div>
  );
};

const App: FC = () => (
  <ReactFlowProvider>
    <AppInner />
  </ReactFlowProvider>
);

export default App;
