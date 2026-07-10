import type { ContentBlock } from "@runyard/common";
type $$ComponentProps = {
    content: ContentBlock[];
    onExplainCode?: (code: string, language: string) => void;
    onOpenFile?: (path: string) => void;
    onPermissionDecision?: (block: ContentBlock, approved: boolean) => void;
};
declare const ContentBlockRenderer: import("svelte").Component<$$ComponentProps, {}, "">;
type ContentBlockRenderer = ReturnType<typeof ContentBlockRenderer>;
export default ContentBlockRenderer;
