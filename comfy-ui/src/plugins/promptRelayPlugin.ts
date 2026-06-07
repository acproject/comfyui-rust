import type { ComfyPlugin, NodeRegistry } from './manager';

/**
 * PromptRelay plugin - registers the PromptRelayEncodeTimeline node
 * with timeline editor support, plus all WhatDreamsCost nodes.
 */
const promptRelayPlugin: ComfyPlugin = {
  name: 'PromptRelay',
  version: '1.0.0',
  description: 'Prompt Relay and WhatDreamsCost nodes for temporal prompt control in video generation',

  registerNodes(_registry: NodeRegistry) {
    // The node definitions are already registered on the backend side.
    // This plugin exists to provide frontend-specific enhancements
    // like the timeline editor for PromptRelayEncodeTimeline and LTXDirector.
  },

  onInit() {
    console.log('[PromptRelay] Plugin initialized — WhatDreamsCost nodes available: LTXKeyframer, LTXSequencer, LTXDirector, LTXDirectorGuide, MultiImageLoader, SpeechLengthCalculator, LoadAudioUI, LoadVideoUI');
  },
};

export default promptRelayPlugin;
