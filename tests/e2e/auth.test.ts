import { assertEquals } from "@std/assert";
import { clearBrowserState, launchBrowser, newPage } from "./browser.ts";

async function gotoEditor(page: import("puppeteer-core").Page): Promise<void> {
  await page.goto("http://localhost:3000/editor");
  await page.waitForSelector("#auth-modal", { visible: true });
}

Deno.test({
  name: "Editor auth modal renders",
  sanitizeResources: false,
  sanitizeOps: false,
  async fn() {
    const browser = await launchBrowser();
    try {
      const page = await newPage(browser);
      await clearBrowserState(page);
      await gotoEditor(page);

      const heading = await page.$eval("#auth-modal h2", (el) =>
        el.textContent ?? ""
      );
      const hasPasswordInput = await page.$("#password") !== null;
      const hasSubmit = await page.$(".auth-submit") !== null;

      assertEquals(heading.includes("Guild Entrance"), true);
      assertEquals(hasPasswordInput, true);
      assertEquals(hasSubmit, true);
    } finally {
      await browser.close();
    }
  },
});

Deno.test({
  name: "Editor auth modal has no page-load JavaScript errors",
  sanitizeResources: false,
  sanitizeOps: false,
  async fn() {
    const browser = await launchBrowser();
    try {
      const page = await newPage(browser);
      await clearBrowserState(page);
      const errors: string[] = [];

      page.on("console", (msg) => {
        if (msg.type() === "error") {
          errors.push(msg.text());
        }
      });

      await gotoEditor(page);

      const actualErrors = errors.filter((error) =>
        !error.includes("favicon") &&
        !error.includes("404") &&
        !error.includes("view-transition")
      );

      assertEquals(
        actualErrors.length,
        0,
        `Expected no JS errors, got: ${actualErrors.join(", ")}`,
      );
    } finally {
      await browser.close();
    }
  },
});
