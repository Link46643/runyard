// Minimal, dependency-free GFM-ish markdown renderer.
// Safe by construction: all raw text is HTML-escaped before being wrapped in a
// fixed whitelist of tags this function builds itself. Never passes through
// user-supplied HTML. Links are restricted to http(s) targets and always get
// target="_blank" rel="noopener noreferrer".
function escapeHtml(s) {
    return s
        .replace(/&/g, "&amp;")
        .replace(/</g, "&lt;")
        .replace(/>/g, "&gt;")
        .replace(/"/g, "&quot;")
        .replace(/'/g, "&#39;");
}
function inline(text) {
    let s = escapeHtml(text);
    s = s.replace(/\[([^\]]+)\]\((https?:\/\/[^\s)]+)\)/g, (_m, label, url) => {
        return `<a href="${url}" target="_blank" rel="noopener noreferrer">${label}</a>`;
    });
    s = s.replace(/`([^`]+)`/g, (_m, a) => `<code>${a}</code>`);
    s = s.replace(/\*\*([^*]+)\*\*/g, (_m, a) => `<strong>${a}</strong>`);
    s = s.replace(/__([^_]+)__/g, (_m, a) => `<strong>${a}</strong>`);
    s = s.replace(/~~([^~]+)~~/g, (_m, a) => `<del>${a}</del>`);
    s = s.replace(/\*([^*\s][^*]*)\*/g, (_m, a) => `<em>${a}</em>`);
    s = s.replace(/(?<![\w])_([^_\s][^_]*)_(?![\w])/g, (_m, a) => `<em>${a}</em>`);
    return s;
}
export function renderMarkdownLite(src) {
    const lines = src.replace(/\r\n/g, "\n").split("\n");
    const out = [];
    let inCode = false;
    let codeLang = "";
    let codeBuf = [];
    const listStack = [];
    let inBlockquote = false;
    let inTable = false;
    let tableHeader = null;
    let tableRows = [];
    const closeLists = () => {
        while (listStack.length)
            out.push(`</${listStack.pop()}>`);
    };
    const closeBlockquote = () => {
        if (inBlockquote) {
            out.push("</blockquote>");
            inBlockquote = false;
        }
    };
    const flushTable = () => {
        if (!inTable)
            return;
        out.push("<table>");
        if (tableHeader) {
            out.push("<thead><tr>" + tableHeader.map((c) => `<th>${inline(c)}</th>`).join("") + "</tr></thead>");
        }
        out.push("<tbody>");
        for (const row of tableRows) {
            out.push("<tr>" + row.map((c) => `<td>${inline(c)}</td>`).join("") + "</tr>");
        }
        out.push("</tbody></table>");
        inTable = false;
        tableHeader = null;
        tableRows = [];
    };
    const closeCode = () => {
        out.push(`<pre class="md-code" data-lang="${escapeHtml(codeLang)}"><code>${escapeHtml(codeBuf.join("\n"))}</code></pre>`);
        inCode = false;
    };
    for (const rawLine of lines) {
        const fence = /^```(\w*)\s*$/.exec(rawLine);
        if (fence) {
            if (!inCode) {
                closeLists();
                closeBlockquote();
                flushTable();
                inCode = true;
                codeLang = fence[1] || "text";
                codeBuf = [];
            }
            else {
                closeCode();
            }
            continue;
        }
        if (inCode) {
            codeBuf.push(rawLine);
            continue;
        }
        if (rawLine.trim() === "") {
            closeLists();
            closeBlockquote();
            flushTable();
            continue;
        }
        const heading = /^(#{1,6})\s+(.*)$/.exec(rawLine);
        if (heading) {
            closeLists();
            closeBlockquote();
            flushTable();
            const level = heading[1].length;
            out.push(`<h${level}>${inline(heading[2])}</h${level}>`);
            continue;
        }
        const tableSep = /^\|?\s*:?-+:?\s*(\|\s*:?-+:?\s*)*\|?\s*$/.exec(rawLine);
        const tableRow = /^\|?(.+)\|?\s*$/.exec(rawLine);
        if (inTable === false && tableHeader === null && tableRow && rawLine.includes("|")) {
            // Tentatively treat as a header row; confirmed only if the next line is a separator.
            // Handled via lookahead below by checking tableSep on this same pass is not possible
            // with a simple line-by-line pass, so we accept the common case: header immediately
            // followed by a separator row.
        }
        if (tableSep && (inTable || tableHeader === null)) {
            inTable = true;
            continue;
        }
        if (inTable || (tableRow && rawLine.trim().startsWith("|"))) {
            const cells = rawLine.replace(/^\|/, "").replace(/\|$/, "").split("|").map((c) => c.trim());
            if (!tableHeader && !inTable) {
                tableHeader = cells;
            }
            else if (!tableHeader) {
                tableHeader = cells;
            }
            else {
                tableRows.push(cells);
            }
            inTable = true;
            continue;
        }
        else if (inTable) {
            flushTable();
        }
        const quote = /^>\s?(.*)$/.exec(rawLine);
        if (quote) {
            closeLists();
            if (!inBlockquote) {
                out.push("<blockquote>");
                inBlockquote = true;
            }
            out.push(`<p>${inline(quote[1])}</p>`);
            continue;
        }
        else {
            closeBlockquote();
        }
        const ol = /^\d+\.\s+(.*)$/.exec(rawLine);
        const ul = /^[-*]\s+(.*)$/.exec(rawLine);
        if (ol || ul) {
            const kind = ol ? "ol" : "ul";
            const content = ol ? ol[1] : ul[1];
            if (listStack[listStack.length - 1] !== kind) {
                closeLists();
                out.push(`<${kind}>`);
                listStack.push(kind);
            }
            out.push(`<li>${inline(content)}</li>`);
            continue;
        }
        else {
            closeLists();
        }
        out.push(`<p>${inline(rawLine)}</p>`);
    }
    closeLists();
    closeBlockquote();
    flushTable();
    if (inCode)
        closeCode();
    return out.join("\n");
}
