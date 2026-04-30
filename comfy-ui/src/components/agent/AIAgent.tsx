import { useState, useRef, useEffect, type FC } from 'react';
import { Send, Bot, User } from 'lucide-react';
import { useWorkflowStore } from '@/store/workflow';
import { api } from '@/api/client';

interface ChatMessage {
  id: string;
  role: 'user' | 'assistant';
  content: string;
  timestamp: number;
}

interface AgentAction {
  type: 'add_node' | 'connect' | 'set_param' | 'run_workflow';
  payload: unknown;
}

const AIAgent: FC = () => {
  const [messages, setMessages] = useState<ChatMessage[]>([]);
  const [input, setInput] = useState('');
  const [isProcessing, setIsProcessing] = useState(false);
  const messagesEndRef = useRef<HTMLDivElement>(null);
  const addNode = useWorkflowStore((s) => s.addNode);
  const getPrompt = useWorkflowStore((s) => s.getPrompt);
  const clientId = useWorkflowStore((s) => s.clientId);

  useEffect(() => {
    messagesEndRef.current?.scrollIntoView({ behavior: 'smooth' });
  }, [messages]);

  const addMessage = (role: 'user' | 'assistant', content: string) => {
    setMessages((prev) => [
      ...prev,
      { id: crypto.randomUUID(), role, content, timestamp: Date.now() },
    ]);
  };

  const parseActions = (text: string): AgentAction[] => {
    const actions: AgentAction[] = [];
    const actionRegex = /```action\n([\s\S]*?)```/g;
    let match;
    while ((match = actionRegex.exec(text)) !== null) {
      try {
        const action = JSON.parse(match[1]);
        actions.push(action);
      } catch {
        // skip invalid actions
      }
    }
    return actions;
  };

  const executeAction = (action: AgentAction) => {
    switch (action.type) {
      case 'add_node': {
        const payload = action.payload as { classType: string; x?: number; y?: number };
        addNode(payload.classType, {
          x: payload.x ?? 300 + Math.random() * 200,
          y: payload.y ?? 200 + Math.random() * 200,
        });
        break;
      }
      case 'run_workflow': {
        const prompt = getPrompt();
        api.submitPrompt({
          prompt: prompt as Record<string, import('@/types/api').NodeDefinition>,
          client_id: clientId,
        });
        break;
      }
      default:
        console.warn('Unknown agent action:', action.type);
    }
  };

  const handleSend = async () => {
    const text = input.trim();
    if (!text || isProcessing) return;

    setInput('');
    addMessage('user', text);
    setIsProcessing(true);

    try {
      const response = await processUserMessage(text);
      addMessage('assistant', response);

      const actions = parseActions(response);
      for (const action of actions) {
        executeAction(action);
      }
    } catch (err) {
      addMessage('assistant', `Error: ${err instanceof Error ? err.message : 'Unknown error'}`);
    } finally {
      setIsProcessing(false);
    }
  };

  const handleKeyDown = (e: React.KeyboardEvent) => {
    if (e.key === 'Enter' && !e.shiftKey) {
      e.preventDefault();
      handleSend();
    }
  };

  return (
    <div
      style={{
        width: 320,
        background: '#1a202c',
        borderLeft: '1px solid #2d3748',
        display: 'flex',
        flexDirection: 'column',
        height: '100%',
        color: '#e2e8f0',
      }}
    >
      <div
        style={{
          padding: '8px 12px',
          borderBottom: '1px solid #2d3748',
          fontWeight: 600,
          fontSize: 13,
          display: 'flex',
          alignItems: 'center',
          gap: 6,
        }}
      >
        <Bot size={16} style={{ color: '#5a6abf' }} />
        AI Agent
      </div>

      <div style={{ flex: 1, overflowY: 'auto', padding: 8 }}>
        {messages.length === 0 && (
          <div style={{ padding: 20, textAlign: 'center', color: '#4a5568', fontSize: 12 }}>
            Ask me to build a workflow, add nodes, or generate images.
            <br />
            <br />
            Examples:
            <br />
            "Create a txt2img workflow"
            <br />
            "Add a KSampler node"
            <br />
            "Run the current workflow"
          </div>
        )}

        {messages.map((msg) => (
          <div
            key={msg.id}
            style={{
              display: 'flex',
              gap: 8,
              marginBottom: 8,
              alignItems: 'flex-start',
            }}
          >
            <div
              style={{
                width: 24,
                height: 24,
                borderRadius: 12,
                background: msg.role === 'user' ? '#4a5568' : '#5a6abf',
                display: 'flex',
                alignItems: 'center',
                justifyContent: 'center',
                flexShrink: 0,
              }}
            >
              {msg.role === 'user' ? (
                <User size={12} />
              ) : (
                <Bot size={12} />
              )}
            </div>
            <div
              style={{
                background: '#2d3748',
                borderRadius: 8,
                padding: '6px 10px',
                fontSize: 12,
                lineHeight: 1.5,
                maxWidth: '100%',
                whiteSpace: 'pre-wrap',
              }}
            >
              {msg.content}
            </div>
          </div>
        ))}

        {isProcessing && (
          <div style={{ display: 'flex', gap: 8, alignItems: 'center', color: '#718096', fontSize: 12 }}>
            <Bot size={16} />
            Thinking...
          </div>
        )}

        <div ref={messagesEndRef} />
      </div>

      <div style={{ padding: 8, borderTop: '1px solid #2d3748' }}>
        <div
          style={{
            display: 'flex',
            gap: 6,
            alignItems: 'flex-end',
          }}
        >
          <textarea
            value={input}
            onChange={(e) => setInput(e.target.value)}
            onKeyDown={handleKeyDown}
            placeholder="Ask AI Agent..."
            rows={2}
            style={{
              flex: 1,
              background: '#2d3748',
              border: '1px solid #4a5568',
              borderRadius: 6,
              color: '#e2e8f0',
              padding: '6px 8px',
              fontSize: 12,
              resize: 'none',
              outline: 'none',
            }}
          />
          <button
            onClick={handleSend}
            disabled={isProcessing || !input.trim()}
            style={{
              background: '#5a6abf',
              border: 'none',
              borderRadius: 6,
              color: '#e2e8f0',
              padding: '6px 10px',
              cursor: isProcessing ? 'not-allowed' : 'pointer',
              display: 'flex',
              alignItems: 'center',
            }}
          >
            <Send size={14} />
          </button>
        </div>
      </div>
    </div>
  );
};

