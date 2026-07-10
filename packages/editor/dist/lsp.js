import { EditorView, hoverTooltip, ViewPlugin, } from "@codemirror/view";
import { linter, lintGutter, setDiagnostics, } from "@codemirror/lint";
import { autocompletion, } from "@codemirror/autocomplete";
import { StateEffect } from "@codemirror/state";
import { keymap } from "@codemirror/view";
// ─── Position conversion helpers ─────────────────────────────────────────────
function cmPosToLsp(state, pos) {
    const line = state.doc.lineAt(pos);
    return {
        line: line.number - 1,
        character: pos - line.from,
    };
}
function lspPosToOffset(state, pos) {
    try {
        const line = state.doc.line(pos.line + 1);
        return line.from + Math.min(pos.character, line.length);
    }
    catch {
        return 0;
    }
}
function lspRangeToOffsets(state, range) {
    return {
        from: lspPosToOffset(state, range.start),
        to: lspPosToOffset(state, range.end),
    };
}
// ─── Diagnostics ─────────────────────────────────────────────────────────────
function lspSeverityToCm(severity) {
    switch (severity) {
        case 1: return "error";
        case 2: return "warning";
        case 3: return "info";
        default: return "info";
    }
}
// ─── Completions ─────────────────────────────────────────────────────────────
function lspCompletionKindToType(kind) {
    // LSP CompletionItemKind -> CodeMirror completion type
    const map = {
        1: "text", 2: "method", 3: "function", 4: "constructor",
        5: "field", 6: "variable", 7: "class", 8: "interface",
        9: "module", 10: "property", 12: "keyword", 14: "keyword",
        15: "snippet", 17: "file", 18: "reference",
    };
    return map[kind ?? 0] ?? "text";
}
export function createLspExtension(options) {
    const { lsp, fileUri, languageId, filePath, onGoToDefinition, formatOnSave } = options;
    let diagnosticsUnlisten = null;
    let documentVersion = 0;
    let isOpen = false;
    let diagnosticsEffect = StateEffect.define();
    // ── Completion source ─────────────────────────────────────────────────────
    const completionSource = async (ctx) => {
        if (!lsp.isReady())
            return null;
        const pos = cmPosToLsp(ctx.state, ctx.pos);
        try {
            const result = (await lsp.sendRequest("textDocument/completion", {
                textDocument: { uri: fileUri },
                position: pos,
            }));
            if (!result)
                return null;
            const items = Array.isArray(result) ? result : result.items ?? [];
            const options = items.map((item) => ({
                label: item.label,
                type: lspCompletionKindToType(item.kind),
                detail: item.detail,
                info: item.documentation?.value ?? item.documentation,
                apply: item.insertText ?? item.label,
                boost: item.sortText ? -1 : 0,
            }));
            // Find the word at cursor for filtering
            const word = ctx.matchBefore(/\w*/);
            const from = word ? word.from : ctx.pos;
            return { from, options };
        }
        catch {
            return null;
        }
    };
    // ── Hover tooltip ─────────────────────────────────────────────────────────
    const hoverSource = hoverTooltip(async (view, pos) => {
        if (!lsp.isReady())
            return null;
        const lspPos = cmPosToLsp(view.state, pos);
        try {
            const result = (await lsp.sendRequest("textDocument/hover", {
                textDocument: { uri: fileUri },
                position: lspPos,
            }));
            if (!result?.contents)
                return null;
            const contents = result.contents;
            let text = "";
            if (typeof contents === "string") {
                text = contents;
            }
            else if (typeof contents === "object" && "value" in contents) {
                text = contents.value;
            }
            else if (Array.isArray(contents)) {
                text = contents
                    .map((c) => (typeof c === "string" ? c : c.value ?? ""))
                    .join("\n\n");
            }
            if (!text.trim())
                return null;
            const range = result.range;
            let hoverFrom = pos;
            let hoverTo = pos;
            if (range) {
                const offsets = lspRangeToOffsets(view.state, range);
                hoverFrom = offsets.from;
                hoverTo = offsets.to;
            }
            return {
                pos: hoverFrom,
                end: hoverTo,
                above: true,
                create() {
                    const dom = document.createElement("div");
                    dom.className = "cm-lsp-hover";
                    // Render as pre-formatted text (handles markdown basics)
                    const pre = document.createElement("pre");
                    pre.textContent = text;
                    dom.appendChild(pre);
                    return { dom };
                },
            };
        }
        catch {
            return null;
        }
    });
    // ── Go-to-definition keymap ───────────────────────────────────────────────
    const gotoDefinitionKeymap = keymap.of([
        {
            key: "F12",
            run(view) {
                if (!lsp.isReady())
                    return false;
                const pos = cmPosToLsp(view.state, view.state.selection.main.head);
                lsp
                    .sendRequest("textDocument/definition", {
                    textDocument: { uri: fileUri },
                    position: pos,
                })
                    .then((result) => {
                    if (!result || !onGoToDefinition)
                        return;
                    const loc = Array.isArray(result) ? result[0] : result;
                    if (!loc)
                        return;
                    // Convert file:// URI to path
                    const targetPath = loc.uri
                        .replace(/^file:\/\//, "")
                        .replace(/^\/([A-Za-z]:)/, "$1"); // Windows drive letter
                    const line = loc.range?.start?.line ?? 0;
                    const col = loc.range?.start?.character ?? 0;
                    onGoToDefinition(targetPath, line, col);
                })
                    .catch(() => { });
                return true;
            },
        },
    ]);
    // ── Format on save ────────────────────────────────────────────────────────
    const formatKeymap = keymap.of([
        {
            key: "Mod-s",
            run(view) {
                if (!formatOnSave || !lsp.isReady())
                    return false;
                lsp
                    .sendRequest("textDocument/formatting", {
                    textDocument: { uri: fileUri },
                    options: { tabSize: 2, insertSpaces: true },
                })
                    .then((edits) => {
                    if (!edits || !Array.isArray(edits) || edits.length === 0)
                        return;
                    const changes = [];
                    for (const edit of edits) {
                        const offsets = lspRangeToOffsets(view.state, edit.range);
                        changes.push({
                            from: offsets.from,
                            to: offsets.to,
                            insert: edit.newText,
                        });
                    }
                    if (changes.length > 0) {
                        view.dispatch({ changes });
                    }
                })
                    .catch(() => { });
                return false; // Don't prevent normal save; let EditorPanel handle it
            },
        },
    ]);
    // ── View plugin (document sync + diagnostics subscription) ───────────────
    const syncPlugin = ViewPlugin.fromClass(class {
        updateTimeout = null;
        diagnosticsUnlisten = null;
        view;
        constructor(view) {
            this.view = view;
            this.openDocument(view);
        }
        update(update) {
            if (update.docChanged) {
                // Debounce document change notifications
                if (this.updateTimeout)
                    clearTimeout(this.updateTimeout);
                this.updateTimeout = setTimeout(() => {
                    this.sendDocChange(update.view);
                }, 300);
            }
        }
        openDocument(view) {
            if (!lsp.isReady()) {
                // Retry after a delay
                setTimeout(() => this.openDocument(view), 500);
                return;
            }
            documentVersion = 0;
            isOpen = true;
            lsp.sendNotification("textDocument/didOpen", {
                textDocument: {
                    uri: fileUri,
                    languageId,
                    version: documentVersion,
                    text: view.state.doc.toString(),
                },
            });
            // Subscribe to diagnostics
            this.diagnosticsUnlisten = lsp.onNotification("textDocument/publishDiagnostics", (params) => {
                if (params.uri !== fileUri)
                    return;
                const cmDiags = (params.diagnostics ?? []).map((d) => {
                    const offsets = lspRangeToOffsets(view.state, d.range);
                    return {
                        from: offsets.from,
                        to: offsets.to,
                        severity: lspSeverityToCm(d.severity),
                        message: d.message,
                        source: d.source,
                    };
                });
                view.dispatch(setDiagnostics(view.state, cmDiags));
            });
        }
        sendDocChange(view) {
            if (!lsp.isReady() || !isOpen)
                return;
            documentVersion++;
            lsp.sendNotification("textDocument/didChange", {
                textDocument: { uri: fileUri, version: documentVersion },
                contentChanges: [{ text: view.state.doc.toString() }],
            });
        }
        destroy() {
            this.diagnosticsUnlisten?.();
            if (lsp.isReady() && isOpen) {
                isOpen = false;
                lsp.sendNotification("textDocument/didClose", {
                    textDocument: { uri: fileUri },
                });
            }
        }
    });
    return [
        lintGutter(),
        linter(() => []), // Diagnostics injected via setDiagnostics
        autocompletion({ override: [completionSource] }),
        hoverSource,
        gotoDefinitionKeymap,
        formatKeymap,
        syncPlugin,
        EditorView.theme({
            ".cm-lsp-hover": {
                backgroundColor: "var(--bg-secondary, #1a1a1a)",
                border: "1px solid var(--border, #333)",
                borderRadius: "4px",
                padding: "6px 10px",
                maxWidth: "500px",
                maxHeight: "300px",
                overflowY: "auto",
                fontSize: "12px",
            },
            ".cm-lsp-hover pre": {
                margin: 0,
                fontFamily: "JetBrains Mono, monospace",
                fontSize: "12px",
                whiteSpace: "pre-wrap",
                wordBreak: "break-word",
            },
        }),
    ];
}
// ─── LspClient: in-browser JSON-RPC client over Tauri invoke ─────────────────
let lspRequestId = 1;
const pendingRequests = new Map();
const notificationHandlers = new Map();
let _lspClientInitialized = false;
/** Wire up incoming LSP messages from Rust to the pending-request/notification system.
 *  Must be called once on app startup (e.g. in StatusBar onMount).
 *  Safe to call multiple times — subsequent calls are no-ops. */
export function initLspClient(lspStore) {
    if (_lspClientInitialized)
        return;
    _lspClientInitialized = true;
    lspStore.onMessage((_language, message) => {
        routeLspMessage(message);
    });
}
function routeLspMessage(message) {
    const msg = message;
    if (msg.id !== undefined && !msg.method) {
        // Response
        const pending = pendingRequests.get(msg.id);
        if (pending) {
            pendingRequests.delete(msg.id);
            if (msg.error) {
                pending.reject(msg.error);
            }
            else {
                pending.resolve(msg.result);
            }
        }
    }
    else if (msg.method) {
        // Notification or request from server
        const handlers = notificationHandlers.get(msg.method);
        if (handlers) {
            handlers.forEach((h) => h(msg.params));
        }
    }
}
export function createLspInterface(language, lspStore) {
    return {
        isReady() {
            return lspStore.getStatus(language) === "ready";
        },
        sendRequest(method, params) {
            return new Promise((resolve, reject) => {
                const id = lspRequestId++;
                pendingRequests.set(id, { resolve, reject });
                lspStore.send(language, {
                    jsonrpc: "2.0",
                    id,
                    method,
                    params,
                });
                // Timeout after 10 seconds
                setTimeout(() => {
                    if (pendingRequests.has(id)) {
                        pendingRequests.delete(id);
                        reject(new Error(`LSP request timed out: ${method}`));
                    }
                }, 10000);
            });
        },
        sendNotification(method, params) {
            lspStore.send(language, {
                jsonrpc: "2.0",
                method,
                params,
            });
        },
        onNotification(method, handler) {
            if (!notificationHandlers.has(method)) {
                notificationHandlers.set(method, new Set());
            }
            notificationHandlers.get(method).add(handler);
            return () => {
                notificationHandlers.get(method)?.delete(handler);
            };
        },
    };
}
// Helper: convert a file path to a file:// URI
export function pathToUri(filePath) {
    if (filePath.startsWith("file://"))
        return filePath;
    // Windows path like C:\... -> file:///C:/...
    if (/^[A-Za-z]:/.test(filePath)) {
        return "file:///" + filePath.replace(/\\/g, "/");
    }
    return "file://" + filePath;
}
// Helper: detect language ID from file extension
export function detectLanguageId(filePath) {
    const ext = filePath.split(".").pop()?.toLowerCase();
    const map = {
        js: "javascript",
        jsx: "javascript",
        mjs: "javascript",
        cjs: "javascript",
        ts: "typescript",
        tsx: "typescript",
        py: "python",
        rs: "rust",
        go: "go",
    };
    return ext ? map[ext] ?? null : null;
}
