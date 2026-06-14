import { assert } from "@std/assert";
import { launchBrowser, newPage } from "./browser.ts";

Deno.test("fetch adds X-Timezone header", async () => {
  const browser = await launchBrowser();
  try {
    const page = await newPage(browser);
    let timezoneHeader: string | undefined;

    await page.setRequestInterception(true);
    page.on("request", (request) => {
      if (request.url().endsWith("/health")) {
        timezoneHeader = request.headers()["x-timezone"];
      }
      request.continue();
    });

    const browserTimezone = await page.evaluate(async () => {
      await fetch("/health", { method: "GET", cache: "no-cache" });
      return Intl.DateTimeFormat().resolvedOptions().timeZone;
    });

    assert(timezoneHeader && timezoneHeader === browserTimezone);
  } finally {
    await browser.close();
  }
});
