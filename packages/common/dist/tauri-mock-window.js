// @ts-ignore
import { getCurrentWindow as tauriGetCurrentWindow } from "original-tauri-window";
export function getCurrentWindow() {
    if (typeof window !== "undefined" && window.__TAURI_INTERNALS__) {
        return tauriGetCurrentWindow();
    }
    else {
        return {
            show: () => {
                console.log("[TauriMockWindow] show called");
            },
            hide: () => {
                console.log("[TauriMockWindow] hide called");
            },
            close: () => {
                console.log("[TauriMockWindow] close called");
            },
            minimize: () => {
                console.log("[TauriMockWindow] minimize called");
            },
            maximize: () => {
                console.log("[TauriMockWindow] maximize called");
            }
        };
    }
}
