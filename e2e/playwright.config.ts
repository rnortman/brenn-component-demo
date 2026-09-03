import { defineConfig, devices } from "@playwright/test";

const baseURL = process.env.BRENN_E2E_BASE_URL;
if (!baseURL) {
  throw new Error(
    "BRENN_E2E_BASE_URL must be set (e.g. http://127.0.0.1:3300); the `make e2e` target exports it.",
  );
}

// Written by global-setup.ts; every spec starts already authenticated.
export const STORAGE_STATE = ".auth/user.json";

export default defineConfig({
  testDir: "./tests",
  // The two pages share a server. `demo-split` runs its counter on the backend
  // over ephemeral channels that outlive the navigation, so the specs are
  // sequential and each establishes its own preconditions.
  fullyParallel: false,
  workers: 1,
  forbidOnly: true,
  retries: 0,
  // Per-assertion budgets (CHAIN_TIMEOUT in the spec) are 20s and a test chains
  // several, so the overall timeout must dominate rather than clip at the 30s
  // default.
  timeout: 90_000,
  reporter: "list",
  globalSetup: "./global-setup.ts",
  use: {
    baseURL,
    storageState: STORAGE_STATE,
    trace: "retain-on-failure",
  },
  projects: [
    {
      name: "chromium",
      use: { ...devices["Desktop Chrome"] },
    },
  ],
});
