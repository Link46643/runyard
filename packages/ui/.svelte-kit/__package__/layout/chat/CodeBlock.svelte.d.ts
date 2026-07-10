import type { CodeBlock } from "@runyard/common";
type $$ComponentProps = {
    block: CodeBlock;
    onExplain?: (code: string, language: string) => void;
};
declare const CodeBlock: import("svelte").Component<$$ComponentProps, {}, "">;
type CodeBlock = ReturnType<typeof CodeBlock>;
export default CodeBlock;
