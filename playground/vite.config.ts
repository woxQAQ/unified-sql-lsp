import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";

export default defineConfig({
  plugins: [react()],
  base: '/unified-sql-lsp/',
  server: {
    port: 3001,
    open: true,
  },
  build: {
    rollupOptions: {
      output: {
        manualChunks: {
          "monaco-editor": ["monaco-editor"],
        },
      },
    },
    chunkSizeWarningLimit: 1000,
  },
});
