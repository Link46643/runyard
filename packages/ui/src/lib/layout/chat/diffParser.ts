// Parses a unified-diff string into structured hunks so the UI can offer
// per-hunk accept/reject even though DiffBlock only stores a flat diff string.

export interface ParsedDiffLine {
  type: "add" | "del" | "context";
  content: string;
  oldLine?: number;
  newLine?: number;
}

export type HunkStatus = "pending" | "accepted" | "rejected";

export interface ParsedDiffHunk {
  id: string;
  header: string;
  lines: ParsedDiffLine[];
  status: HunkStatus;
}

export function parseUnifiedDiff(diff: string): ParsedDiffHunk[] {
  const lines = diff.replace(/\r\n/g, "\n").split("\n");
  const hunks: ParsedDiffHunk[] = [];
  let current: ParsedDiffHunk | null = null;
  let oldLine = 0;
  let newLine = 0;
  let hunkIndex = 0;

  for (const line of lines) {
    const hunkMatch = /^@@ -(\d+)(?:,\d+)? \+(\d+)(?:,\d+)? @@.*$/.exec(line);
    if (hunkMatch) {
      if (current) hunks.push(current);
      oldLine = parseInt(hunkMatch[1], 10);
      newLine = parseInt(hunkMatch[2], 10);
      current = { id: `hunk-${hunkIndex++}`, header: line, lines: [], status: "pending" };
      continue;
    }
    if (line.startsWith("diff --git") || line.startsWith("index ") || line.startsWith("--- ") || line.startsWith("+++ ")) {
      continue;
    }
    if (!current) continue;

    if (line.startsWith("+")) {
      current.lines.push({ type: "add", content: line.slice(1), newLine: newLine++ });
    } else if (line.startsWith("-")) {
      current.lines.push({ type: "del", content: line.slice(1), oldLine: oldLine++ });
    } else {
      current.lines.push({ type: "context", content: line.slice(1), oldLine: oldLine++, newLine: newLine++ });
    }
  }
  if (current) hunks.push(current);

  // A diff with no @@ headers at all (just a raw block) is treated as one hunk.
  if (hunks.length === 0 && diff.trim().length > 0) {
    hunks.push({
      id: "hunk-0",
      header: "",
      status: "pending",
      lines: lines
        .filter((l) => !l.startsWith("diff --git") && !l.startsWith("index ") && !l.startsWith("--- ") && !l.startsWith("+++ "))
        .map((l) => {
          if (l.startsWith("+")) return { type: "add" as const, content: l.slice(1) };
          if (l.startsWith("-")) return { type: "del" as const, content: l.slice(1) };
          return { type: "context" as const, content: l.replace(/^ /, "") };
        }),
    });
  }

  return hunks;
}

export interface PairedDiffRow {
  left: ParsedDiffLine | null;
  right: ParsedDiffLine | null;
}

/** Pairs up del/add lines side by side for the side-by-side diff view.
 * Consecutive deletions and additions are zipped together; context lines
 * appear identically on both sides. */
export function pairHunkLines(lines: ParsedDiffLine[]): PairedDiffRow[] {
  const rows: PairedDiffRow[] = [];
  let i = 0;
  while (i < lines.length) {
    const line = lines[i];
    if (line.type === "context") {
      rows.push({ left: line, right: line });
      i++;
      continue;
    }
    // Collect a run of deletions followed by a run of additions
    const dels: ParsedDiffLine[] = [];
    while (i < lines.length && lines[i].type === "del") {
      dels.push(lines[i]);
      i++;
    }
    const adds: ParsedDiffLine[] = [];
    while (i < lines.length && lines[i].type === "add") {
      adds.push(lines[i]);
      i++;
    }
    const max = Math.max(dels.length, adds.length);
    for (let j = 0; j < max; j++) {
      rows.push({ left: dels[j] ?? null, right: adds[j] ?? null });
    }
  }
  return rows;
}

/** Reconstructs file content by applying only the accepted hunks' additions
 * on top of the original content lines. Best-effort: assumes hunks are
 * ordered and non-overlapping, which unified diffs always are. */
export function applyAcceptedHunks(hunks: ParsedDiffHunk[]): string {
  const parts: string[] = [];
  for (const hunk of hunks) {
    if (hunk.status !== "accepted") continue;
    for (const line of hunk.lines) {
      if (line.type === "add" || line.type === "context") {
        parts.push(line.content);
      }
    }
  }
  return parts.join("\n");
}
