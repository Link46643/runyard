// ─── Layout types ──────────────────────────────────────────────────────────────
export const DEFAULT_SETTINGS = {
    editor: {
        font_size: 14,
        tab_size: 2,
        line_wrap: false,
        format_on_save: false,
        vim_mode: false,
    },
    terminal: {
        default_shell: null,
        font_size: 13,
        scrollback_limit: 5000,
    },
    appearance: {
        theme: "dark",
        font_family: "JetBrains Mono",
    },
    lsp: {
        typescript: { enabled: true, path_override: null },
        python: { enabled: true, path_override: null },
        rust: { enabled: true, path_override: null },
        go: { enabled: true, path_override: null },
    },
};
