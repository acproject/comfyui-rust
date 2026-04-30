import type { WsMessage } from '@/types/api';

type WsMessageHandler = (message: WsMessage) => void;

export class WsClient {
  private ws: WebSocket | null = null;
  private clientId: string;
  private handlers: Map<string, Set<WsMessageHandler>> = new Map();
  private reconnectTimer: ReturnType<typeof setTimeout> | null = null;
  private url: string;
  private _connected = false;

  constructor(clientId?: string) {
    this.clientId = clientId || crypto.randomUUID();
    const protocol = window.location.protocol === 'https:' ? 'wss:' : 'ws:';
    this.url = `${protocol}//${window.location.host}/ws?clientId=${this.clientId}`;
  }

  get connected(): boolean {
    return this._connected;
  }

  connect(): void {
    if (this.ws?.readyState === WebSocket.OPEN) return;

    this.ws = new WebSocket(this.url);

    this.ws.onopen = () => {
      this._connected = true;
      this.emit({ type: 'connected', data: { clientId: this.clientId } });
    };

    this.ws.onmessage = (event) => {
      try {
        const message: WsMessage = JSON.parse(event.data);
        this.emit(message);
      } catch {
        console.warn('Failed to parse WebSocket message');
      }
    };

    this.ws.onclose = () => {
      this._connected = false;
      this.emit({ type: 'disconnected', data: {} });
      this.scheduleReconnect();
    };

    this.ws.onerror = () => {
      this._connected = false;
    };
  }

  disconnect(): void {
    if (this.reconnectTimer) {
      clearTimeout(this.reconnectTimer);
      this.reconnectTimer = null;
    }
    this.ws?.close();
    this.ws = null;
    this._connected = false;
  }

  on(type: string, handler: WsMessageHandler): () => void {
    if (!this.handlers.has(type)) {
      this.handlers.set(type, new Set());
    }
    this.handlers.get(type)!.add(handler);
    return () => this.handlers.get(type)?.delete(handler);
  }

  off(type: string, handler: WsMessageHandler): void {
    this.handlers.get(type)?.delete(handler);
  }

  private emit(message: WsMessage): void {
    const handlers = this.handlers.get(message.type);
    if (handlers) {
      handlers.forEach((handler) => handler(message));
    }
    const allHandlers = this.handlers.get('*');
    if (allHandlers) {
      allHandlers.forEach((handler) => handler(message));
    }
  }

  private scheduleReconnect(): void {
    if (this.reconnectTimer) return;
    this.reconnectTimer = setTimeout(() => {
      this.reconnectTimer = null;
      this.connect();
    }, 2000);
  }
}

let _instance: WsClient | null = null;

export function getWsClient(): WsClient {
  if (!_instance) {
    _instance = new WsClient();
  }
  return _instance;
}
