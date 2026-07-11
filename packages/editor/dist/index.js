import { EditorState, StateField } from "@codemirror/state";
import { EditorView, ViewPlugin, Decoration, keymap, drawSelection, rectangularSelection, crosshairCursor, } from "@codemirror/view";
import { basicSetup } from "@codemirror/basic-setup";
import { javascript } from "@codemirror/lang-javascript";
import { python } from "@codemirror/lang-python";
import { rust } from "@codemirror/lang-rust";
import { go } from "@codemirror/lang-go";
import { html } from "@codemirror/lang-html";
import { css } from "@codemirror/lang-css";
import { json } from "@codemirror/lang-json";
import { markdown } from "@codemirror/lang-markdown";
import { cpp } from "@codemirror/lang-cpp";
import { java } from "@codemirror/lang-java";
import { sql } from "@codemirror/lang-sql";
import { yaml } from "@codemirror/lang-yaml";
import { php } from "@codemirror/lang-php";
import { xml } from "@codemirror/lang-xml";
import { wast } from "@codemirror/lang-wast";
import { shell } from "@codemirror/legacy-modes/mode/shell";
import { ruby } from "@codemirror/legacy-modes/mode/ruby";
import { csharp, kotlin, scala } from "@codemirror/legacy-modes/mode/clike";
import { swift } from "@codemirror/legacy-modes/mode/swift";
import { lua } from "@codemirror/legacy-modes/mode/lua";
import { sass } from "@codemirror/legacy-modes/mode/sass";
import { r } from "@codemirror/legacy-modes/mode/r";
import { toml } from "@codemirror/legacy-modes/mode/toml";
import { perl } from "@codemirror/legacy-modes/mode/perl";
import { dockerFile } from "@codemirror/legacy-modes/mode/dockerfile";
import { oneDark } from "@codemirror/theme-one-dark";
import { history, historyKeymap, undo, redo, indentWithTab, cursorLineStart, cursorLineEnd, cursorCharRight, cursorCharLeft, cursorLineDown, cursorLineUp, deleteCharForward, } from "@codemirror/commands";
import { foldGutter, foldKeymap, StreamLanguage } from "@codemirror/language";
// @codemirror/search is not yet in the editor package's dependencies.
// searchKeymap and highlightSelectionMatches are already included via basicSetup.
// selectNextOccurrence (Cmd+D) is deferred until the package is approved/added.
// import { searchKeymap, highlightSelectionMatches, selectNextOccurrence } from "@codemirror/search";
// ─── Emacs keybindings (1.12.2) ───────────────────────────────────────────────
// @replit/codemirror-emacs and @uiw/codemirror-extensions-emacs are not
// installed. Implement a minimal Emacs keymap using @codemirror/commands which
// IS installed.
// TODO 1.12.2: if @replit/codemirror-emacs is added, replace this with:
//   import { emacs } from "@replit/codemirror-emacs";
//   if (options.emacsMode) extensions.push(emacs());
const minimalEmacsKeymap = keymap.of([
    { key: "Ctrl-a", run: cursorLineStart },
    { key: "Ctrl-e", run: cursorLineEnd },
    { key: "Ctrl-k", run: (view) => { /* kill to end of line */ const { from } = view.state.selection.main; const line = view.state.doc.lineAt(from); view.dispatch({ changes: { from, to: line.to } }); return true; } },
    { key: "Ctrl-f", run: cursorCharRight },
    { key: "Ctrl-b", run: cursorCharLeft },
    { key: "Ctrl-n", run: cursorLineDown },
    { key: "Ctrl-p", run: cursorLineUp },
    { key: "Ctrl-d", run: deleteCharForward },
]);
// ─── Sticky scroll extension (1.12.7) ─────────────────────────────────────────
// Finds the last line before the viewport top that has LESS indentation than
// the first visible line, which represents the enclosing scope header.
// Displayed as a 24px bar pinned at the top of the editor using a DOM widget
// injected into the editor's outer element.
function stickyScrollPlugin() {
    return ViewPlugin.fromClass(class {
        dom;
        view;
        constructor(view) {
            this.view = view;
            this.dom = document.createElement("div");
            this.dom.className = "cm-sticky-scroll";
            this.dom.setAttribute("aria-hidden", "true");
            view.dom.insertBefore(this.dom, view.dom.firstChild);
            this._render(view);
        }
        update(upd) {
            if (upd.geometryChanged || upd.viewportChanged || upd.docChanged) {
                this._render(upd.view);
            }
        }
        _render(view) {
            const { from } = view.viewport;
            const topLine = view.state.doc.lineAt(from);
            const topIndent = this._indentOf(topLine.text);
            let headerText = "";
            for (let n = topLine.number - 1; n >= 1; n--) {
                const line = view.state.doc.line(n);
                const text = line.text.trimEnd();
                if (text.length === 0)
                    continue;
                const indent = this._indentOf(text);
                if (indent < topIndent) {
                    headerText = text.trimStart();
                    break;
                }
            }
            if (headerText) {
                this.dom.textContent = headerText;
                this.dom.style.display = "block";
            }
            else {
                this.dom.style.display = "none";
            }
        }
        _indentOf(text) {
            let i = 0;
            while (i < text.length && (text[i] === " " || text[i] === "\t"))
                i++;
            return i;
        }
        destroy() { this.dom.remove(); }
    });
}
/** Returns the sticky scroll ViewPlugin extension. Only added when enabled. */
export function stickyScrollExtension() {
    return stickyScrollPlugin();
}
// ─── Inline diff decoration (1.12.10) ─────────────────────────────────────────
function buildDiffDecorations(state, additions, deletions) {
    const decos = [];
    const marks = [];
    for (const lineNum of additions) {
        if (lineNum < 1 || lineNum > state.doc.lines)
            continue;
        const line = state.doc.line(lineNum);
        marks.push({ from: line.from, value: Decoration.line({ class: "cm-diff-add" }) });
    }
    for (const lineNum of deletions) {
        if (lineNum < 1 || lineNum > state.doc.lines)
            continue;
        const line = state.doc.line(lineNum);
        marks.push({ from: line.from, value: Decoration.line({ class: "cm-diff-del" }) });
    }
    // Decoration.set expects sorted, non-overlapping decorations
    marks.sort((a, b) => a.from - b.from);
    return Decoration.set(marks.map(({ from, value }) => value.range(from)));
}
function inlineDiffField(additions, deletions) {
    return StateField.define({
        create(state) {
            return buildDiffDecorations(state, additions, deletions);
        },
        update(deco, tr) {
            if (tr.docChanged) {
                return buildDiffDecorations(tr.state, additions, deletions);
            }
            return deco;
        },
        provide(f) {
            return EditorView.decorations.from(f);
        },
    });
}
// ─── Utilities ─────────────────────────────────────────────────────────────────
/** Return basic document statistics for the breadcrumb strip. */
export function getDocumentStats(view) {
    const state = view.state;
    const pos = state.selection.main.head;
    const line = state.doc.lineAt(pos);
    return {
        lineCount: state.doc.lines,
        charCount: state.doc.length,
        cursorLine: line.number,
        cursorCol: pos - line.from + 1,
    };
}
function resolveLanguageExtension(options) {
    const lang = (options.language ?? "").toLowerCase();
    const ext = options.filePath?.split(".").pop()?.toLowerCase();
    // Filename without extension — used for dotfiles and extensionless files.
    const filename = options.filePath?.split(/[\\/]/).pop()?.toLowerCase();
    // ── TypeScript / JavaScript ──────────────────────────────────────────────
    const isTs = lang === "typescript" || lang === "ts" || lang === "tsx" || ext === "ts" || ext === "tsx";
    const isJs = lang === "javascript" || lang === "js" || lang === "jsx" || lang === "mjs" ||
        ext === "js" || ext === "jsx" || ext === "mjs" || ext === "cjs";
    if (isTs)
        return javascript({ typescript: true, jsx: lang === "tsx" || ext === "tsx" });
    if (isJs)
        return javascript({ typescript: false, jsx: lang === "jsx" || ext === "jsx" });
    // ── Python ───────────────────────────────────────────────────────────────
    if (lang === "python" || lang === "py" || ext === "py" || ext === "pyw")
        return python();
    // ── Rust ─────────────────────────────────────────────────────────────────
    if (lang === "rust" || lang === "rs" || ext === "rs")
        return rust();
    // ── Go ───────────────────────────────────────────────────────────────────
    if (lang === "go" || lang === "golang" || ext === "go")
        return go();
    // ── HTML ─────────────────────────────────────────────────────────────────
    if (lang === "html" || ext === "html" || ext === "htm")
        return html();
    // ── CSS / SCSS / LESS ────────────────────────────────────────────────────
    if (lang === "css" || ext === "css")
        return css();
    if (lang === "scss" || ext === "scss")
        return css();
    if (lang === "sass" || ext === "sass")
        return StreamLanguage.define(sass);
    if (lang === "less" || ext === "less")
        return StreamLanguage.define(sass);
    // ── JSON ─────────────────────────────────────────────────────────────────
    if (lang === "json" || lang === "jsonc" || ext === "json" || ext === "jsonc" || ext === "jsonl")
        return json();
    // ── Markdown ─────────────────────────────────────────────────────────────
    if (lang === "markdown" || lang === "md" || lang === "mdx" || ext === "md" || ext === "markdown" || ext === "mdx")
        return markdown();
    // ── C / C++ ──────────────────────────────────────────────────────────────
    if (lang === "c" || lang === "cpp" || lang === "c++" || lang === "cxx" || lang === "cc" ||
        ext === "c" || ext === "cpp" || ext === "cc" || ext === "cxx" ||
        ext === "h" || ext === "hpp" || ext === "hxx" || ext === "hh")
        return cpp();
    // ── Java ─────────────────────────────────────────────────────────────────
    if (lang === "java" || ext === "java")
        return java();
    // ── SQL ──────────────────────────────────────────────────────────────────
    if (lang === "sql" || ext === "sql")
        return sql();
    // ── YAML ─────────────────────────────────────────────────────────────────
    if (lang === "yaml" || lang === "yml" || ext === "yaml" || ext === "yml")
        return yaml();
    // ── PHP ──────────────────────────────────────────────────────────────────
    if (lang === "php" || ext === "php")
        return php();
    // ── XML / SVG / XAML ─────────────────────────────────────────────────────
    if (lang === "xml" || lang === "svg" || ext === "xml" || ext === "svg" || ext === "xaml" || ext === "xsl" || ext === "xslt")
        return xml();
    // ── WebAssembly Text ─────────────────────────────────────────────────────
    if (lang === "wasm" || lang === "wast" || lang === "wat" || ext === "wat" || ext === "wast")
        return wast();
    // ── TOML ─────────────────────────────────────────────────────────────────
    if (lang === "toml" || ext === "toml")
        return StreamLanguage.define(toml);
    // ── Shell / Bash / Zsh / Fish ────────────────────────────────────────────
    if (lang === "shell" || lang === "bash" || lang === "sh" || lang === "zsh" || lang === "fish" ||
        ext === "sh" || ext === "bash" || ext === "zsh" || ext === "fish" ||
        filename === ".bashrc" || filename === ".zshrc" || filename === ".profile" || filename === ".bash_profile") {
        return StreamLanguage.define(shell);
    }
    // ── Ruby ─────────────────────────────────────────────────────────────────
    if (lang === "ruby" || lang === "rb" || ext === "rb" || ext === "rake" || ext === "gemspec" ||
        filename === "gemfile" || filename === "rakefile") {
        return StreamLanguage.define(ruby);
    }
    // ── C# ───────────────────────────────────────────────────────────────────
    if (lang === "csharp" || lang === "c#" || lang === "cs" || ext === "cs")
        return StreamLanguage.define(csharp);
    // ── Kotlin ───────────────────────────────────────────────────────────────
    if (lang === "kotlin" || lang === "kt" || ext === "kt" || ext === "kts")
        return StreamLanguage.define(kotlin);
    // ── Scala ────────────────────────────────────────────────────────────────
    if (lang === "scala" || ext === "scala" || ext === "sc")
        return StreamLanguage.define(scala);
    // ── Swift ────────────────────────────────────────────────────────────────
    if (lang === "swift" || ext === "swift")
        return StreamLanguage.define(swift);
    // ── Lua ──────────────────────────────────────────────────────────────────
    if (lang === "lua" || ext === "lua")
        return StreamLanguage.define(lua);
    // ── R ────────────────────────────────────────────────────────────────────
    if (lang === "r" || ext === "r" || ext === "rmd")
        return StreamLanguage.define(r);
    // ── Perl ─────────────────────────────────────────────────────────────────
    if (lang === "perl" || lang === "pl" || ext === "pl" || ext === "pm")
        return StreamLanguage.define(perl);
    // ── Dockerfile ───────────────────────────────────────────────────────────
    if (lang === "dockerfile" || filename === "dockerfile")
        return StreamLanguage.define(dockerFile);
    return null;
}
// ─── setupEditor ──────────────────────────────────────────────────────────────
export function setupEditor(options) {
    const extensions = [
        basicSetup,
        oneDark,
        history(),
        // Multi-cursor: drawSelection for custom selection rendering, rectangularSelection
        // for Alt+drag block selection, crosshairCursor for the Alt-drag crosshair cursor.
        drawSelection(),
        rectangularSelection(),
        crosshairCursor(),
        // Search is included via basicSetup. highlightSelectionMatches and selectNextOccurrence
        // need @codemirror/search which is not yet installed - omitted until added.
        keymap.of([
            ...historyKeymap,
            { key: "Mod-z", run: undo, preventDefault: true },
            { key: "Mod-y", run: redo, preventDefault: true },
            { key: "Mod-Shift-z", run: redo, preventDefault: true },
            indentWithTab,
        ]),
        EditorView.updateListener.of((update) => {
            if (update.docChanged && options.onChange) {
                options.onChange(update.state.doc.toString());
            }
            if (update.selectionSet && options.onSelectionChange) {
                const pos = update.state.selection.main.head;
                const line = update.state.doc.lineAt(pos);
                options.onSelectionChange(line.number, pos - line.from + 1);
            }
        }),
    ];
    // Code folding — enabled by default, opt-out via showFoldGutter: false.
    const foldEnabled = options.showFoldGutter !== false;
    if (foldEnabled) {
        extensions.push(foldGutter());
        extensions.push(keymap.of(foldKeymap));
    }
    // Word wrap
    if (options.lineWrap) {
        extensions.push(EditorView.lineWrapping);
    }
    // Vim mode — @replit/codemirror-vim is not installed; stub with a console warning.
    // TODO 1.12.1: install @replit/codemirror-vim then replace with:
    //   import { vim } from "@replit/codemirror-vim";
    //   if (options.vimMode) extensions.push(vim());
    if (options.vimMode) {
        console.warn("[Editor] Vim mode requested but @replit/codemirror-vim is not installed.");
    }
    // 1.12.2 — Emacs keybindings
    // @replit/codemirror-emacs and @uiw/codemirror-extensions-emacs are NOT installed.
    // Using a minimal built-in implementation from @codemirror/commands instead.
    if (options.emacsMode) {
        extensions.push(minimalEmacsKeymap);
    }
    if (options.readOnly) {
        extensions.push(EditorView.editable.of(false));
    }
    // Font size — set a CSS custom property on the editor's DOM root so the
    // scroller font-size rule in EditorPanel.svelte picks it up.
    if (options.fontSize) {
        extensions.push(EditorView.theme({
            "&": { "--editor-font-size": `${options.fontSize}px` },
        }));
    }
    // Tab size
    if (options.tabSize) {
        extensions.push(EditorState.tabSize.of(options.tabSize));
    }
    // Language auto-detection (explicit `language` wins over filePath extension)
    const langExt = resolveLanguageExtension(options);
    if (langExt)
        extensions.push(langExt);
    // LSP extensions (diagnostics, hover, completions, etc.)
    if (options.lspExtensions && options.lspExtensions.length > 0) {
        extensions.push(...options.lspExtensions);
    }
    // Save keymap (only if no lspExtensions handle Mod-s, to avoid duplicate)
    if (options.onSave) {
        extensions.push(keymap.of([
            {
                key: "Mod-s",
                preventDefault: true,
                run: (view) => {
                    options.onSave(view.state.doc.toString());
                    return true;
                },
            },
        ]));
    }
    // 1.12.7 — Sticky scroll
    if (options.stickyScroll) {
        extensions.push(stickyScrollExtension());
    }
    // 1.12.10 — Inline diff decorations
    if (options.inlineDiff) {
        extensions.push(inlineDiffField(options.inlineDiff.additions, options.inlineDiff.deletions));
    }
    const state = EditorState.create({
        doc: options.doc,
        extensions,
    });
    const view = new EditorView({
        state,
        parent: options.parent,
    });
    return {
        view,
        getValue: () => view.state.doc.toString(),
        setValue: (content) => {
            view.dispatch({
                changes: { from: 0, to: view.state.doc.length, insert: content },
            });
        },
        destroy: () => view.destroy(),
    };
}
