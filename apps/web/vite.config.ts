import { defineConfig } from "vitest/config";

export default defineConfig({
  server: {
    port: 5173,
    strictPort: true,
  },
  preview: {
    port: 5173,
  },
  test: {
    environment: "node",
    include: ["src/**/*.test.ts"],
  },
});
