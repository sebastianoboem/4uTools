import { resolve } from "node:path";
import { defineConfig } from "vite";

export default defineConfig({
  build: {
    emptyOutDir: true,
    lib: {
      entry: resolve(__dirname, "src/tabs/gphotos/engine/inject-entry.ts"),
      name: "GoogleFotoInject",
      formats: ["iife"],
      fileName: () => "gfm-engine.js",
    },
    outDir: resolve(__dirname, "src-tauri/gen-assets"),
    rollupOptions: {
      output: {
        inlineDynamicImports: true,
      },
    },
  },
});
