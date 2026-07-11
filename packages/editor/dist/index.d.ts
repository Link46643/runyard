import { type Extension } from "@codemirror/state";
import { EditorView } from "@codemirror/view";
/** Returns the sticky scroll ViewPlugin extension. Only added when enabled. */
export declare function stickyScrollExtension(): Extension;
export interface EditorOptions {
    parent: HTMLElement;
    doc: string;
    /** File path - used for language auto-detection by extension when `language` isn't given. */
    filePath?: string;
    /** Explicit language id (e.g. "typescript", "python", "rust", "go"). Takes precedence over filePath. */
    language?: string;
    /** Read-only mode - disables editing while keeping syntax highlighting. Used for chat code blocks. */
    readOnly?: boolean;
    /** Optional LSP-related CodeMirror extensions (diagnostics, hover, completions, etc.) */
    lspExtensions?: Extension[];
    /** Font size in pixels (sets --editor-font-size CSS variable on the editor DOM element). */
    fontSize?: number;
    /** Number of spaces per tab stop. */
    tabSize?: number;
    /** Wrap long lines instead of scrolling horizontally. */
    lineWrap?: boolean;
    /** Enable Vim keybindings. Requires @replit/codemirror-vim (not currently installed — stub only). */
    vimMode?: boolean;
    /** Enable minimal Emacs keybindings (Ctrl-a/e/k/f/b/n/p/d). Uses @codemirror/commands. */
    emacsMode?: boolean;
    /** Show fold gutters and enable fold keyboard shortcuts. Default: true. */
    showFoldGutter?: boolean;
    /** Show a minimap sidebar. Handled by the MiniMap.svelte component, not a CM extension. */
    showMinimap?: boolean;
    /**
     * Enable sticky scroll — shows the enclosing scope header at the top of the
     * viewport while scrolling through a file.
     */
    stickyScroll?: boolean;
    /**
     * Inline diff decoration: line numbers for added/deleted lines.
     * Added lines get class `cm-diff-add`, deleted lines get `cm-diff-del`.
     */
    inlineDiff?: {
        additions: number[];
        deletions: number[];
    };
    onChange?: (content: string) => void;
    onSave?: (content: string) => void;
    onSelectionChange?: (line: number, col: number) => void;
}
/** Return basic document statistics for the breadcrumb strip. */
export declare function getDocumentStats(view: EditorView): {
    lineCount: number;
    charCount: number;
    cursorLine: number;
    cursorCol: number;
};
export declare function setupEditor(options: EditorOptions): {
    view: EditorView;
    getValue: () => string;
    setValue: (content: string) => void;
    destroy: () => void;
};
