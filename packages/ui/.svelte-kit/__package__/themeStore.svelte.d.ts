declare class ThemeStore {
    current: "dark" | "light";
    constructor();
    load(): void;
    save(): void;
    toggle(): void;
    set(theme: "light" | "dark"): void;
    apply(): void;
}
export declare const theme: ThemeStore;
export {};
