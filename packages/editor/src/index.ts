import { EditorState, type Extension } from "@codemirror/state";
import {
  EditorView,
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
import { history, historyKeymap, undo, redo } from "@codemirror/commands";
import { indentWithTab } from "@codemirror/commands";
import { foldGutter, foldKeymap } from "@codemirror/language";
// @codemirror/search is not yet in the editor package's dependencies.
// searchKeymap and highlightSelectionMatches are already included via basicSetup.
// selectNextOccurrence (Cmd+D) is deferred until the package is approved/added.
// import { searchKeymap, highlightSelectionMatches, selectNextOccurrence } from "@codemirror/search";

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
  /** Show fold gutters and enable fold keyboard shortcuts. Default: true. */
  showFoldGutter?: boolean;
  /** Show a minimap sidebar. Handled by the MiniMap.svelte component, not a CM extension. */
  showMinimap?: boolean;
  onChange?: (content: string) => void;
  onSave?: (content: string) => void;
  onSelectionChange?: (line: number, col: number) => void;
}

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

  // TODO 1.12.7: sticky scroll requires @codemirror/language v7+ stickyTop feature.
  // Not available in @codemirror/language 6.x.

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
