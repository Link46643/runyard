import type { Message } from "@runyard/common";
type $$ComponentProps = {
    messages: Message[];
    onOpenFile?: (path: string) => void;
};
declare const ChatMessageList: import("svelte").Component<$$ComponentProps, {}, "">;
type ChatMessageList = ReturnType<typeof ChatMessageList>;
export default ChatMessageList;
