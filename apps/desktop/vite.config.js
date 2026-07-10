import { defineConfig } from "vite";
import { sveltekit } from "@sveltejs/kit/vite";
import path from "path";

// process is a nodejs global
const host = process.env.TAURI_DEV_HOST;

// https://vite.dev/config/
export default defineConfig(async () => ({
  plugins: [sveltekit()],

  // Vite options tailored for Tauri development
  resolve: {
    alias: {
      "original-tauri-core": path.resolve(__dirname, "node_modules/@tauri-apps/api/core.js"),
      "original-tauri-event": path.resolve(__dirname, "node_modules/@tauri-apps/api/event.js"),
      "original-tauri-window": path.resolve(__dirname, "node_modules/@tauri-apps/api/window.js"),
      "@tauri-apps/api/core": path.resolve(__dirname, "../../packages/common/src/tauri-mock-core.ts"),
      "@tauri-apps/api/event": path.resolve(__dirname, "../../packages/common/src/tauri-mock-event.ts"),
      "@tauri-apps/api/window": path.resolve(__dirname, "../../packages/common/src/tauri-mock-window.ts"),
      "@runyard/ui": path.resolve(__dirname, "../../packages/ui/src/lib"),
      "@runyard/editor": path.resolve(__dirname, "../../packages/editor/src"),
      "@runyard/protocol": path.resolve(__dirname, "../../packages/protocol/src"),
      "@runyard/common": path.resolve(__dirname, "../../packages/common/src")
    },
    dedupe: [
      "@codemirror/state",
      "@codemirror/view",
      "@codemirror/basic-setup",
      "@codemirror/lang-javascript",
      "@codemirror/lang-python",
      "@codemirror/theme-one-dark",
      "@codemirror/language",
      "@codemirror/commands",
      "@codemirror/search",
      "@codemirror/autocomplete"
    ]
  },
  optimizeDeps: {
    include: [
      "@codemirror/state",
      "@codemirror/view",
      "@codemirror/basic-setup",
      "@codemirror/lang-javascript",
      "@codemirror/lang-python",
      "@codemirror/theme-one-dark",
      "@codemirror/language",
      "@codemirror/commands",
      "@codemirror/search",
      "@codemirror/autocomplete",
      "@fontsource-variable/google-sans-flex"
    ]
  },

  server: {
    fs: {
      allow: ["../../"] // Allow workspace root
    },
    port: 1420,
    strictPort: true,
    host: host || false,
    hmr: host
      ? {
          protocol: "ws",
          host,
          port: 1421,
        }
      : undefined,
    watch: {
      // 3. tell Vite to ignore watching `src-tauri`
      ignored: ["**/src-tauri/**"],
    },
  },
}));
