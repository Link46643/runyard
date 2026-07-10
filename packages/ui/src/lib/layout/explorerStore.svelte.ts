class ExplorerStore {
  expandedPaths = $state<Set<string>>(new Set());

  constructor() {
    this.load();
  }

  load() {
    if (typeof window !== "undefined") {
      const saved = localStorage.getItem("runyard:explorer:expanded");
      if (saved) {
        try {
          const paths = JSON.parse(saved);
          if (Array.isArray(paths)) {
            this.expandedPaths = new Set(paths);
          }
        } catch (e) {
          console.error("Failed to parse explorer state", e);
        }
      }
    }
  }

  save() {
    if (typeof window !== "undefined") {
      localStorage.setItem("runyard:explorer:expanded", JSON.stringify(Array.from(this.expandedPaths)));
    }
  }

  toggle(path: string, expanded: boolean) {
    if (expanded) {
      this.expandedPaths.add(path);
    } else {
      this.expandedPaths.delete(path);
    }
    this.save();
  }

  isExpanded(path: string): boolean {
    return this.expandedPaths.has(path);
  }
}

export const explorerStore = new ExplorerStore();
