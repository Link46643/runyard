// Parses a unified-diff string into structured hunks so the UI can offer
// per-hunk accept/reject even though DiffBlock only stores a flat diff string.
export function parseUnifiedDiff(diff) {
    const lines = diff.replace(/\r\n/g, "\n").split("\n");
    const hunks = [];
    let current = null;
    let oldLine = 0;
    let newLine = 0;
    let hunkIndex = 0;
    for (const line of lines) {
        const hunkMatch = /^@@ -(\d+)(?:,\d+)? \+(\d+)(?:,\d+)? @@.*$/.exec(line);
        if (hunkMatch) {
            if (current)
                hunks.push(current);
            oldLine = parseInt(hunkMatch[1], 10);
            newLine = parseInt(hunkMatch[2], 10);
            current = { id: `hunk-${hunkIndex++}`, header: line, lines: [], status: "pending" };
            continue;
        }
        if (line.startsWith("diff --git") || line.startsWith("index ") || line.startsWith("--- ") || line.startsWith("+++ ")) {
            continue;
        }
        if (!current)
            continue;
        if (line.startsWith("+")) {
            current.lines.push({ type: "add", content: line.slice(1), newLine: newLine++ });
        }
        else if (line.startsWith("-")) {
            current.lines.push({ type: "del", content: line.slice(1), oldLine: oldLine++ });
        }
        else {
            current.lines.push({ type: "context", content: line.slice(1), oldLine: oldLine++, newLine: newLine++ });
        }
    }
    if (current)
        hunks.push(current);
    // A diff with no @@ headers at all (just a raw block) is treated as one hunk.
    if (hunks.length === 0 && diff.trim().length > 0) {
        hunks.push({
            id: "hunk-0",
            header: "",
            status: "pending",
            lines: lines
                .filter((l) => !l.startsWith("diff --git") && !l.startsWith("index ") && !l.startsWith("--- ") && !l.startsWith("+++ "))
                .map((l) => {
                if (l.startsWith("+"))
                    return { type: "add", content: l.slice(1) };
                if (l.startsWith("-"))
                    return { type: "del", content: l.slice(1) };
                return { type: "context", content: l.replace(/^ /, "") };
            }),
        });
    }
    return hunks;
}
/** Reconstructs file content by applying only the accepted hunks' additions
 * on top of the original content lines. Best-effort: assumes hunks are
 * ordered and non-overlapping, which unified diffs always are. */
export function applyAcceptedHunks(hunks) {
    const parts = [];
    for (const hunk of hunks) {
        if (hunk.status !== "accepted")
            continue;
        for (const line of hunk.lines) {
            if (line.type === "add" || line.type === "context") {
                parts.push(line.content);
            }
        }
    }
    return parts.join("\n");
}
