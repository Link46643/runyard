// @ts-ignore
import { listen as tauriListen } from "original-tauri-event";
import { webSocketClient } from "./websocket-client";
export function listen(event, handler) {
    if (typeof window !== "undefined" && window.__TAURI_INTERNALS__) {
        return tauriListen(event, handler);
    }
    else {
        return webSocketClient.listen(event, handler);
    }
}
