type ResolveReject = { resolve: (val: any) => void; reject: (err: any) => void };

class WebSocketClient {
  private ws: WebSocket | null = null;
  private requestId = 0;
  private pendingRequests = new Map<number, ResolveReject>();
  private eventHandlers = new Map<string, Set<(event: { payload: any; event: string }) => void>>();
  private queuedRequests: string[] = [];
  private isConnecting = false;

  constructor() {
    if (typeof window !== "undefined") {
      this.connect();
    }
  }

  private connect() {
    if (this.ws || this.isConnecting) return;
    this.isConnecting = true;

    const protocol = window.location.protocol === "https:" ? "wss:" : "ws:";
    const host = window.location.host; // e.g. localhost:7820

    let token = localStorage.getItem("runyard:token") || "";
    const urlToken = new URLSearchParams(window.location.search).get("token");
    if (urlToken) {
      token = urlToken;
      localStorage.setItem("runyard:token", token);
    }

    const wsUrl = `${protocol}//${host}/ws?token=${encodeURIComponent(token)}`;
    console.log(`[WebSocketClient] Connecting to ${wsUrl}`);
    
    const socket = new WebSocket(wsUrl);
    this.ws = socket;

    socket.onopen = () => {
      console.log("[WebSocketClient] Connection open");
      this.isConnecting = false;
      // Send queued messages
      for (const msg of this.queuedRequests) {
        socket.send(msg);
      }
      this.queuedRequests = [];
    };

    socket.onmessage = (event) => {
      try {
        const msg = JSON.parse(event.data);
        if (msg.id !== undefined) {
          console.log(`[WebSocketClient] Received response for request id: ${msg.id}`, msg);
          const handler = this.pendingRequests.get(msg.id);
          if (handler) {
            this.pendingRequests.delete(msg.id);
            if (msg.error) {
              handler.reject(msg.error);
            } else {
              handler.resolve(msg.result);
            }
          }
        } else if (msg.method) {
          console.log(`[WebSocketClient] Received event: ${msg.method}`, msg.params);
          const handlers = this.eventHandlers.get(msg.method);
          if (handlers) {
            const ev = { payload: msg.params, event: msg.method };
            for (const h of handlers) {
              h(ev);
            }
          }
        }
      } catch (e) {
        console.error("[WebSocketClient] Failed to parse message:", e);
      }
    };

    socket.onclose = () => {
      console.log("[WebSocketClient] Connection closed. Retrying in 2 seconds...");
      this.ws = null;
      this.isConnecting = false;
      setTimeout(() => this.connect(), 2000);
    };

    socket.onerror = (err) => {
      console.error("[WebSocketClient] Socket error:", err);
      socket.close();
    };
  }

  public invoke<T>(method: string, params?: any): Promise<T> {
    return new Promise((resolve, reject) => {
      const id = ++this.requestId;
      console.log(`[WebSocketClient] Invoking method: ${method} (id: ${id})`, params);
      const requestMsg = JSON.stringify({
        jsonrpc: "2.0",
        id,
        method,
        params: params || {}
      });

      this.pendingRequests.set(id, { resolve, reject });

      if (this.ws && this.ws.readyState === WebSocket.OPEN) {
        this.ws.send(requestMsg);
      } else {
        console.log(`[WebSocketClient] Queueing request (id: ${id}) - socket not ready`);
        this.queuedRequests.push(requestMsg);
      }
    });
  }

  public listen<T>(event: string, handler: (event: { payload: T; event: string }) => void): Promise<() => void> {
    if (!this.eventHandlers.has(event)) {
      this.eventHandlers.set(event, new Set());
    }
    this.eventHandlers.get(event)!.add(handler);

    const unlisten = () => {
      const handlers = this.eventHandlers.get(event);
      if (handlers) {
        handlers.delete(handler);
      }
    };

    return Promise.resolve(unlisten);
  }
}

export const webSocketClient = new WebSocketClient();
