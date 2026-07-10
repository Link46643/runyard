// @ts-ignore
import { listen as tauriListen } from "original-tauri-event";
import { webSocketClient } from "./websocket-client";

export type UnlistenFn = () => void;

export function listen<T>(
  event: string,
  handler: (event: { payload: T; event: string }) => void
): Promise<UnlistenFn> {
  if (typeof window !== "undefined" && (window as any).__TAURI_INTERNALS__) {
    return tauriListen<T>(event, handler);
  } else {
    return webSocketClient.listen<T>(event, handler);
  }
}
