declare class AppStatusStore {
    activeFilePath: string | null;
    cursorPosition: {
        line: number;
        col: number;
    };
    gitBranch: string;
    justOpenedFiles: Set<string>;
    suppressSaveConfirmation: boolean;
    /** Recent editor file paths, most-recently-opened first (max 20). */
    recentFiles: string[];
    updateActiveFile(path: string | null): void;
    markAsJustOpened(path: string): void;
    consumeJustOpened(path: string): boolean;
    updateCursor(line: number, col: number): void;
    updateGitBranch(branch: string): void;
}
export declare const appStatus: AppStatusStore;
export {};
