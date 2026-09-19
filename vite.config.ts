import { fileURLToPath } from "node:url";
import { defineConfig } from "vite";

// The page is served only to the Tauri webview; nothing here reaches a network.
export default defineConfig({
  clearScreen: false,
  resolve: { alias: { "@": fileURLToPath(new URL("./src", import.meta.url)) } },
  server: { port: 5173, strictPort: true },
  build: { target: "es2023", sourcemap: false },
});
