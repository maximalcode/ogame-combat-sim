import { defineConfig } from "@playwright/test";
const port = process.env["PLAYWRIGHT_PORT"] ?? "5175";
export default defineConfig({
  testDir: "./tests",
  use: {
    baseURL: `http://127.0.0.1:${port}`,
    headless: true,
    launchOptions: process.env["PLAYWRIGHT_CHROME"]
      ? { executablePath: process.env["PLAYWRIGHT_CHROME"] }
      : {},
  },
  webServer: {
    command: `npm run dev -- --host 127.0.0.1 --port ${port} --strictPort`,
    url: `http://127.0.0.1:${port}`,
    reuseExistingServer: !process.env["CI"],
  },
});
