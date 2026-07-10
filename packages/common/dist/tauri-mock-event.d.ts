export type UnlistenFn = () => void;
export declare function listen<T>(event: string, handler: (event: {
    payload: T;
    event: string;
}) => void): Promise<UnlistenFn>;
