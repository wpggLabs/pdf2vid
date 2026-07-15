import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";

// `process` is a Node global available when Vite runs under Node, but it
// is only typed when `@types/node` is installed. Read it via `globalThis`
// with a defensive cast so this file type-checks with or without those
// types (an unguarded `@ts-expect-error` would itself error as "unused"
// once `@types/node` is added).
const host = (
  typeof globalThis !== "undefined" &&
  (globalThis as { process?: { env?: Record<string, string | undefined> } }).process
    ?.env?.T_AURI_DEV_HOST
) as string | undefined;

// https://vite.dev/config/
export default defineConfig(async () => ({
  plugins: [react()],

  // Vite options tailored for Tauri development and only applied in `tauri dev` or `tauri build`
  //
  // 1. prevent Vite from obscuring rust errors
  clearScreen: false,
  // 2. tauri expects a fixed port, fail if that port is not available
  server: {
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
