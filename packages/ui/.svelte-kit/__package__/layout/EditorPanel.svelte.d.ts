type $$ComponentProps = {
    filePath: string;
    onDirtyChange: (dirty: boolean) => void;
};
declare const EditorPanel: import("svelte").Component<$$ComponentProps, {}, "">;
type EditorPanel = ReturnType<typeof EditorPanel>;
export default EditorPanel;