async function processUserMessage(text: string): Promise<string> {
  const lower = text.toLowerCase();

  if (lower.includes('txt2img') || lower.includes('text to image') || lower.includes('text-to-image')) {
    return `I'll create a text-to-image workflow for you! Here are the nodes I'm adding:

1. **CheckpointLoaderSimple** - Load the SD model
2. **CLIPTextEncode** (positive) - Encode the positive prompt
3. **CLIPTextEncode** (negative) - Encode the negative prompt
4. **EmptyLatentImage** - Create empty latent
5. **KSampler** - Sample the latent
6. **VAEDecode** - Decode latent to image
7. **SaveImage** - Save the output

\`\`\`action
{"type": "add_node", "payload": {"classType": "CheckpointLoaderSimple", "x": 50, "y": 100}}
\`\`\`
\`\`\`action
{"type": "add_node", "payload": {"classType": "CLIPTextEncode", "x": 300, "y": 50}}
\`\`\`
\`\`\`action
{"type": "add_node", "payload": {"classType": "CLIPTextEncode", "x": 300, "y": 200}}
\`\`\`
\`\`\`action
{"type": "add_node", "payload": {"classType": "EmptyLatentImage", "x": 300, "y": 350}}
\`\`\`
\`\`\`action
{"type": "add_node", "payload": {"classType": "KSampler", "x": 550, "y": 150}}
\`\`\`
\`\`\`action
{"type": "add_node", "payload": {"classType": "VAEDecode", "x": 800, "y": 150}}
\`\`\`
\`\`\`action
{"type": "add_node", "payload": {"classType": "SaveImage", "x": 1050, "y": 150}}
\`\`\`

Connect the nodes and adjust the parameters as needed!`;
  }

  if (lower.includes('add') && lower.includes('ksampler')) {
    return `Adding a KSampler node for you.

\`\`\`action
{"type": "add_node", "payload": {"classType": "KSampler", "x": 400, "y": 200}}
\`\`\`

The KSampler node handles the denoising process. Configure the seed, steps, CFG, and sampler method to control generation quality.`;
  }

  if (lower.includes('run') || lower.includes('execute') || lower.includes('generate')) {
    return `Running the current workflow!

\`\`\`action
{"type": "run_workflow", "payload": {}}
\`\`\`

The workflow has been submitted to the queue. You can monitor progress in the status bar below.`;
  }

  if (lower.includes('help') || lower.includes('what can you do')) {
    return `I can help you with:

- **Build workflows**: "Create a txt2img workflow"
- **Add nodes**: "Add a KSampler node"
- **Run workflows**: "Run the current workflow"
- **Explain nodes**: "What does KSampler do?"

I can also connect nodes and set parameters. Just describe what you want to achieve!`;
  }

  return `I understand you want: "${text}"

Currently, I support basic workflow building commands. For more complex operations, you can:
1. Use the node panel on the left to drag and drop nodes
2. Connect nodes by dragging from output to input handles
3. Configure parameters in the property panel on the right

What would you like to do next?`;
}

export { AIAgent };
