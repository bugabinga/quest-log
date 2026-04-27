import { assertEquals } from "@std/assert";
import { launchBrowser, newPage } from "./browser.ts";

Deno.test("Basic Smoke Test", async () => {
  const browser = await launchBrowser();
  try {
    const page = await newPage(browser);
    // Basic assertion
    const title = await page.title();
    assertEquals(title.includes("Quest"), true);
  } finally {
    if (browser) await browser.close();
  }
});
