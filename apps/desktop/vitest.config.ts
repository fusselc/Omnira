import { defineConfig } from "vitest/config";

// Minimal test config, separate from vite.config.ts so the Tauri dev/build
// server config (fixed port, HMR host, watch ignores) stays untouched.
export default defineConfig({
  test: {
    environment: "node",
    include: ["src/**/*.test.{ts,tsx}"],
  },
});
