/**
 * LSP Client for TCP/WebSocket connection
 *
 * This module provides a WebSocket-based LSP client that connects to the
 * Rust LSP server running on localhost:4137.
 */

export interface CompletionItem {
  label: string;
  kind?: number;
  detail?: string;
  documentation?: string;
  insertText?: string;
  sortText?: string;
  filterText?: string;
}

export interface Hover {
  contents: {
    kind: string;
    value: string;
  };
  range?: {
    start: { line: number; character: number };
    end: { line: number; character: number };
  };
}

export interface Diagnostic {
  severity: number;
  message: string;
  range: {
    start: { line: number; character: number };
    end: { line: number; character: number };
  };
  source?: string;
  code?: string | number;
}

export interface LspClientOptions {
  url?: string;
  reconnectAttempts?: number;
  reconnectDelay?: number;
}

export class LspClient {
  private ws: WebSocket | null = null;
  private requestId = 0;
  private pendingRequests = new Map<number, {
    resolve: (value: any) => void;
    reject: (error: Error) => void;
  }>();
  private options: Required<LspClientOptions>;
  private connected = false;
  private reconnectTimer: ReturnType<typeof setTimeout> | null = null;

  constructor(options: LspClientOptions = {}) {
    this.options = {
      url: options.url || 'ws://localhost:4137',
      reconnectAttempts: options.reconnectAttempts || 3,
      reconnectDelay: options.reconnectDelay || 1000,
    };
  }

  /**
   * Connect to the LSP server
   */
  async connect(): Promise<void> {
    if (this.connected) {
      console.warn('[LSP Client] Already connected');
      return;
    }

    return new Promise((resolve, reject) => {
      try {
        this.ws = new WebSocket(this.options.url);

        this.ws.onopen = () => {
          console.log('[LSP Client] Connected to', this.options.url);
          this.connected = true;
          resolve();
        };

        this.ws.onerror = (error) => {
          console.error('[LSP Client] WebSocket error:', error);
          reject(new Error('WebSocket connection failed'));
        };

        this.ws.onclose = () => {
          console.log('[LSP Client] Connection closed');
          this.connected = false;
          this.cleanup();
        };

        this.ws.onmessage = (event) => this.handleMessage(event.data);
      } catch (error) {
        reject(error);
      }
    });
  }

  /**
   * Check if client is connected
   */
  isConnected(): boolean {
    return this.connected && (this.ws?.readyState === WebSocket.OPEN);
  }

  /**
   * Disconnect from the server
   */
  disconnect(): void {
    if (this.reconnectTimer) {
      clearTimeout(this.reconnectTimer);
      this.reconnectTimer = null;
    }

    if (this.ws) {
      this.ws.close();
      this.ws = null;
    }

    this.connected = false;
    this.cleanup();
  }

  /**
   * Request completion items
   */
  async completion(
    _text: string,
    line: number,
    character: number
  ): Promise<CompletionItem[]> {
    const result = await this.sendRequest('textDocument/completion', {
      textDocument: { uri: 'file:///playground.sql' },
      position: { line, character },
    });

    // Handle different response formats
    if (Array.isArray(result)) {
      return result;
    } else if (result && typeof result === 'object' && 'items' in result) {
      return (result as any).items || [];
    }

    return [];
  }

  /**
   * Request hover information
   */
  async hover(
    _text: string,
    line: number,
    character: number
  ): Promise<Hover | null> {
    return this.sendRequest('textDocument/hover', {
      textDocument: { uri: 'file:///playground.sql' },
      position: { line, character },
    });
  }

  /**
   * Request diagnostics
   */
  async diagnostics(_text: string): Promise<Diagnostic[]> {
    const result = await this.sendRequest('textDocument/diagnostic', {
      textDocument: { uri: 'file:///playground.sql' },
      content: _text,
    });

    if (Array.isArray(result)) {
      return result;
    }

    return [];
  }

  /**
   * Send a JSON-RPC request
   */
  private sendRequest<T>(method: string, params: any): Promise<T> {
    return new Promise((resolve, reject) => {
      if (!this.isConnected()) {
        reject(new Error('Not connected to LSP server'));
        return;
      }

      const id = ++this.requestId;
      this.pendingRequests.set(id, { resolve, reject });

      const message = JSON.stringify({
        jsonrpc: '2.0',
        id,
        method,
        params,
      });

      this.ws!.send(message);

      // Set timeout for request
      setTimeout(() => {
        if (this.pendingRequests.has(id)) {
          this.pendingRequests.delete(id);
          reject(new Error(`Request timeout: ${method}`));
        }
      }, 5000);
    });
  }

  /**
   * Handle incoming WebSocket message
   */
  private handleMessage(data: string) {
    try {
      const response = JSON.parse(data);

      if (response.id) {
        const pending = this.pendingRequests.get(response.id);
        if (pending) {
          if (response.error) {
            pending.reject(
              new Error(response.error.message || 'LSP request failed')
            );
          } else {
            pending.resolve(response.result);
          }
          this.pendingRequests.delete(response.id);
        }
      }
    } catch (error) {
      console.error('[LSP Client] Failed to parse message:', error);
    }
  }

  /**
   * Clean up pending requests
   */
  private cleanup() {
    for (const [_id, pending] of this.pendingRequests) {
      pending.reject(new Error('Connection closed'));
    }
    this.pendingRequests.clear();
  }
}
