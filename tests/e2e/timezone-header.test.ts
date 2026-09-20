import { assert, assertEquals } from "@std/assert";
import { launchBrowser, newPage } from "./browser.ts";

const BASE_URL = "http://localhost:3000";
const WAIT = { timeout: 5_000 };

function isRootDocumentRequest(
  page: import("puppeteer-core").Page,
  request: import("puppeteer-core").HTTPRequest,
): boolean {
  return request.isNavigationRequest() &&
    request.frame() === page.mainFrame() &&
    request.url() === `${BASE_URL}/`;
}

Deno.test("initial visit stores raw timezone and reloads exactly once", async () => {
  const browser = await launchBrowser();
  try {
    const page = await browser.newPage();
    let documentRequests = 0;
    page.on("request", (request) => {
      if (isRootDocumentRequest(page, request)) documentRequests += 1;
    });

    const firstEvents = page.waitForRequest(
      (request) => request.url().endsWith("/events"),
      WAIT,
    );
    await page.goto(BASE_URL);
    await page.waitForFunction(
      () => {
        const timezone = Intl.DateTimeFormat().resolvedOptions().timeZone;
        return document.cookie.split(";").some((cookie) =>
          cookie.trim() === `QuestLog-TZ=${timezone}`
        );
      },
      WAIT,
    );
    await firstEvents;
    assertEquals(documentRequests, 2);

    const cookie = await page.evaluate(() =>
      document.cookie.split(";").map((value) => value.trim()).find((value) =>
        value.startsWith("QuestLog-TZ=")
      )
    );
    const browserTimezone = await page.evaluate(() =>
      Intl.DateTimeFormat().resolvedOptions().timeZone
    );
    assertEquals(cookie, `QuestLog-TZ=${browserTimezone}`);
    await page.close();

    const matchingPage = await browser.newPage();
    let matchingDocumentRequests = 0;
    matchingPage.on("request", (request) => {
      if (isRootDocumentRequest(matchingPage, request)) {
        matchingDocumentRequests += 1;
      }
    });
    const matchingEvents = matchingPage.waitForRequest(
      (request) => request.url().endsWith("/events"),
      WAIT,
    );
    await matchingPage.goto(BASE_URL);
    await matchingEvents;
    assertEquals(matchingDocumentRequests, 1);
  } finally {
    await browser.close();
  }
});

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
