// @ts-ignore
import { invoke as tauriInvoke } from "original-tauri-core";
import { webSocketClient } from "./websocket-client";

export function invoke<T>(cmd: string, args?: any): Promise<T> {
  if (typeof window !== "undefined" && (window as any).__TAURI_INTERNALS__) {
    return tauriInvoke<T>(cmd, args);
  } else {
    return webSocketClient.invoke<T>(cmd, args);
  }
}
