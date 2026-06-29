import { assertEquals } from "@std/assert";
import { launchBrowser, newPage, typeIntoInput } from "./browser.ts";

async function login(page: import("puppeteer-core").Page): Promise<void> {
  await page.goto("http://localhost:3000/editor");
  await page.waitForSelector("#auth-modal", { visible: true });
  await typeIntoInput(page, "#password", "dev");
  await page.click(".auth-submit");
  await page.waitForSelector(".editor-container", { visible: true });
}

async function clickTab(
  page: import("puppeteer-core").Page,
  label: string,
): Promise<void> {
  await page.$$eval(".editor-tab", (tabs, label) => {
    const tab = tabs.find((element) => element.textContent?.includes(label));
    if (!(tab instanceof HTMLButtonElement)) {
      throw new Error(`Missing editor tab: ${label}`);
    }
    tab.click();
  }, label);
}

async function panelVisible(
  page: import("puppeteer-core").Page,
  selector: string,
): Promise<boolean> {
  return await page.$eval(selector, (element) => {
    const style = getComputedStyle(element);
    return style.display !== "none" && style.visibility !== "hidden";
  });
}

Deno.test({
  name: "Editor tabs switch without server errors",
  sanitizeResources: false,
  sanitizeOps: false,
  async fn() {
    const browser = await launchBrowser();
    try {
      const page = await newPage(browser);
      const failedUrls: string[] = [];
      page.on("response", (response) => {
        if (response.url().includes("/editor/tab/") && response.status() >= 400) {
          failedUrls.push(`${response.status()} ${response.url()}`);
        }
      });

      await login(page);
      await clickTab(page, "Rewards");
      await page.waitForFunction(() => {
        const panel = document.querySelector("#rewards-panel")?.parentElement;
        return panel && getComputedStyle(panel).display !== "none";
      });

      assertEquals(await panelVisible(page, "#rewards-panel"), true);
      assertEquals(failedUrls, []);

      await clickTab(page, "Settings");
      await page.waitForFunction(() => {
        const panel = document.querySelector("#settings-panel")?.parentElement;
        return panel && getComputedStyle(panel).display !== "none";
      });

      assertEquals(await panelVisible(page, "#settings-panel"), true);
      assertEquals(failedUrls, []);
    } finally {
      await browser.close();
    }
  },
});
