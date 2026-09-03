import { expect, test, type Locator, type Page } from "@playwright/test";

/**
 * Both demo pages (`demo` and `demo-split`) hold the same panel and counter
 * artifacts; they differ only in where the counter runs:
 *
 *  - `demo` — both on the page, wired over `local:` planes.
 *  - `demo-split` — counter on the backend, wired over `ephemeral:` channels.
 *
 * Selectors are the component's own `data-demo-*` marker attributes, never
 * styling hooks. TODO(e2e-tag-scheme-tie)
 */

// Cold wasm load + WS connect + mount can outlast the 5s assertion default on a
// fresh navigation, and on `demo-split` the first total additionally makes a
// round trip through the backend counter.
const CHAIN_TIMEOUT = 20_000;

function total(page: Page): Locator {
  return page.locator("[data-demo-total]");
}

function press(page: Page): Locator {
  return page.locator("[data-demo-press]");
}

// The pre-click assertion on `0` proves the component mounted, not merely
// that the element exists. Each page's counter starts from nothing — `local:`
// planes are page-scoped and the `ephemeral:` channels live in a server this
// run started fresh — so the count after each press is the number of presses so
// far.
//
// Each press is awaited to its own number before the next one. A burst that the
// counter coalesced into one activation would still end at the right total
// while never once reading a retained total back — which is the whole claim
// this demo exists to make — so the intermediate values are what is asserted,
// not just the last.
async function pressAndCount(
  page: Page,
  slug: string,
  presses: number,
): Promise<void> {
  await page.goto(`/surface/${slug}`, { waitUntil: "load" });
  await expect(total(page)).toHaveText("0", { timeout: CHAIN_TIMEOUT });
  for (let i = 1; i <= presses; i += 1) {
    await press(page).click();
    await expect(total(page)).toHaveText(String(i), {
      timeout: CHAIN_TIMEOUT,
    });
  }
}

test("the page-hosted counter totals clicks over local planes", async ({
  page,
}) => {
  await pressAndCount(page, "demo", 3);
});

test("the backend-hosted counter totals clicks over ephemeral channels", async ({
  page,
}) => {
  await pressAndCount(page, "demo-split", 3);
});
