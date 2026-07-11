// IPC batching utility (engineering-todo-v2.md 1.14.1).
// Coalesces rapid Tauri IPC calls within a time window to reduce roundtrips.
// Particularly useful for: file watcher events, LSP diagnostics, any store
// method called on every keystroke/scroll/resize.

/**
 * Wraps an async function in a debounce+deduplication layer. Rapid calls
 * within `delayMs` are collapsed into a single call; all callers share the
 * same in-flight promise so no duplicate network round-trips occur.
 */
export function batchInvoke<T>(fn: () => Promise<T>, delayMs = 100): () => Promise<T> {
  let timer: ReturnType<typeof setTimeout> | null = null;
  let pending: { resolve: (v: T) => void; reject: (e: unknown) => void }[] = [];
  let inFlight: Promise<T> | null = null;

  return () => {
    if (inFlight) return inFlight;

    const p = new Promise<T>((resolve, reject) => {
      pending.push({ resolve, reject });
      if (timer) clearTimeout(timer);
      timer = setTimeout(() => {
        timer = null;
        const batch = pending.splice(0);
        inFlight = fn()
          .then((result) => {
            inFlight = null;
            batch.forEach((b) => b.resolve(result));
            return result;
          })
          .catch((err) => {
            inFlight = null;
            batch.forEach((b) => b.reject(err));
            throw err;
          });
      }, delayMs);
    });

    return p;
  };
}

/**
 * Throttle: runs `fn` at most once per `intervalMs`.
 * Unlike debounce, the first call fires immediately.
 */
export function throttleInvoke<T>(fn: () => Promise<T>, intervalMs = 500): () => Promise<T> {
  let lastRun = 0;
  let pending: Promise<T> | null = null;

  return async () => {
    const now = Date.now();
    if (pending) return pending;
    if (now - lastRun >= intervalMs) {
      lastRun = now;
      pending = fn().finally(() => { pending = null; });
      return pending;
    }
    // Within throttle window — schedule for next window.
    return new Promise<T>((resolve, reject) => {
      setTimeout(() => {
        lastRun = Date.now();
        fn().then(resolve).catch(reject);
      }, intervalMs - (now - lastRun));
    });
  };
}
