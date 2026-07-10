const MAX_RECENT_FILES = 20;

class AppStatusStore {
  activeFilePath = $state<string | null>(null);
  cursorPosition = $state<{ line: number; col: number }>({ line: 1, col: 1 });
  gitBranch = $state<string>("detached");
  // Tracks files that were just opened from the explorer
  justOpenedFiles = $state<Set<string>>(new Set());
  suppressSaveConfirmation = $state(false);
  /** Recent editor file paths, most-recently-opened first (max 20). */
  recentFiles = $state<string[]>([]);

  updateActiveFile(path: string | null) {
    this.activeFilePath = path;
    if (path) {
      // Prepend to recent files, deduplicated, max 20
      this.recentFiles = [
        path,
        ...this.recentFiles.filter((p) => p !== path),
      ].slice(0, MAX_RECENT_FILES);
    }
  }

  markAsJustOpened(path: string) {
    this.justOpenedFiles.add(path);
  }

  consumeJustOpened(path: string): boolean {
    if (this.justOpenedFiles.has(path)) {
      this.justOpenedFiles.delete(path);
      return true;
    }
    return false;
  }

  updateCursor(line: number, col: number) {
    this.cursorPosition = { line, col };
  }

  updateGitBranch(branch: string) {
    this.gitBranch = branch;
  }
}

export const appStatus = new AppStatusStore();
