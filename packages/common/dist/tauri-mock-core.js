// @ts-ignore
import { invoke as tauriInvoke } from "original-tauri-core";
import { webSocketClient } from "./websocket-client";
export function invoke(cmd, args) {
    if (typeof window !== "undefined" && window.__TAURI_INTERNALS__) {
        return tauriInvoke(cmd, args);
    }
    else {
        return webSocketClient.invoke(cmd, args);
    }
}
