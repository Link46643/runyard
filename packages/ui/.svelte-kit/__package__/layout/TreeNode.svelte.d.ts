import type { FsEntry } from "@runyard/common";
import TreeNode from "./TreeNode.svelte";
type $$ComponentProps = {
    node: FsEntry;
    onOpenFile: (path: string, name: string) => void;
    depth?: number;
};
declare const TreeNode: import("svelte").Component<$$ComponentProps, {
    refresh: () => Promise<void>;
}, "">;
type TreeNode = ReturnType<typeof TreeNode>;
export default TreeNode;
