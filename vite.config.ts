import { defineConfig } from "vite";
import vue from "@vitejs/plugin-vue";
import tailwindcss from "@tailwindcss/vite";

// Tauri drives this dev server on a fixed port.
export default defineConfig({
  plugins: [vue(), tailwindcss()],
  clearScreen: false,
  server: {
    port: 1420,
    strictPort: true,
    // src-tauri/target holds the running exe; watching it spams EBUSY and blanks the webview.
    watch: { ignored: ["**/src-tauri/**"] },
  },
});
