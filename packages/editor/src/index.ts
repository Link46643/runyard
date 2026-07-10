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
  filePath: string;
  /** Optional LSP-related CodeMirror extensions (diagnostics, hover, completions, etc.) */
  lspExtensions?: Extension[];
  onChange?: (content: string) => void;
  onSave?: (content: string) => void;
  onSelectionChange?: (line: number, col: number) => void;
}

export function setupEditor(options: EditorOptions) {
  const ext = options.filePath.split(".").pop()?.toLowerCase();

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

  // Language auto-detection
  if (ext === "js" || ext === "jsx" || ext === "mjs" || ext === "cjs") {
    extensions.push(javascript({ typescript: false, jsx: ext === "jsx" }));
  } else if (ext === "ts" || ext === "tsx") {
    extensions.push(javascript({ typescript: true, jsx: ext === "tsx" }));
  } else if (ext === "py") {
    extensions.push(python());
  } else if (ext === "rs") {
    extensions.push(rust());
  } else if (ext === "go") {
    extensions.push(go());
  }

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
