import { assertEquals } from "@std/assert";
import { launchBrowser, newPage } from "./browser.ts";

Deno.test("Basic Smoke Test", async () => {
  const browser = await launchBrowser();
  try {
    const page = await newPage(browser);
    const heading = await page.$eval("h1", (el) => el.textContent ?? "");
    assertEquals(heading.includes("Quest Log"), true);
  } finally {
    if (browser) await browser.close();
  }
});
