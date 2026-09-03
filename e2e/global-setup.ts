import { chromium } from "@playwright/test";
import * as path from "path";
import { STORAGE_STATE } from "./playwright.config";

// Password must be >= 12 chars; username must be alphanumeric.
const E2E_USERNAME = "e2e-user";
const E2E_PASSWORD = "e2e-password-12";

/**
 * Registers and logs in a single user through the real auth forms, then saves
 * the authenticated session as Playwright storageState for every spec.
 */
async function globalSetup(): Promise<void> {
  const baseURL = process.env.BRENN_E2E_BASE_URL;
  if (!baseURL) {
    throw new Error("BRENN_E2E_BASE_URL must be set for e2e global setup.");
  }
  const inviteCode = process.env.BRENN_E2E_INVITE;
  if (!inviteCode) {
    throw new Error(
      "BRENN_E2E_INVITE must be set (the `make e2e` target mints and exports it).",
    );
  }

  const browser = await chromium.launch();
  try {
    const context = await browser.newContext({ baseURL });

    // maxRedirects: 0 so we can assert on the Location header.
    const register = await context.request.post("/auth/register", {
      form: {
        invite_code: inviteCode,
        username: E2E_USERNAME,
        password: E2E_PASSWORD,
      },
      maxRedirects: 0,
    });
    const registerLocation = register.headers()["location"];
    if (register.status() !== 303 || registerLocation !== "/") {
      throw new Error(
        `registration failed: status=${register.status()} location=${registerLocation}`,
      );
    }

    // Explicit login: independent coverage of POST /auth/login (no spec
    // otherwise exercises it) and a clean session cookie before storageState.
    const login = await context.request.post("/auth/login", {
      form: {
        username: E2E_USERNAME,
        password: E2E_PASSWORD,
      },
      maxRedirects: 0,
    });
    const loginLocation = login.headers()["location"];
    if (login.status() !== 303 || loginLocation !== "/") {
      throw new Error(
        `login failed: status=${login.status()} location=${loginLocation}`,
      );
    }

    const storagePath = path.join(__dirname, STORAGE_STATE);
    await context.storageState({ path: storagePath });

    await context.close();
  } finally {
    await browser.close();
  }
}

export default globalSetup;
