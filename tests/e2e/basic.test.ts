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

Deno.test("Quest Log keeps its dark theme and hides the unfocused skip link", async () => {
  const browser = await launchBrowser();
  try {
    const page = await newPage(browser);
    const state = await page.$eval("html", (html) => {
      const skipLink = document.querySelector(".skip-link");
      return {
        colorScheme: getComputedStyle(html).colorScheme,
        theme: html.getAttribute("data-theme"),
        skipLinkWidth: skipLink ? getComputedStyle(skipLink).width : null,
      };
    });
    assertEquals(state, {
      colorScheme: "dark",
      theme: null,
      skipLinkWidth: "1px",
    });
  } finally {
    if (browser) await browser.close();
  }
});
