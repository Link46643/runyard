import type { LspServerStatus, LspStatusKind } from "@runyard/common";
type LspMessageHandler = (language: string, message: unknown) => void;
declare class LspStore {
    statuses: Record<string, LspServerStatus>;
    private messageHandlers;
    private unlistenFn;
    init(): Promise<void>;
    destroy(): void;
    onMessage(handler: LspMessageHandler): () => void;
    start(language: string, workspacePath: string, pathOverride?: string): Promise<LspServerStatus>;
    stop(language: string): Promise<void>;
    send(language: string, message: unknown): Promise<void>;
    getStatus(language: string): LspStatusKind;
    getActiveLangs(): string[];
}
export declare const lspStore: LspStore;
export {};
