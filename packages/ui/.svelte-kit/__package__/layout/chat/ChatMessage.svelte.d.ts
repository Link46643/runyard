import type { Message } from "@runyard/common";
type $$ComponentProps = {
    message: Message;
    onOpenFile?: (path: string) => void;
};
declare const ChatMessage: import("svelte").Component<$$ComponentProps, {}, "">;
type ChatMessage = ReturnType<typeof ChatMessage>;
export default ChatMessage;
