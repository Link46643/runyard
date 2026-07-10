declare class ExplorerStore {
    expandedPaths: Set<string>;
    constructor();
    load(): void;
    save(): void;
    toggle(path: string, expanded: boolean): void;
    isExpanded(path: string): boolean;
}
export declare const explorerStore: ExplorerStore;
export {};
