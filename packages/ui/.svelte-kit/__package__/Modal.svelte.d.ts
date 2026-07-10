type $$ComponentProps = {
    title: string;
    message: string;
    onConfirm: () => void;
    onCancel?: () => void;
    confirmLabel?: string;
    cancelLabel?: string;
    show: boolean;
    children?: import("svelte").Snippet;
};
declare const Modal: import("svelte").Component<$$ComponentProps, {}, "show">;
type Modal = ReturnType<typeof Modal>;
export default Modal;
