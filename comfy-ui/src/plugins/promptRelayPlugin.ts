import type { ComfyPlugin, NodeRegistry } from './manager';

/**
 * PromptRelay plugin - registers the PromptRelayEncodeTimeline node
 * with timeline editor support.
 */
const promptRelayPlugin: ComfyPlugin = {
  name: 'PromptRelay',
  version: '1.0.0',
  description: 'Prompt Relay nodes for temporal prompt control in video generation',

  registerNodes(_registry: NodeRegistry) {
    // The node definitions are already registered on the backend side.
    // This plugin exists to provide frontend-specific enhancements
    // like the timeline editor for PromptRelayEncodeTimeline.
  },

  onInit() {
    console.log('[PromptRelay] Plugin initialized');
  },
};

export default promptRelayPlugin;
