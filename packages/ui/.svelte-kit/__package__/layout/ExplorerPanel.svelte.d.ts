type $$ComponentProps = {
    workspacePath: string;
    onOpenFile: (path: string, name: string) => void;
};
declare const ExplorerPanel: import("svelte").Component<$$ComponentProps, {}, "">;
type ExplorerPanel = ReturnType<typeof ExplorerPanel>;
export default ExplorerPanel;
