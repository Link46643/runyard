type $$ComponentProps = {
    terminalId: string;
    cwd?: string;
    onExit?: () => void;
};
declare const TerminalPanel: import("svelte").Component<$$ComponentProps, {
    destroyTerminal: () => void;
}, "">;
type TerminalPanel = ReturnType<typeof TerminalPanel>;
export default TerminalPanel;
