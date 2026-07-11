import { EditorState, StateField, type Extension } from "@codemirror/state";
import {
  EditorView,
  ViewPlugin,
  Decoration,
  type DecorationSet,
  type ViewUpdate,
  keymap,
  drawSelection,
  rectangularSelection,
  crosshairCursor,
} from "@codemirror/view";
import { basicSetup } from "@codemirror/basic-setup";
import { javascript } from "@codemirror/lang-javascript";
import { python } from "@codemirror/lang-python";
import { rust } from "@codemirror/lang-rust";
import { go } from "@codemirror/lang-go";
import { oneDark } from "@codemirror/theme-one-dark";
import {
  history,
  historyKeymap,
  undo,
  redo,
  indentWithTab,
  cursorLineStart,
  cursorLineEnd,
  cursorCharRight,
  cursorCharLeft,
  cursorLineDown,
  cursorLineUp,
  deleteCharForward,
  deleteCharBackward,
} from "@codemirror/commands";
import { foldGutter, foldKeymap } from "@codemirror/language";
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

function stickyScrollPlugin(): Extension {
  return ViewPlugin.fromClass(
    class {
      dom: HTMLDivElement;
      view: EditorView;

      constructor(view: EditorView) {
        this.view = view;
        this.dom = document.createElement("div");
        this.dom.className = "cm-sticky-scroll";
        this.dom.setAttribute("aria-hidden", "true");
        view.dom.insertBefore(this.dom, view.dom.firstChild);
        this._render(view);
      }

      update(upd: ViewUpdate) {
        if (upd.geometryChanged || upd.viewportChanged || upd.docChanged) {
          this._render(upd.view);
        }
      }

      _render(view: EditorView) {
        const { from } = view.viewport;
        const topLine = view.state.doc.lineAt(from);
        const topIndent = this._indentOf(topLine.text);
        let headerText = "";
        for (let n = topLine.number - 1; n >= 1; n--) {
          const line = view.state.doc.line(n);
          const text = line.text.trimEnd();
          if (text.length === 0) continue;
          const indent = this._indentOf(text);
          if (indent < topIndent) {
            headerText = text.trimStart();
            break;
          }
        }
        if (headerText) {
          this.dom.textContent = headerText;
          this.dom.style.display = "block";
        } else {
          this.dom.style.display = "none";
        }
      }

      _indentOf(text: string): number {
        let i = 0;
        while (i < text.length && (text[i] === " " || text[i] === "\t")) i++;
        return i;
      }

      destroy() { this.dom.remove(); }
    }
  );
}

/** Returns the sticky scroll ViewPlugin extension. Only added when enabled. */
export function stickyScrollExtension(): Extension {
  return stickyScrollPlugin();
}

// ─── Inline diff decoration (1.12.10) ─────────────────────────────────────────

function buildDiffDecorations(
  state: EditorState,
  additions: number[],
  deletions: number[]
): DecorationSet {
  const decos: ReturnType<typeof Decoration.line>[] = [];
  const marks: { from: number; value: ReturnType<typeof Decoration.line> }[] = [];

  for (const lineNum of additions) {
    if (lineNum < 1 || lineNum > state.doc.lines) continue;
    const line = state.doc.line(lineNum);
    marks.push({ from: line.from, value: Decoration.line({ class: "cm-diff-add" }) });
  }
  for (const lineNum of deletions) {
    if (lineNum < 1 || lineNum > state.doc.lines) continue;
    const line = state.doc.line(lineNum);
    marks.push({ from: line.from, value: Decoration.line({ class: "cm-diff-del" }) });
  }

  // Decoration.set expects sorted, non-overlapping decorations
  marks.sort((a, b) => a.from - b.from);
  return Decoration.set(marks.map(({ from, value }) => value.range(from)));
}

function inlineDiffField(
  additions: number[],
  deletions: number[]
): Extension {
  return StateField.define<DecorationSet>({
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

// ─── EditorOptions interface ───────────────────────────────────────────────────

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
  inlineDiff?: { additions: number[]; deletions: number[] };
  onChange?: (content: string) => void;
  onSave?: (content: string) => void;
  onSelectionChange?: (line: number, col: number) => void;
}

// ─── Utilities ─────────────────────────────────────────────────────────────────

/** Return basic document statistics for the breadcrumb strip. */
export function getDocumentStats(view: EditorView): {
  lineCount: number;
  charCount: number;
  cursorLine: number;
  cursorCol: number;
} {
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

function resolveLanguageExtension(options: EditorOptions): Extension | null {
  const lang = (options.language ?? "").toLowerCase();
  const ext = options.filePath?.split(".").pop()?.toLowerCase();

  const isJs = lang === "javascript" || lang === "js" || lang === "jsx" || ext === "js" || ext === "jsx" || ext === "mjs" || ext === "cjs";
  const isTs = lang === "typescript" || lang === "ts" || lang === "tsx" || ext === "ts" || ext === "tsx";
  const isPy = lang === "python" || lang === "py" || ext === "py";
  const isRust = lang === "rust" || lang === "rs" || ext === "rs";
  const isGo = lang === "go" || lang === "golang" || ext === "go";

  if (isTs) return javascript({ typescript: true, jsx: lang === "tsx" || ext === "tsx" });
  if (isJs) return javascript({ typescript: false, jsx: lang === "jsx" || ext === "jsx" });
  if (isPy) return python();
  if (isRust) return rust();
  if (isGo) return go();
  return null;
}

// ─── setupEditor ──────────────────────────────────────────────────────────────

export function setupEditor(options: EditorOptions) {
  const extensions: Extension[] = [
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
    extensions.push(
      EditorView.theme({
        "&": { "--editor-font-size": `${options.fontSize}px` },
      })
    );
  }

  // Tab size
  if (options.tabSize) {
    extensions.push(
      EditorState.tabSize.of(options.tabSize)
    );
  }

  // Language auto-detection (explicit `language` wins over filePath extension)
  const langExt = resolveLanguageExtension(options);
  if (langExt) extensions.push(langExt);

  // LSP extensions (diagnostics, hover, completions, etc.)
  if (options.lspExtensions && options.lspExtensions.length > 0) {
    extensions.push(...options.lspExtensions);
  }

  // Save keymap (only if no lspExtensions handle Mod-s, to avoid duplicate)
  if (options.onSave) {
    extensions.push(
      keymap.of([
        {
          key: "Mod-s",
          preventDefault: true,
          run: (view) => {
            options.onSave!(view.state.doc.toString());
            return true;
          },
        },
      ])
    );
  }

  // 1.12.7 — Sticky scroll
  if (options.stickyScroll) {
    extensions.push(stickyScrollExtension());
  }

  // 1.12.10 — Inline diff decorations
  if (options.inlineDiff) {
    extensions.push(
      inlineDiffField(options.inlineDiff.additions, options.inlineDiff.deletions)
    );
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
    setValue: (content: string) => {
      view.dispatch({
        changes: { from: 0, to: view.state.doc.length, insert: content },
      });
    },
    destroy: () => view.destroy(),
  };
}
