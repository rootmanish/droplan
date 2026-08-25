import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";
import tailwindcss from "@tailwindcss/vite";
import path from "node:path";

// Served from the domain root (droplan.devopsinfo.in), both in dev and on
// GitHub Pages behind the CNAME, so asset paths never need a repo-name base.
export default defineConfig({
  base: "/",
  plugins: [react(), tailwindcss()],
  resolve: {
    alias: { "@": path.resolve(import.meta.dirname, "./src") },
  },
  build: {
    outDir: "dist",
    assetsDir: "assets",
  },
});
