const MAX_RECENT_FILES = 20;
class AppStatusStore {
    activeFilePath = $state(null);
    cursorPosition = $state({ line: 1, col: 1 });
    gitBranch = $state("detached");
    // Tracks files that were just opened from the explorer
    justOpenedFiles = $state(new Set());
    suppressSaveConfirmation = $state(false);
    /** Recent editor file paths, most-recently-opened first (max 20). */
    recentFiles = $state([]);
    updateActiveFile(path) {
        this.activeFilePath = path;
        if (path) {
            // Prepend to recent files, deduplicated, max 20
            this.recentFiles = [
                path,
                ...this.recentFiles.filter((p) => p !== path),
            ].slice(0, MAX_RECENT_FILES);
        }
    }
    markAsJustOpened(path) {
        this.justOpenedFiles.add(path);
    }
    consumeJustOpened(path) {
        if (this.justOpenedFiles.has(path)) {
            this.justOpenedFiles.delete(path);
            return true;
        }
        return false;
    }
    updateCursor(line, col) {
        this.cursorPosition = { line, col };
    }
    updateGitBranch(branch) {
        this.gitBranch = branch;
    }
}
export const appStatus = new AppStatusStore();
