import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";

export default defineConfig({
  plugins: [react()],
  clearScreen: false,
  server: {
    port: 1420,
    strictPort: true,
    watch: {
      ignored: ["**/src-tauri/**"],
    },
  },
  build: {
    target: "es2021",
    // Keep built frontend assets out of the gui source tree.  The Tauri
    // config (`frontendDist`) points at the same root-level directory.
    outDir: "../build/gui-dist",
    emptyOutDir: true,
  },
});
