import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";
import { fileURLToPath, URL } from "node:url";

// The API server's port is environment-driven (PORT env on combat-api, default
// 3000), so the frontend cannot hardcode localhost:3000. Vite exposes the
// configured base URL to the client through `import.meta.env.VITE_API_BASE_URL`
// (see src/config.ts); the dev server also proxies `/api` there so a frontend
// with no env set still talks to a locally running API without CORS setup.
const apiBaseUrl = process.env.VITE_API_BASE_URL ?? "http://localhost:3000";

export default defineConfig({
  plugins: [react()],
  resolve: {
    alias: {
      "@": fileURLToPath(new URL("./src", import.meta.url)),
    },
  },
  server: {
    port: 5173,
    proxy: {
      "/api": {
        target: apiBaseUrl,
        changeOrigin: true,
      },
    },
  },
});
