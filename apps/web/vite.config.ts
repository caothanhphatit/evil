import { defineConfig } from "vitest/config";
import type { PreviewServer, ViteDevServer } from "vite";
import { createReadStream } from "node:fs";
import { resolve } from "node:path";

const registryUrl = "/content/releases/evil-hunter-1.411/building-registry.json";
const registryPath = resolve(import.meta.dirname, "../../packages/content/releases/evil-hunter-1.411/building-registry.json");
const gearCatalogUrl = "/content/releases/evil-hunter-1.411/gear-catalog.json";
const gearCatalogPath = resolve(import.meta.dirname, "../../packages/content/releases/evil-hunter-1.411/gear-catalog.json");

function serveBuildingRegistry() {
  return (server: ViteDevServer | PreviewServer) => {
    server.middlewares.use((request, response, next) => {
      const url = request.url?.split("?", 1)[0];
      const source = url === registryUrl ? registryPath : url === gearCatalogUrl ? gearCatalogPath : null;
      if (!source) return next();
      response.setHeader("Content-Type", "application/json");
      createReadStream(source).pipe(response);
    });
  };
}

export default defineConfig({
  plugins: [{
    name: "serve-building-evidence-registry",
    configureServer: serveBuildingRegistry(),
    configurePreviewServer: serveBuildingRegistry(),
  }],
  server: {
    port: 5173,
    strictPort: true,
    proxy: {
      "/session": "http://127.0.0.1:8080",
      "/ws": {
        target: "ws://127.0.0.1:8080",
        ws: true,
      },
    },
  },
  preview: {
    port: 5173,
  },
  test: {
    environment: "node",
    include: ["src/**/*.test.ts"],
  },
});
