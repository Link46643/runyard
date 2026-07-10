import { EditorState, type Extension } from "@codemirror/state";
import { EditorView, keymap } from "@codemirror/view";
import { basicSetup } from "@codemirror/basic-setup";
import { javascript } from "@codemirror/lang-javascript";
import { python } from "@codemirror/lang-python";
import { rust } from "@codemirror/lang-rust";
import { go } from "@codemirror/lang-go";
import { oneDark } from "@codemirror/theme-one-dark";
import { history, historyKeymap, undo, redo } from "@codemirror/commands";
import { indentWithTab } from "@codemirror/commands";

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
  onChange?: (content: string) => void;
  onSave?: (content: string) => void;
  onSelectionChange?: (line: number, col: number) => void;
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

  if (options.readOnly) {
    extensions.push(EditorView.editable.of(false));
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
