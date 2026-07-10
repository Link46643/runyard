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
export declare function parseUnifiedDiff(diff: string): ParsedDiffHunk[];
/** Reconstructs file content by applying only the accepted hunks' additions
 * on top of the original content lines. Best-effort: assumes hunks are
 * ordered and non-overlapping, which unified diffs always are. */
export declare function applyAcceptedHunks(hunks: ParsedDiffHunk[]): string;
