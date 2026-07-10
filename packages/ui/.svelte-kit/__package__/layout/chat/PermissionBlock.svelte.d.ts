import type { PermissionBlock } from "@runyard/common";
type $$ComponentProps = {
    block: PermissionBlock;
    onDecide?: (approved: boolean) => void;
};
declare const PermissionBlock: import("svelte").Component<$$ComponentProps, {}, "">;
type PermissionBlock = ReturnType<typeof PermissionBlock>;
export default PermissionBlock;
