declare class WebSocketClient {
    private ws;
    private requestId;
    private pendingRequests;
    private eventHandlers;
    private queuedRequests;
    private isConnecting;
    private statusListeners;
    status: "connected" | "connecting" | "disconnected" | "local";
    constructor();
    private setStatus;
    onStatusChange(callback: (status: "connected" | "connecting" | "disconnected" | "local") => void): () => void;
    private connect;
    invoke<T>(method: string, params?: any): Promise<T>;
    listen<T>(event: string, handler: (event: {
        payload: T;
        event: string;
    }) => void): Promise<() => void>;
}
export declare const webSocketClient: WebSocketClient;
export {};
