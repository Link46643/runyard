import { Extension } from "@codemirror/state";
export interface LspInterface {
    sendRequest(method: string, params: unknown): Promise<unknown>;
    sendNotification(method: string, params: unknown): void;
    onNotification(method: string, handler: (params: unknown) => void): () => void;
    isReady(): boolean;
}
export interface LspExtensionOptions {
    lsp: LspInterface;
    fileUri: string;
    languageId: string;
    filePath: string;
    onGoToDefinition?: (path: string, line: number, col: number) => void;
    formatOnSave?: boolean;
}
export declare function createLspExtension(options: LspExtensionOptions): Extension[];
/** Wire up incoming LSP messages from Rust to the pending-request/notification system.
 *  Must be called once on app startup (e.g. in StatusBar onMount).
 *  Safe to call multiple times — subsequent calls are no-ops. */
export declare function initLspClient(lspStore: {
    onMessage: (handler: (language: string, message: unknown) => void) => () => void;
}): void;
export declare function createLspInterface(language: string, lspStore: {
    send: (language: string, message: unknown) => void;
    getStatus: (language: string) => string;
    onMessage: (handler: (language: string, message: unknown) => void) => () => void;
}): LspInterface;
export declare function pathToUri(filePath: string): string;
export declare function detectLanguageId(filePath: string): string | null;
