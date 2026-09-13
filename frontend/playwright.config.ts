import { defineConfig } from "@playwright/test";
export default defineConfig({
  testDir: "./tests",
  use: {
    baseURL: "http://127.0.0.1:5175",
    headless: true,
    launchOptions: process.env["PLAYWRIGHT_CHROME"]
      ? { executablePath: process.env["PLAYWRIGHT_CHROME"] }
      : {},
  },
  webServer: {
    command: "npm run dev -- --host 127.0.0.1 --port 5175 --strictPort",
    url: "http://127.0.0.1:5175",
    reuseExistingServer: !process.env["CI"],
  },
});
