import { defineConfig } from "vite";
import preact from "@preact/preset-vite";
import tailwindcss from "@tailwindcss/vite";
import path from "node:path";

export default defineConfig({
  plugins: [tailwindcss(), preact()],
  build: {
    outDir: path.resolve(__dirname, "frontend/dist"),
    assetsDir: "assets",
    emptyOutDir: true,
    manifest: true,
    sourcemap: true,
    rollupOptions: {
      input: path.resolve(__dirname, "frontend/src/main.tsx"),
      output: {
        entryFileNames: "assets/[name]-[hash].js",
        chunkFileNames: "assets/[name]-[hash].js",
        assetFileNames: "assets/[name]-[hash][extname]",
      },
    },
  },
  resolve: {
    alias: {
      "@": path.resolve(__dirname, "frontend/src"),
    },
  },
});
