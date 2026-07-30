import { defineConfig } from "vite";
import tailwindcss from "@tailwindcss/vite";

export default defineConfig({
  base: process.env.VITE_BASE_PATH ?? "/",
  plugins: [tailwindcss()],
  server: {
    port: 5174,
    strictPort: true,
    proxy: { "/admin": "http://127.0.0.1:8080" },
  },
  preview: { port: 5174 },
});
